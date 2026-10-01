use dock_flints::{
    cleanup::{
        plan::{build_plan, unsupported_reason},
        service::execute_plan,
        *,
    },
    jupiter::swap::BuildResponse,
    models::*,
    rpc::cleanup::{
        associated_address, burn_instruction, close_instruction, source_instructions,
        transaction_accounting,
    },
    swap::*,
};
use serde_json::{Value, json};
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use std::{
    collections::{BTreeMap, VecDeque},
    sync::Mutex,
    time::{Duration, Instant},
};
fn key(n: u8) -> Pubkey {
    Pubkey::new_from_array([n; 32])
}
fn options() -> CleanupOptions {
    CleanupOptions {
        quote_interval: Duration::ZERO,
        ..Default::default()
    }
}
fn asset(owner: Pubkey, n: u8, amount: u64) -> CleanupAsset {
    CleanupAsset {
        account: TokenAccount {
            address: key(n).to_string(),
            mint: key(200).to_string(),
            program: TokenProgram::Legacy,
            program_id: TokenProgram::Legacy.id().to_string(),
            owner: owner.to_string(),
            raw_amount: amount,
            decimals: Some(6),
            lamports: 2_039_280,
            data_len: 165,
            state: "Initialized".into(),
            delegate: None,
            delegated_amount: 0,
            close_authority: None,
            is_native: false,
            native_reserve_lamports: None,
            extensions: json!([]),
            extension_types: vec![],
            closure: ClosureAssessment::PotentiallyReclaimable,
        },
        mint: Some(MintInfo {
            mint: key(200).to_string(),
            program: TokenProgram::Legacy,
            decimals: 6,
            supply: 1_000_000,
            mint_authority: None,
            freeze_authority: None,
            metadata: Default::default(),
            extensions: json!([]),
            extension_types: vec![],
        }),
        kind: AssetKind::Fungible,
    }
}
#[derive(Clone)]
enum Answer {
    Route,
    NoRoute,
    Api,
    Illiquid,
    Expired,
}
struct Provider {
    answers: Mutex<VecDeque<Answer>>,
    requests: Mutex<Vec<SwapRequest>>,
}
impl Provider {
    fn new(answers: Vec<Answer>) -> Self {
        Self {
            answers: Mutex::new(answers.into()),
            requests: Mutex::new(vec![]),
        }
    }
}
impl SwapProvider for Provider {
    async fn build_swap(&self, request: &SwapRequest) -> Result<PreparedSwap> {
        self.requests.lock().unwrap().push(request.clone());
        match self
            .answers
            .lock()
            .unwrap()
            .pop_front()
            .expect("unexpected Jupiter request")
        {
            Answer::NoRoute => Err(SwapError::NoRoute("fixture no route".into())),
            Answer::Api => Err(SwapError::Api("fixture 429".into())),
            Answer::Illiquid => Err(SwapError::InsufficientLiquidity("fixture".into())),
            Answer::Expired => Err(SwapError::Expired),
            Answer::Route => {
                let mut build: BuildResponse =
                    serde_json::from_str(include_str!("fixtures/jupiter_build.json")).unwrap();
                build.input_mint = request.mint.to_string();
                build.in_amount = request.raw_amount.to_string();
                build.slippage_bps = request.slippage_bps;
                Ok(PreparedSwap {
                    request: request.clone(),
                    quote: build.quote(request).unwrap(),
                    build,
                    requested_at: Instant::now(),
                })
            }
        }
    }
}
#[derive(Default)]
struct Executor {
    assets: Mutex<BTreeMap<String, CleanupAsset>>,
    events: Mutex<Vec<(String, CleanupOperation)>>,
    fail_address: Option<String>,
    fail_operation: Option<CleanupOperation>,
    uncertain: bool,
    leave_balance: bool,
}
impl Executor {
    fn new(assets: &[CleanupAsset]) -> Self {
        Self {
            assets: Mutex::new(
                assets
                    .iter()
                    .map(|a| (a.account.address.clone(), a.clone()))
                    .collect(),
            ),
            ..Default::default()
        }
    }
}
impl CleanupExecutor for Executor {
    async fn refresh(&self, address: &str, _: &Pubkey) -> Result<Option<CleanupAsset>> {
        Ok(self.assets.lock().unwrap().get(address).cloned())
    }
    async fn perform(
        &self,
        operation: CleanupOperation,
        asset: &CleanupAsset,
        fresh: Option<PreparedSwap>,
        _: &Keypair,
        _: &SwapLimits,
    ) -> Result<OperationReceipt> {
        self.events
            .lock()
            .unwrap()
            .push((asset.account.address.clone(), operation));
        if self.fail_address.as_deref() == Some(&asset.account.address)
            && self.fail_operation == Some(operation)
        {
            return if self.uncertain {
                Err(SwapError::Uncertain {
                    signature: "pending-signature".into(),
                    reason: "timeout".into(),
                })
            } else {
                Err(SwapError::Rpc("fixture failure".into()))
            };
        }
        let mut assets = self.assets.lock().unwrap();
        match operation {
            CleanupOperation::Swap => {
                let fresh = fresh.expect("fresh swap");
                assert_eq!(fresh.request.raw_amount, asset.account.raw_amount);
                assert_eq!(fresh.request.mint.to_string(), asset.account.mint);
                assets
                    .get_mut(&asset.account.address)
                    .unwrap()
                    .account
                    .raw_amount = if self.leave_balance { 1 } else { 0 };
            }
            CleanupOperation::Burn => {
                assert!(fresh.is_none());
                assets
                    .get_mut(&asset.account.address)
                    .unwrap()
                    .account
                    .raw_amount = if self.leave_balance { 1 } else { 0 };
            }
            CleanupOperation::Close => {
                assert_eq!(asset.account.raw_amount, 0);
                assets.remove(&asset.account.address);
            }
        }
        Ok(OperationReceipt {
            operation,
            signature: format!("{operation:?}-{}", asset.account.address),
            wallet_delta_lamports: Some(if operation == CleanupOperation::Swap {
                9_995_000
            } else {
                -5000
            }),
            reclaimed_lamports: (operation == CleanupOperation::Close)
                .then_some(asset.account.lamports),
        })
    }
}
async fn plan(signer: &Keypair, assets: Vec<CleanupAsset>, provider: &Provider) -> CleanupPlan {
    build_plan(
        &signer.pubkey(),
        assets,
        ScanStatus::Complete,
        vec![],
        provider,
        &options(),
    )
    .await
}

#[tokio::test]
async fn preview_categories_use_real_route_checks_not_price_or_error_fallbacks() {
    let signer = Keypair::new();
    let owner = signer.pubkey();
    let mut frozen = asset(owner, 7, 4);
    frozen.account.state = "Frozen".into();
    let mut nft = asset(owner, 8, 1);
    nft.kind = AssetKind::NonFungible;
    let provider = Provider::new(vec![
        Answer::Route,
        Answer::NoRoute,
        Answer::Api,
        Answer::Api,
        Answer::Illiquid,
    ]);
    let plan = plan(
        &signer,
        vec![
            asset(owner, 1, 0),
            asset(owner, 2, 10),
            asset(owner, 3, 20),
            asset(owner, 4, 30),
            asset(owner, 5, 40),
            frozen,
            nft,
        ],
        &provider,
    )
    .await;
    assert_eq!(
        plan.entries.iter().map(|e| e.category).collect::<Vec<_>>(),
        vec![
            CleanupCategory::Empty,
            CleanupCategory::Swappable,
            CleanupCategory::Burnable,
            CleanupCategory::Unsupported,
            CleanupCategory::Unsupported,
            CleanupCategory::Unsupported,
            CleanupCategory::Unsupported
        ]
    );
    assert_eq!(plan.summary.accounts_to_close, 3);
    assert_eq!(plan.summary.estimated_swap_lamports, 10_000_000);
    assert_eq!(plan.summary.estimated_reclaimed_lamports, 3 * 2_039_280);
    assert_eq!(
        provider
            .requests
            .lock()
            .unwrap()
            .iter()
            .map(|r| r.raw_amount)
            .collect::<Vec<_>>(),
        [10, 20, 30, 30, 40]
    );
    let mut output = vec![];
    dock_flints::output::cleanup::write_plan(&mut output, &plan).unwrap();
    let output = String::from_utf8(output).unwrap();
    for action in [
        "SWAP -> CLOSE",
        "BURN -> CLOSE",
        "SKIP",
        "No transactions submitted.",
    ] {
        assert!(output.contains(action));
    }
}
#[test]
fn authority_nft_native_and_extension_guards_are_conservative() {
    let owner = key(9);
    for scenario in 0..8 {
        let mut asset = asset(owner, 1, 10);
        match scenario {
            0 => asset.account.close_authority = Some(key(22).to_string()),
            1 => asset.account.owner = key(22).to_string(),
            2 => asset.account.extension_types = vec!["TransferFeeAmount".into()],
            3 => asset.mint.as_mut().unwrap().extension_types = vec!["TransferHook".into()],
            4 => asset.kind = AssetKind::Unknown,
            5 => asset.account.is_native = true,
            6 => asset.account.decimals = None,
            _ => asset.account.state = "Frozen".into(),
        }
        assert!(unsupported_reason(&asset, &owner.to_string()).is_some());
        assert!(burn_instruction(&asset, &owner).is_err());
    }
    let mut empty = asset(owner, 1, 0);
    empty.account.state = "Frozen".into();
    assert!(close_instruction(&empty, &owner).is_ok()); // SPL permits frozen empty closure.
}
#[tokio::test]
async fn execution_is_sequential_requotes_burns_and_verifies_zero_before_close() {
    let signer = Keypair::new();
    let assets = vec![
        asset(signer.pubkey(), 1, 0),
        asset(signer.pubkey(), 2, 5),
        asset(signer.pubkey(), 3, 7),
    ];
    let executor = Executor::new(&assets);
    let provider = Provider::new(vec![
        Answer::Route,
        Answer::NoRoute,
        Answer::Route,
        Answer::NoRoute,
    ]);
    let plan = plan(&signer, assets.clone(), &provider).await;
    assert!(executor.events.lock().unwrap().is_empty());
    let report = execute_plan(&plan, &provider, &executor, &signer, &options())
        .await
        .unwrap();
    assert_eq!(report.closed, 3);
    assert_eq!(report.failed, 0);
    assert_eq!(report.known_swap_net_lamports, 9_995_000);
    assert_eq!(report.known_reclaimed_lamports, 3 * 2_039_280);
    assert!(report.accounting_complete);
    assert_eq!(
        executor
            .events
            .lock()
            .unwrap()
            .iter()
            .map(|(_, op)| *op)
            .collect::<Vec<_>>(),
        [
            CleanupOperation::Close,
            CleanupOperation::Swap,
            CleanupOperation::Close,
            CleanupOperation::Burn,
            CleanupOperation::Close
        ]
    );
    assert_eq!(
        provider
            .requests
            .lock()
            .unwrap()
            .iter()
            .map(|r| r.raw_amount)
            .collect::<Vec<_>>(),
        [5, 7, 5, 7]
    );
    assert!(executor.assets.lock().unwrap().is_empty());
}
#[tokio::test]
async fn new_route_or_api_failure_prevents_a_previously_planned_burn() {
    for answer in [Answer::Route, Answer::Api] {
        let signer = Keypair::new();
        let assets = vec![asset(signer.pubkey(), 1, 10)];
        let executor = Executor::new(&assets);
        let provider = Provider::new(vec![Answer::NoRoute, answer.clone(), answer]);
        let plan = plan(&signer, assets, &provider).await;
        let report = execute_plan(&plan, &provider, &executor, &signer, &options())
            .await
            .unwrap();
        assert_eq!(report.failed, 1);
        assert!(executor.events.lock().unwrap().is_empty());
    }
}
#[tokio::test]
async fn one_failed_close_preserves_burn_receipt_and_does_not_stop_other_accounts() {
    let signer = Keypair::new();
    let assets = vec![asset(signer.pubkey(), 1, 10), asset(signer.pubkey(), 2, 0)];
    let mut executor = Executor::new(&assets);
    executor.fail_address = Some(assets[0].account.address.clone());
    executor.fail_operation = Some(CleanupOperation::Close);
    let provider = Provider::new(vec![Answer::NoRoute, Answer::NoRoute]);
    let plan = plan(&signer, assets, &provider).await;
    let report = execute_plan(&plan, &provider, &executor, &signer, &options())
        .await
        .unwrap();
    assert_eq!(report.failed, 1);
    assert_eq!(report.closed, 1);
    assert_eq!(report.results[0].operations.len(), 1);
    assert_eq!(
        report.results[0].operations[0].operation,
        CleanupOperation::Burn
    );
    assert_eq!(report.known_reclaimed_lamports, 2_039_280);
}
#[tokio::test]
async fn uncertain_swap_is_not_retried_or_burned_and_related_mint_is_skipped() {
    let signer = Keypair::new();
    let mut assets = vec![
        asset(signer.pubkey(), 1, 10),
        asset(signer.pubkey(), 2, 0),
        asset(signer.pubkey(), 3, 0),
    ];
    assets[2].account.mint = key(201).to_string();
    assets[2].mint.as_mut().unwrap().mint = key(201).to_string();
    let mut executor = Executor::new(&assets);
    executor.fail_address = Some(assets[0].account.address.clone());
    executor.fail_operation = Some(CleanupOperation::Swap);
    executor.uncertain = true;
    let provider = Provider::new(vec![Answer::Route, Answer::Route]);
    let plan = plan(&signer, assets, &provider).await;
    let report = execute_plan(&plan, &provider, &executor, &signer, &options())
        .await
        .unwrap();
    assert_eq!((report.failed, report.skipped, report.closed), (1, 1, 1));
    assert!(!report.accounting_complete);
    assert_eq!(
        report.results[0].uncertain_signature.as_deref(),
        Some("pending-signature")
    );
    assert_eq!(executor.events.lock().unwrap().len(), 2);
}
#[tokio::test]
async fn changed_balance_or_nonzero_post_balance_never_gets_closed() {
    for changed in [true, false] {
        let signer = Keypair::new();
        let assets = vec![asset(signer.pubkey(), 1, 10)];
        let mut executor = Executor::new(&assets);
        executor.leave_balance = !changed;
        let provider = Provider::new(vec![Answer::NoRoute, Answer::NoRoute]);
        let plan = plan(&signer, assets.clone(), &provider).await;
        if changed {
            executor
                .assets
                .lock()
                .unwrap()
                .get_mut(&assets[0].account.address)
                .unwrap()
                .account
                .raw_amount += 1;
        }
        let report = execute_plan(&plan, &provider, &executor, &signer, &options())
            .await
            .unwrap();
        assert_eq!(report.failed, 1);
        assert!(
            executor
                .events
                .lock()
                .unwrap()
                .iter()
                .all(|(_, op)| *op != CleanupOperation::Close)
        );
        assert_eq!(executor.events.lock().unwrap().len(), usize::from(!changed));
    }
}
#[tokio::test]
async fn stale_quote_retries_are_bounded_and_fresh_route_is_used() {
    let signer = Keypair::new();
    let assets = vec![asset(signer.pubkey(), 1, 5)];
    let executor = Executor::new(&assets);
    let provider = Provider::new(vec![
        Answer::Expired,
        Answer::Route,
        Answer::Expired,
        Answer::Route,
    ]);
    let plan = plan(&signer, assets, &provider).await;
    let report = execute_plan(&plan, &provider, &executor, &signer, &options())
        .await
        .unwrap();
    assert_eq!(report.closed, 1);
    assert_eq!(provider.requests.lock().unwrap().len(), 4);
}
#[test]
fn burn_checked_close_and_auxiliary_source_instructions_preserve_program_amount_and_destination() {
    use spl_token_2022_interface::instruction::TokenInstruction;
    let owner = key(9);
    for program in [TokenProgram::Legacy, TokenProgram::Token2022] {
        let mut asset = asset(owner, 1, u64::MAX);
        asset.account.program = program;
        asset.mint.as_mut().unwrap().program = program;
        let burn = burn_instruction(&asset, &owner).unwrap();
        assert_eq!(burn.program_id, program.id());
        assert_eq!(burn.accounts[0].pubkey, key(1));
        assert_eq!(burn.accounts[2].pubkey, owner);
        assert!(matches!(
            TokenInstruction::unpack(&burn.data).unwrap(),
            TokenInstruction::BurnChecked {
                amount: u64::MAX,
                decimals: 6
            }
        ));
        for exists in [false, true] {
            let (prefix, suffix) = source_instructions(&asset, &owner, exists).unwrap();
            assert_eq!(prefix.len(), if exists { 1 } else { 2 });
            assert_eq!(suffix.len(), usize::from(!exists));
            let transfer = prefix.last().unwrap();
            assert_eq!(transfer.accounts[0].pubkey, key(1));
            assert_eq!(
                transfer.accounts[2].pubkey,
                associated_address(&owner, &key(200), program)
            );
            assert!(matches!(
                TokenInstruction::unpack(&transfer.data).unwrap(),
                TokenInstruction::TransferChecked {
                    amount: u64::MAX,
                    decimals: 6
                }
            ));
        }
        assert!(close_instruction(&asset, &owner).is_err());
        asset.account.raw_amount = 0;
        let close = close_instruction(&asset, &owner).unwrap();
        assert_eq!(close.accounts[1].pubkey, owner);
        assert!(matches!(
            TokenInstruction::unpack(&close.data).unwrap(),
            TokenInstruction::CloseAccount
        ));
    }
    assert_ne!(
        associated_address(&owner, &key(200), TokenProgram::Legacy),
        associated_address(&owner, &key(200), TokenProgram::Token2022)
    );
}
#[test]
fn receipt_accounting_uses_confirmed_metadata_including_loaded_addresses_not_quotes() {
    let value = json!({"transaction":{"message":{"accountKeys":["wallet"]}},"meta":{"err":null,"preBalances":[100000,2039280],"postBalances":[2134280,0],"loadedAddresses":{"writable":["source"],"readonly":[]}}});
    assert_eq!(
        transaction_accounting(&value, "wallet", Some("source")),
        (Some(2034280), Some(2039280))
    );
    assert_eq!(
        transaction_accounting(&Value::Null, "wallet", Some("source")),
        (None, None)
    );
}
#[test]
fn cli_defaults_to_preview_and_requires_explicit_execution_for_keypair_and_yes() {
    use clap::Parser;
    use dock_flints::cli::{Cli, Command};
    let wallet = key(9).to_string();
    let cli = Cli::try_parse_from(["dock_flints", "cleanup", "-p", &wallet]).unwrap();
    assert!(matches!(cli.command,Some(Command::Cleanup(args)) if !args.execute));
    for tail in [
        vec!["--execute"],
        vec!["--yes"],
        vec!["--keypair", "key.json"],
        vec!["--execute", "--keypair", "key.json", "--dry-run"],
    ] {
        let mut args = vec!["dock_flints", "cleanup", "-p", &wallet];
        args.extend(tail);
        assert!(Cli::try_parse_from(args).is_err());
    }
    assert!(
        Cli::try_parse_from([
            "dock_flints",
            "cleanup",
            "-p",
            &wallet,
            "--execute",
            "--keypair",
            "key.json",
            "--yes"
        ])
        .is_ok()
    );
}

#[test]
fn endpoint_authentication_and_unsupported_api_errors_cannot_authorize_burn() {
    use dock_flints::jupiter::swap::api_error;
    for (status, body) in [
        (401, json!({"error":"No routes found"})),
        (429, json!({"error":"No routes found"})),
        (404, json!({"error":"No routes found"})),
        (
            400,
            json!({"errorCode":"NOT_SUPPORTED","error":"option not supported"}),
        ),
    ] {
        assert!(!matches!(api_error(status, &body), SwapError::NoRoute(_)));
    }
}
#[test]
fn auxiliary_transfer_and_swap_are_compiled_into_one_atomic_transaction() {
    let signer = Keypair::new();
    let asset = asset(signer.pubkey(), 1, 7);
    let mut build: BuildResponse =
        serde_json::from_str(include_str!("fixtures/jupiter_build.json")).unwrap();
    let request =
        dock_flints::cleanup::plan::request(&asset, &signer.pubkey(), &options()).unwrap();
    build.input_mint = request.mint.to_string();
    build.in_amount = request.raw_amount.to_string();
    let prepared = PreparedSwap {
        quote: build.quote(&request).unwrap(),
        request,
        build,
        requested_at: Instant::now(),
    };
    let (prefix, suffix) = source_instructions(&asset, &signer.pubkey(), false).unwrap();
    let tx = dock_flints::rpc::swap::signed_transaction_with_extras(
        &prepared,
        &signer,
        400000,
        &SwapLimits::default(),
        &prefix,
        &suffix,
    )
    .unwrap();
    tx.sanitize().unwrap();
    let instructions = tx.message.instructions();
    assert_eq!(instructions.len(), 5); // CU limit, create ATA, transfer exact source amount, Jupiter swap, close staging ATA
    assert_eq!(instructions[1].data, [1]);
    assert!(matches!(
        spl_token_2022_interface::instruction::TokenInstruction::unpack(&instructions[2].data)
            .unwrap(),
        spl_token_2022_interface::instruction::TokenInstruction::TransferChecked {
            amount: 7,
            decimals: 6
        }
    ));
    assert_eq!(instructions[4].data, [9]);
}

#[derive(Clone)]
struct TransactionRpc {
    methods: std::sync::Arc<Mutex<Vec<solana_client::rpc_request::RpcRequest>>>,
    amount: std::sync::Arc<Mutex<Option<u64>>>,
    simulation_fails: bool,
}
#[async_trait::async_trait]
impl solana_client::rpc_sender::RpcSender for TransactionRpc {
    async fn send(
        &self,
        method: solana_client::rpc_request::RpcRequest,
        params: Value,
    ) -> solana_client::client_error::Result<Value> {
        use base64::Engine;
        use solana_client::rpc_request::RpcRequest;
        self.methods.lock().unwrap().push(method);
        match method {
            RpcRequest::GetLatestBlockhash => Ok(
                json!({"context":{"slot":1},"value":{"blockhash":key(3).to_string(),"lastValidBlockHeight":100}}),
            ),
            RpcRequest::GetBlockHeight => Ok(json!(1)),
            RpcRequest::SimulateTransaction => {
                assert_eq!(params[1]["sigVerify"], true);
                Ok(
                    json!({"context":{"slot":1},"value":{"err":if self.simulation_fails {json!("InsufficientFundsForFee")} else {Value::Null},"logs":[],"unitsConsumed":2000}}),
                )
            }
            RpcRequest::SendTransaction => {
                assert_eq!(params[1]["skipPreflight"], false);
                let data = base64::engine::general_purpose::STANDARD
                    .decode(params[0].as_str().unwrap())
                    .unwrap();
                let tx: solana_transaction::versioned::VersionedTransaction =
                    bincode::deserialize(&data).unwrap();
                tx.sanitize().unwrap();
                let instructions = tx.message.instructions();
                assert_eq!(instructions.len(), 2); // CU limit plus one burn or close
                let mut amount = self.amount.lock().unwrap();
                match spl_token_2022_interface::instruction::TokenInstruction::unpack(
                    &instructions[1].data,
                )
                .unwrap()
                {
                    spl_token_2022_interface::instruction::TokenInstruction::BurnChecked {
                        amount: burn,
                        decimals,
                    } => {
                        assert_eq!(decimals, 6);
                        assert_eq!(*amount, Some(burn));
                        *amount = Some(0);
                    }
                    spl_token_2022_interface::instruction::TokenInstruction::CloseAccount => {
                        assert_eq!(*amount, Some(0));
                        *amount = None;
                    }
                    _ => panic!("unexpected token mutation"),
                }
                Ok(json!(tx.signatures[0].to_string()))
            }
            RpcRequest::GetSignatureStatuses => Ok(
                json!({"context":{"slot":2},"value":[{"slot":2,"confirmations":1,"err":null,"status":{"Ok":null},"confirmationStatus":"confirmed"}]}),
            ),
            _ => panic!("Unexpected RPC request {method}"),
        }
    }
    fn get_transport_stats(&self) -> solana_client::rpc_sender::RpcTransportStats {
        Default::default()
    }
    fn url(&self) -> String {
        "http://fixture.invalid".into()
    }
}
#[tokio::test]
async fn real_rpc_adapter_simulates_and_confirms_burn_and_close_without_sending_after_simulation_failure()
 {
    use solana_client::{nonblocking::rpc_client::RpcClient, rpc_request::RpcRequest};
    for fail in [false, true] {
        let signer = Keypair::new();
        let mut asset = asset(signer.pubkey(), 1, 7);
        let methods = std::sync::Arc::new(Mutex::new(vec![]));
        let amount = std::sync::Arc::new(Mutex::new(Some(7)));
        let rpc = RpcClient::new_sender(
            TransactionRpc {
                methods: methods.clone(),
                amount: amount.clone(),
                simulation_fails: fail,
            },
            Default::default(),
        );
        let result = dock_flints::rpc::transactions::send_instructions(
            &rpc,
            &signer,
            &[burn_instruction(&asset, &signer.pubkey()).unwrap()],
            &Default::default(),
        )
        .await;
        if fail {
            assert!(matches!(result, Err(SwapError::Simulation(_))));
            assert_eq!(*amount.lock().unwrap(), Some(7));
            assert!(
                !methods
                    .lock()
                    .unwrap()
                    .contains(&RpcRequest::SendTransaction)
            );
        } else {
            assert!(result.is_ok());
            assert_eq!(*amount.lock().unwrap(), Some(0));
            asset.account.raw_amount = 0;
            assert!(
                dock_flints::rpc::transactions::send_instructions(
                    &rpc,
                    &signer,
                    &[close_instruction(&asset, &signer.pubkey()).unwrap()],
                    &Default::default()
                )
                .await
                .is_ok()
            );
            assert_eq!(*amount.lock().unwrap(), None);
            assert_eq!(
                methods
                    .lock()
                    .unwrap()
                    .iter()
                    .filter(|method| **method == RpcRequest::GetSignatureStatuses)
                    .count(),
                2
            );
        }
    }
}
