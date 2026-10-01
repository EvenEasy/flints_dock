use async_trait::async_trait;
use base64::{Engine, engine::general_purpose::STANDARD};
use clap::Parser;
use dock_flints::{
    cli::Cli,
    jupiter::{
        WRAPPED_SOL,
        swap::{ApiAccount, ApiInstruction, BuildResponse, api_error},
    },
    rpc::swap::signed_transaction,
    swap::*,
};
use serde_json::{Value, json};
use solana_client::{
    nonblocking::rpc_client::RpcClient,
    rpc_request::RpcRequest,
    rpc_sender::{RpcSender, RpcTransportStats},
};
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use solana_transaction::versioned::VersionedTransaction;
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

const MINT: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
fn request(signer: &Keypair) -> SwapRequest {
    SwapRequest {
        mint: MINT.parse().unwrap(),
        raw_amount: 1_000_000,
        wallet: signer.pubkey(),
        slippage_bps: 50,
    }
}
fn build() -> BuildResponse {
    serde_json::from_str(include_str!("fixtures/jupiter_build.json")).unwrap()
}
fn prepared(request: &SwapRequest, build: BuildResponse) -> PreparedSwap {
    PreparedSwap {
        request: request.clone(),
        quote: build.quote(request).unwrap(),
        build,
        requested_at: Instant::now(),
    }
}
#[test]
fn quote_validates_identity_amount_slippage_and_exact_sol_units() {
    let signer = Keypair::new();
    let req = request(&signer);
    let quote = build().quote(&req).unwrap();
    assert_eq!(quote.expected_out_sol, "0.01");
    assert_eq!(quote.min_out_sol, "0.00995");
    assert_eq!(quote.price_impact_pct, 0.1); // API value is a ratio, not percent.
    assert!(quote.route_exists);
    for change in 0..8 {
        let mut body = build();
        match change {
            0 => body.input_mint = WRAPPED_SOL.into(),
            1 => body.output_mint = MINT.into(),
            2 => body.in_amount = "999999".into(),
            3 => body.slippage_bps = 100,
            4 => body.swap_mode = "ExactOut".into(),
            5 => body.price_impact_pct = "NaN".into(),
            6 => body.other_amount_threshold = "9900000".into(),
            _ => body.other_amount_threshold = "10000001".into(),
        }
        assert!(matches!(
            body.quote(&req),
            Err(SwapError::InvalidResponse(_))
        ));
    }
    let mut empty = build();
    empty.route_plan.clear();
    assert!(matches!(empty.quote(&req), Err(SwapError::NoRoute(_))));
    let mut illiquid = build();
    illiquid.out_amount = "0".into();
    assert!(matches!(
        illiquid.quote(&req),
        Err(SwapError::InsufficientLiquidity(_))
    ));
}
#[test]
fn normal_api_errors_remain_distinct() {
    assert!(matches!(
        api_error(400, &json!({"error":"No routes found"})),
        SwapError::NoRoute(_)
    ));
    assert!(matches!(
        api_error(400, &json!({"errorCode":"TOKEN_NOT_TRADABLE"})),
        SwapError::NoRoute(_)
    ));
    assert!(matches!(
        api_error(400, &json!({"error":"Insufficient liquidity"})),
        SwapError::InsufficientLiquidity(_)
    ));
    assert!(matches!(
        api_error(400, &json!({"error":"Quote expired"})),
        SwapError::Expired
    ));
    assert!(matches!(
        api_error(429, &json!({"error":"rate limited"})),
        SwapError::Api(_)
    ));
    assert!(matches!(
        api_error(401, &json!({"error":"invalid API key"})),
        SwapError::Api(_)
    ));
}

struct Provider {
    builds: Mutex<VecDeque<BuildResponse>>,
    calls: Mutex<Vec<SwapRequest>>,
    stale: bool,
}
impl Provider {
    fn new(builds: Vec<BuildResponse>) -> Self {
        Self {
            builds: Mutex::new(builds.into()),
            calls: Mutex::new(vec![]),
            stale: false,
        }
    }
}
impl SwapProvider for Provider {
    async fn build_swap(&self, req: &SwapRequest) -> Result<PreparedSwap> {
        self.calls.lock().unwrap().push(req.clone());
        let mut prepared = prepared(
            req,
            self.builds
                .lock()
                .unwrap()
                .pop_front()
                .expect("unexpected quote request"),
        );
        if self.stale {
            prepared.requested_at = Instant::now() - Duration::from_secs(60);
        }
        Ok(prepared)
    }
}
#[derive(Default)]
struct Executor {
    submitted: Mutex<Vec<String>>,
    checks: Mutex<usize>,
    fail_check: bool,
}
impl SwapExecutor for Executor {
    async fn check_input(&self, _: &SwapRequest) -> Result<()> {
        *self.checks.lock().unwrap() += 1;
        if self.fail_check {
            Err(SwapError::InsufficientFunds("fixture".into()))
        } else {
            Ok(())
        }
    }
    async fn submit(&self, fresh: PreparedSwap, _: &Keypair, _: &SwapLimits) -> Result<String> {
        self.submitted.lock().unwrap().push(fresh.build.out_amount);
        Ok("fixture-confirmed-signature".into())
    }
}
#[tokio::test]
async fn execution_requotes_and_sends_only_the_fresh_route() {
    let signer = Keypair::new();
    let req = request(&signer);
    let mut fresh = build();
    fresh.out_amount = "20000000".into();
    fresh.other_amount_threshold = "19900000".into();
    let provider = Provider::new(vec![build(), fresh]);
    let executor = Executor::default();
    let limits = SwapLimits::default();
    let preview = preview(&provider, &req, &limits).await.unwrap();
    let receipt = execute(&provider, &executor, &preview, &signer, &limits)
        .await
        .unwrap();
    assert_eq!(receipt.status, "confirmed");
    assert_eq!(receipt.output_asset, "native SOL");
    assert_eq!(receipt.fresh_quote.expected_out_sol, "0.02");
    assert_eq!(*provider.calls.lock().unwrap(), vec![req.clone(), req]);
    assert_eq!(*executor.submitted.lock().unwrap(), vec!["20000000"]);
    assert_eq!(*executor.checks.lock().unwrap(), 1);
}
#[tokio::test]
async fn fresh_price_impact_and_worse_minimum_stop_before_submission() {
    for impact in [false, true] {
        let signer = Keypair::new();
        let mut fresh = build();
        if impact {
            fresh.price_impact_pct = "0.02".into();
        } else {
            fresh.out_amount = "9000000".into();
            fresh.other_amount_threshold = "8955000".into();
        }
        let provider = Provider::new(vec![build(), fresh]);
        let executor = Executor::default();
        let limits = SwapLimits::default();
        let preview = preview(&provider, &request(&signer), &limits)
            .await
            .unwrap();
        let error = execute(&provider, &executor, &preview, &signer, &limits)
            .await
            .unwrap_err();
        assert!(if impact {
            matches!(error, SwapError::PriceImpact { .. })
        } else {
            matches!(error, SwapError::PreviewChanged)
        });
        assert!(executor.submitted.lock().unwrap().is_empty());
    }
}
#[tokio::test]
async fn stale_quotes_wrong_signer_and_insufficient_holdings_do_not_send() {
    let signer = Keypair::new();
    let limits = SwapLimits::default();
    let mut provider = Provider::new(vec![build()]);
    provider.stale = true;
    assert!(matches!(
        preview(&provider, &request(&signer), &limits).await,
        Err(SwapError::Expired)
    ));
    let provider = Provider::new(vec![build()]);
    let preview = preview(&provider, &request(&signer), &limits)
        .await
        .unwrap();
    let executor = Executor {
        fail_check: true,
        ..Default::default()
    };
    assert!(matches!(
        execute(&provider, &executor, &preview, &Keypair::new(), &limits).await,
        Err(SwapError::InvalidRequest(_))
    ));
    assert!(matches!(
        execute(&provider, &executor, &preview, &signer, &limits).await,
        Err(SwapError::InsufficientFunds(_))
    ));
    assert_eq!(provider.calls.lock().unwrap().len(), 1);
    assert!(executor.submitted.lock().unwrap().is_empty());
}

#[derive(Clone, Default)]
struct RpcFixture {
    calls: Arc<Mutex<Vec<(RpcRequest, Value)>>>,
    simulation_error: bool,
    send_error: bool,
    onchain_error: bool,
    expired: bool,
    pending: bool,
    processed_error: bool,
}
#[async_trait]
impl RpcSender for RpcFixture {
    async fn send(
        &self,
        method: RpcRequest,
        params: Value,
    ) -> solana_client::client_error::Result<Value> {
        self.calls.lock().unwrap().push((method, params.clone()));
        match method {
            RpcRequest::GetBlockHeight => Ok(json!(if self.expired { 1001 } else { 100 })),
            RpcRequest::SimulateTransaction => {
                assert_eq!(params[1]["sigVerify"], true);
                assert_eq!(params[1]["replaceRecentBlockhash"], false);
                let tx: VersionedTransaction =
                    bincode::deserialize(&STANDARD.decode(params[0].as_str().unwrap()).unwrap())
                        .unwrap();
                tx.sanitize().unwrap();
                assert_ne!(tx.signatures[0].to_string(), "1".repeat(64));
                Ok(
                    json!({"context":{"slot":100}, "value":{"err":if self.simulation_error {json!("InsufficientFundsForFee")} else {Value::Null}, "logs":[], "unitsConsumed":200000}}),
                )
            }
            RpcRequest::SendTransaction => {
                assert_eq!(params[1]["skipPreflight"], false);
                assert_eq!(params[1]["preflightCommitment"], "confirmed");
                if self.send_error {
                    return Err(std::io::Error::other("fixture send timeout").into());
                }
                let tx: VersionedTransaction =
                    bincode::deserialize(&STANDARD.decode(params[0].as_str().unwrap()).unwrap())
                        .unwrap();
                let instructions = tx.message.instructions();
                assert_eq!(
                    u32::from_le_bytes(instructions[0].data[1..].try_into().unwrap()),
                    240000
                );
                Ok(json!(tx.signatures[0].to_string()))
            }
            RpcRequest::GetSignatureStatuses => {
                assert_eq!(params[1]["searchTransactionHistory"], true);
                let error = if self.onchain_error || self.processed_error {
                    json!({"InstructionError":[1,{"Custom":6001}]})
                } else {
                    Value::Null
                };
                let status = if self.pending {
                    Value::Null
                } else {
                    json!({"slot":101,"confirmations":1,"err":error,"status":if self.onchain_error || self.processed_error {json!({"Err":error})} else {json!({"Ok":null})},"confirmationStatus":if self.processed_error {"processed"} else {"confirmed"}})
                };
                Ok(json!({"context":{"slot":101},"value":[status]}))
            }
            _ => panic!("Unexpected RPC method {method}"),
        }
    }
    fn get_transport_stats(&self) -> RpcTransportStats {
        Default::default()
    }
    fn url(&self) -> String {
        "http://fixture.invalid".into()
    }
}
#[tokio::test]
async fn rpc_simulates_signs_sends_once_and_confirms_or_preserves_uncertain_signature() {
    for scenario in 0..7 {
        let fixture = RpcFixture {
            simulation_error: scenario == 1,
            send_error: scenario == 2,
            onchain_error: scenario == 3,
            expired: scenario == 4,
            pending: scenario == 5,
            processed_error: scenario == 6,
            ..Default::default()
        };
        let calls = fixture.calls.clone();
        let rpc = RpcClient::new_sender(fixture, Default::default());
        let signer = Keypair::new();
        let request = request(&signer);
        let result = rpc
            .submit(
                prepared(&request, build()),
                &signer,
                &SwapLimits {
                    confirmation_timeout: Duration::from_millis(5),
                    ..Default::default()
                },
            )
            .await;
        match scenario {
            0 => assert!(result.is_ok(), "{result:?}"),
            1 => assert!(matches!(result, Err(SwapError::Simulation(_)))),
            2 | 5 | 6 => assert!(
                matches!(result, Err(SwapError::Uncertain { signature, .. }) if !signature.is_empty())
            ),
            3 => assert!(matches!(result, Err(SwapError::TransactionFailed { .. }))),
            4 => assert!(matches!(result, Err(SwapError::Expired))),
            _ => unreachable!(),
        }
        let sends = calls
            .lock()
            .unwrap()
            .iter()
            .filter(|(m, _)| *m == RpcRequest::SendTransaction)
            .count();
        assert_eq!(sends, if [1, 4].contains(&scenario) { 0 } else { 1 });
    }
}
#[test]
fn transaction_keeps_cleanup_and_lookup_tables_and_rejects_extra_signers_or_fees() {
    let signer = Keypair::new();
    let req = request(&signer);
    let mut body = build();
    let account = Pubkey::new_from_array([44; 32]).to_string();
    let table = Pubkey::new_from_array([45; 32]).to_string();
    body.addresses_by_lookup_table_address =
        Some([(table, vec![account.clone()])].into_iter().collect());
    body.cleanup_instruction = Some(ApiInstruction {
        program_id: spl_token_interface::ID.to_string(),
        data: STANDARD.encode([9]),
        accounts: vec![
            ApiAccount {
                pubkey: account,
                is_signer: false,
                is_writable: true,
            },
            ApiAccount {
                pubkey: signer.pubkey().to_string(),
                is_signer: false,
                is_writable: true,
            },
            ApiAccount {
                pubkey: signer.pubkey().to_string(),
                is_signer: true,
                is_writable: false,
            },
        ],
    });
    let plan = prepared(&req, body.clone());
    let tx = signed_transaction(&plan, &signer, 200000, &Default::default()).unwrap();
    tx.sanitize().unwrap();
    assert_eq!(tx.message.address_table_lookups().unwrap().len(), 1);
    assert_eq!(tx.message.instructions().last().unwrap().data, [9]);
    body.swap_instruction.accounts.push(ApiAccount {
        pubkey: Pubkey::new_from_array([99; 32]).to_string(),
        is_signer: true,
        is_writable: false,
    });
    assert!(
        signed_transaction(&prepared(&req, body), &signer, 200000, &Default::default()).is_err()
    );
    let mut body = build();
    let mut price = vec![3];
    price.extend_from_slice(&u64::MAX.to_le_bytes());
    body.compute_budget_instructions.push(ApiInstruction {
        program_id: "ComputeBudget111111111111111111111111111111".into(),
        accounts: vec![],
        data: STANDARD.encode(price),
    });
    assert!(matches!(
        signed_transaction(&prepared(&req, body), &signer, 1400000, &Default::default()),
        Err(SwapError::InvalidRequest(_))
    ));
}
#[test]
fn swap_cli_is_separate_from_scanning_and_requires_exact_mint_amount_and_keypair() {
    let wallet = Keypair::new().pubkey().to_string();
    assert!(
        Cli::try_parse_from([
            "dock_flints",
            "quote",
            "-p",
            &wallet,
            "--mint",
            MINT,
            "--raw-amount",
            "1000000"
        ])
        .is_ok()
    );
    assert!(
        Cli::try_parse_from([
            "dock_flints",
            "swap",
            "--keypair",
            "wallet.json",
            "--mint",
            MINT,
            "--raw-amount",
            "1000000",
            "--yes"
        ])
        .is_ok()
    );
    for args in [
        vec!["dock_flints", "quote", "--mint", MINT, "--raw-amount", "1"],
        vec!["dock_flints", "swap", "--mint", MINT, "--raw-amount", "1"],
        vec![
            "dock_flints",
            "quote",
            "-p",
            &wallet,
            "--mint",
            "USDC",
            "--raw-amount",
            "1",
        ],
        vec![
            "dock_flints",
            "quote",
            "-p",
            &wallet,
            "--mint",
            MINT,
            "--raw-amount",
            "0",
        ],
        vec![
            "dock_flints",
            "--tokens",
            "quote",
            "-p",
            &wallet,
            "--mint",
            MINT,
            "--raw-amount",
            "1",
        ],
    ] {
        assert!(Cli::try_parse_from(args).is_err());
    }
}

#[tokio::test]
async fn stale_fresh_quote_is_not_submitted_after_a_valid_preview() {
    let signer = Keypair::new();
    let limits = SwapLimits::default();
    let mut provider = Provider::new(vec![build(), build()]);
    let preview = preview(&provider, &request(&signer), &limits)
        .await
        .unwrap();
    provider.stale = true;
    let executor = Executor::default();
    assert!(matches!(
        execute(&provider, &executor, &preview, &signer, &limits).await,
        Err(SwapError::Expired)
    ));
    assert_eq!(provider.calls.lock().unwrap().len(), 2);
    assert!(executor.submitted.lock().unwrap().is_empty());
}

#[derive(Clone)]
struct InputRpc {
    wallet: Pubkey,
    program: Pubkey,
    amount: u64,
    nft: bool,
    calls: Arc<Mutex<Vec<RpcRequest>>>,
}
#[async_trait]
impl RpcSender for InputRpc {
    async fn send(
        &self,
        method: RpcRequest,
        params: Value,
    ) -> solana_client::client_error::Result<Value> {
        use borsh::BorshSerialize;
        use mpl_token_metadata::{
            accounts::Metadata,
            types::{Key, TokenStandard},
        };
        self.calls.lock().unwrap().push(method);
        let mint: Pubkey = MINT.parse().unwrap();
        let ui = |data: Vec<u8>, owner: String| json!({"lamports":2039280,"data":[STANDARD.encode(data),"base64"],"owner":owner,"executable":false,"rentEpoch":0});
        match method {
            RpcRequest::GetTokenAccountsByOwner => {
                assert_eq!(params[1], json!({"mint":MINT}));
                let mut data = vec![0; 165];
                data[..32].copy_from_slice(mint.as_ref());
                data[32..64].copy_from_slice(self.wallet.as_ref());
                data[64..72].copy_from_slice(&self.amount.to_le_bytes());
                data[108] = 1;
                Ok(
                    json!({"context":{"slot":100},"value":[{"pubkey":Pubkey::new_from_array([22;32]).to_string(),"account":ui(data,self.program.to_string())}]}),
                )
            }
            RpcRequest::GetMultipleAccounts => {
                let values: Vec<Value> = params[0]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|key| {
                        if key == MINT {
                            let mut data = vec![0; 82];
                            data[36..44].copy_from_slice(&self.amount.to_le_bytes());
                            data[44] = if self.nft { 0 } else { 6 };
                            data[45] = 1;
                            ui(data, self.program.to_string())
                        } else if self.nft {
                            let metadata = Metadata {
                                key: Key::MetadataV1,
                                update_authority: dock_flints::rpc::to_metaplex(self.wallet),
                                mint: dock_flints::rpc::to_metaplex(mint),
                                name: "NFT fixture".into(),
                                symbol: "NFT".into(),
                                uri: "https://example.invalid".into(),
                                seller_fee_basis_points: 0,
                                creators: None,
                                primary_sale_happened: false,
                                is_mutable: true,
                                edition_nonce: None,
                                token_standard: Some(TokenStandard::NonFungible),
                                collection: None,
                                uses: None,
                                collection_details: None,
                                programmable_config: None,
                            };
                            ui(
                                metadata.try_to_vec().unwrap(),
                                mpl_token_metadata::ID.to_string(),
                            )
                        } else {
                            Value::Null
                        }
                    })
                    .collect();
                Ok(json!({"context":{"slot":100},"value":values}))
            }
            _ => panic!("Unexpected input validation RPC {method}"),
        }
    }
    fn get_transport_stats(&self) -> RpcTransportStats {
        Default::default()
    }
    fn url(&self) -> String {
        "http://fixture.invalid".into()
    }
}
#[tokio::test]
async fn execution_input_check_reuses_decoders_rejects_nfts_and_accepts_both_fungible_programs() {
    for scenario in 0..4 {
        let signer = Keypair::new();
        let calls = Arc::new(Mutex::new(vec![]));
        let fixture = InputRpc {
            wallet: signer.pubkey(),
            program: if scenario == 1 {
                spl_token_2022_interface::ID
            } else {
                spl_token_interface::ID
            },
            amount: if scenario == 2 {
                1
            } else if scenario == 3 {
                0
            } else {
                1_000_000
            },
            nft: scenario == 2,
            calls: calls.clone(),
        };
        let rpc = RpcClient::new_sender(fixture, Default::default());
        let mut req = request(&signer);
        if scenario == 2 {
            req.raw_amount = 1;
        }
        let result = rpc.check_input(&req).await;
        if scenario < 2 {
            assert!(result.is_ok(), "{result:?}");
        } else if scenario == 2 {
            assert!(matches!(result, Err(SwapError::InvalidRequest(_))));
        } else {
            assert!(matches!(result, Err(SwapError::InsufficientFunds(_))));
        }
        let calls = calls.lock().unwrap();
        assert_eq!(
            calls
                .iter()
                .filter(|m| **m == RpcRequest::GetTokenAccountsByOwner)
                .count(),
            1
        );
        assert!(!calls.contains(&RpcRequest::SendTransaction));
    }
}
