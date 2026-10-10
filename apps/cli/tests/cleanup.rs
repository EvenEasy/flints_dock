use dock_flints::{
    app::cleanup::{execute::execute_plan, plan::build_plan, *},
    app::swap::*,
    core::*,
    infra::jupiter::swap::BuildResponse,
    infra::solana::cleanup::{
        associated_address, burn_instruction, close_instruction, source_instructions,
        transaction_accounting,
    },
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
            supply: amount.max(1_000_000),
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
                let mut build: BuildResponse = serde_json::from_str(include_str!(
                    "../../../tests/fixtures/jupiter_build.json"
                ))
                .unwrap();
                build.input_mint = request.mint.to_string();
                build.in_amount = request.raw_amount.to_string();
                build.slippage_bps = request.slippage_bps;
                Ok(PreparedSwap {
                    request: request.clone(),
                    quote: build.quote(request).unwrap(),
                    build: build.try_into().unwrap(),
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
                if asset.account.decimals == Some(0) {
                    let stored = assets.get_mut(&asset.account.address).unwrap();
                    stored.kind = AssetKind::Unknown;
                    stored.mint.as_mut().unwrap().supply = 0;
                }
                assets
                    .get_mut(&asset.account.address)
                    .unwrap()
                    .account
                    .raw_amount = if self.leave_balance { 1 } else { 0 };
            }
            CleanupOperation::Close => {
                assert!(asset.account.raw_amount == 0 || asset.account.is_native);
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
    dock_flints::cli::output::cleanup::write_plan(&mut output, &plan).unwrap();
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
        asset.account.program_id = program.id().to_string();
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
fn cli_defaults_to_preview_and_requires_signer_for_execution() {
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
    use dock_flints::infra::jupiter::swap::api_error;
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
        serde_json::from_str(include_str!("../../../tests/fixtures/jupiter_build.json")).unwrap();
    let request =
        dock_flints::app::cleanup::plan::request(&asset, &signer.pubkey(), &options()).unwrap();
    build.input_mint = request.mint.to_string();
    build.in_amount = request.raw_amount.to_string();
    let prepared = PreparedSwap {
        quote: build.quote(&request).unwrap(),
        request,
        build: build.try_into().unwrap(),
        requested_at: Instant::now(),
    };
    let (prefix, suffix) = source_instructions(&asset, &signer.pubkey(), false).unwrap();
    let tx = dock_flints::infra::solana::swap::signed_transaction_with_extras(
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
    methods: std::sync::Arc<Mutex<Vec<solana_rpc_client_api::request::RpcRequest>>>,
    amount: std::sync::Arc<Mutex<Option<u64>>>,
    simulation_fails: bool,
}
#[async_trait::async_trait]
impl solana_rpc_client::rpc_sender::RpcSender for TransactionRpc {
    async fn send(
        &self,
        method: solana_rpc_client_api::request::RpcRequest,
        params: Value,
    ) -> solana_rpc_client_api::client_error::Result<Value> {
        use base64::Engine;
        use solana_rpc_client_api::request::RpcRequest;
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
    fn get_transport_stats(&self) -> solana_rpc_client::rpc_sender::RpcTransportStats {
        Default::default()
    }
    fn url(&self) -> String {
        "http://fixture.invalid".into()
    }
}
#[tokio::test]
async fn real_rpc_adapter_simulates_and_confirms_burn_and_close_without_sending_after_simulation_failure()
 {
    use solana_rpc_client::nonblocking::rpc_client::RpcClient;
    use solana_rpc_client_api::request::RpcRequest;
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
        let result = dock_flints::infra::solana::transactions::send_instructions(
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
                dock_flints::infra::solana::transactions::send_instructions(
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

#[tokio::test]
async fn ignored_mints_protect_all_backing_accounts_even_empty_without_quotes_or_mutations() {
    let signer = Keypair::new();
    let assets = vec![
        asset(signer.pubkey(), 20, 50),
        asset(signer.pubkey(), 21, 0),
    ];
    let provider = Provider::new(vec![]); // Any quote is an error: ignore happens first.
    let executor = Executor::new(&assets);
    let mut opts = options();
    opts.selection
        .ignored_mints
        .insert(assets[0].account.mint.clone());
    let plan = build_plan(
        &signer.pubkey(),
        assets,
        ScanStatus::Complete,
        vec![],
        &provider,
        &opts,
    )
    .await;
    assert_eq!(plan.summary.accounts_to_close, 0);
    assert_eq!(plan.summary.unsupported, 2);
    assert!(
        plan.entries
            .iter()
            .all(|entry| entry.reason.contains("protected"))
    );
    // Saved exclusions survive even if a caller supplies default execution options.
    let report = execute_plan(&plan, &provider, &executor, &signer, &options())
        .await
        .unwrap();
    assert_eq!(report.skipped, 2);
    assert!(executor.events.lock().unwrap().is_empty());
    assert!(provider.requests.lock().unwrap().is_empty());
}

#[tokio::test]
async fn execution_can_add_protection_and_ignored_wsol_blocks_implicit_unwrap() {
    let signer = Keypair::new();
    let assets = vec![asset(signer.pubkey(), 22, 50)];
    let provider = Provider::new(vec![Answer::Route]);
    let executor = Executor::new(&assets);
    let plan = build_plan(
        &signer.pubkey(),
        assets.clone(),
        ScanStatus::Complete,
        vec![],
        &provider,
        &options(),
    )
    .await;
    let mut opts = options();
    opts.selection
        .ignored_mints
        .insert(assets[0].account.mint.clone());
    let report = execute_plan(&plan, &provider, &executor, &signer, &opts)
        .await
        .unwrap();
    assert_eq!(report.skipped, 1);
    assert!(executor.events.lock().unwrap().is_empty());
    opts.selection.ignored_mints.clear();
    opts.selection.ignored_mints.insert(WRAPPED_SOL.into());
    let provider = Provider::new(vec![Answer::Route]);
    let plan = build_plan(
        &signer.pubkey(),
        assets,
        ScanStatus::Complete,
        vec![],
        &provider,
        &opts,
    )
    .await;
    assert_eq!(plan.entries[0].category, CleanupCategory::Unsupported);
    assert!(plan.entries[0].reason.contains("WSOL"));
}

#[tokio::test]
async fn explicit_discard_never_calls_provider_and_burns_exactly_approved_accounts() {
    let signer = Keypair::new();
    let mut full = asset(signer.pubkey(), 1, 9007199254740993);
    full.account.decimals = Some(0);
    full.mint.as_mut().unwrap().decimals = 0;
    full.kind = AssetKind::FungibleAsset;
    let mut empty = asset(signer.pubkey(), 2, 0);
    empty.kind = AssetKind::Unknown;
    empty.mint = None;
    let provider = Provider::new(vec![]);
    let executor = Executor::new(&[full.clone(), empty.clone()]);
    let options = CleanupOptions {
        policy: CleanupPolicy::ExplicitDiscard,
        ..options()
    };
    let plan = build_plan(
        &signer.pubkey(),
        vec![full.clone(), empty],
        ScanStatus::Complete,
        vec![],
        &provider,
        &options,
    )
    .await;
    assert_eq!(plan.summary.burnable, 1);
    assert_eq!(plan.summary.accounts_to_close, 2);
    assert_eq!(
        plan.entries[0].reason_code,
        CleanupReasonCode::ExplicitDiscard
    );
    assert_eq!(plan.summary.estimated_swap_lamports, 0);
    let instruction = burn_instruction(&full, &signer.pubkey()).unwrap();
    assert_eq!(instruction.data[0], 15);
    assert_eq!(
        u64::from_le_bytes(instruction.data[1..9].try_into().unwrap()),
        9007199254740993
    );
    assert_eq!(instruction.data[9], 0);
    let report = execute_plan(&plan, &provider, &executor, &signer, &options)
        .await
        .unwrap();
    assert_eq!(report.closed, 2);
    assert_eq!(report.failed, 0);
    assert_eq!(
        executor
            .events
            .lock()
            .unwrap()
            .iter()
            .map(|(_, o)| *o)
            .collect::<Vec<_>>(),
        vec![
            CleanupOperation::Burn,
            CleanupOperation::Close,
            CleanupOperation::Close
        ]
    );
    assert!(provider.requests.lock().unwrap().is_empty());
    assert!(
        execute_plan(&plan, &provider, &executor, &signer, &self::options())
            .await
            .is_err()
    );
}

#[tokio::test]
async fn explicit_discard_keeps_nfts_frozen_accounts_none_and_ignored_empty_accounts() {
    let signer = Keypair::new();
    let mut nft = asset(signer.pubkey(), 1, 1);
    nft.kind = AssetKind::NonFungible;
    let mut frozen = asset(signer.pubkey(), 2, 2);
    frozen.account.state = "Frozen".into();
    let empty = asset(signer.pubkey(), 3, 0);
    let provider = Provider::new(vec![]);
    let mut options = CleanupOptions {
        policy: CleanupPolicy::ExplicitDiscard,
        ..options()
    };
    let plan = build_plan(
        &signer.pubkey(),
        vec![nft, frozen, empty.clone()],
        ScanStatus::Complete,
        vec![],
        &provider,
        &options,
    )
    .await;
    assert_eq!(
        plan.entries[0].reason_code,
        CleanupReasonCode::NftUnsupported
    );
    assert_eq!(plan.entries[1].reason_code, CleanupReasonCode::Frozen);
    assert_eq!(plan.summary.empty, 1);
    options
        .selection
        .ignored_mints
        .insert(empty.account.mint.clone());
    let plan = build_plan(
        &signer.pubkey(),
        vec![empty.clone()],
        ScanStatus::Complete,
        vec![],
        &provider,
        &options,
    )
    .await;
    assert_eq!(plan.entries[0].reason_code, CleanupReasonCode::IgnoredMint);
    options.selection.ignored_mints.clear();
    options.selection.none = true;
    let plan = build_plan(
        &signer.pubkey(),
        vec![empty],
        ScanStatus::Complete,
        vec![],
        &provider,
        &options,
    )
    .await;
    assert_eq!(plan.summary.accounts_to_close, 0);
    assert_eq!(plan.entries[0].reason_code, CleanupReasonCode::NotSelected);
}

#[tokio::test]
async fn explicit_discard_changed_balance_and_residual_balance_never_close() {
    for changed in [true, false] {
        let signer = Keypair::new();
        let full = asset(signer.pubkey(), 1, 42);
        let provider = Provider::new(vec![]);
        let options = CleanupOptions {
            policy: CleanupPolicy::ExplicitDiscard,
            ..options()
        };
        let plan = build_plan(
            &signer.pubkey(),
            vec![full.clone()],
            ScanStatus::Complete,
            vec![],
            &provider,
            &options,
        )
        .await;
        let mut executor = Executor::new(std::slice::from_ref(&full));
        if changed {
            executor
                .assets
                .lock()
                .unwrap()
                .get_mut(&full.account.address)
                .unwrap()
                .account
                .raw_amount += 1;
        } else {
            executor.leave_balance = true;
        }
        let report = execute_plan(&plan, &provider, &executor, &signer, &options)
            .await
            .unwrap();
        assert_eq!(report.failed, 1);
        assert_eq!(report.closed, 0);
        assert!(
            executor
                .events
                .lock()
                .unwrap()
                .iter()
                .all(|(_, o)| *o != CleanupOperation::Close)
        );
    }
}

#[tokio::test]
async fn explicit_discard_partial_failures_preserve_receipts_and_uncertain_burn_is_not_resent() {
    for uncertain in [false, true] {
        let signer = Keypair::new();
        let assets = vec![asset(signer.pubkey(), 1, 42), asset(signer.pubkey(), 2, 13)];
        let provider = Provider::new(vec![]);
        let options = CleanupOptions {
            policy: CleanupPolicy::ExplicitDiscard,
            ..options()
        };
        let plan = build_plan(
            &signer.pubkey(),
            assets.clone(),
            ScanStatus::Complete,
            vec![],
            &provider,
            &options,
        )
        .await;
        let mut executor = Executor::new(&assets);
        executor.fail_address = Some(assets[0].account.address.clone());
        executor.fail_operation = Some(CleanupOperation::Burn);
        executor.uncertain = uncertain;
        let report = execute_plan(&plan, &provider, &executor, &signer, &options)
            .await
            .unwrap();
        assert_eq!(report.failed, 1);
        if uncertain {
            assert_eq!(report.skipped, 1);
            assert_eq!(report.closed, 0);
            assert_eq!(executor.events.lock().unwrap().len(), 1);
            assert_eq!(
                report.results[0].uncertain_signature.as_deref(),
                Some("pending-signature")
            );
            assert!(!report.accounting_complete);
        } else {
            assert_eq!(report.closed, 1);
            assert_eq!(report.results[1].operations.len(), 2);
            assert_eq!(report.known_swap_net_lamports, 0);
        }
    }
}

#[test]
fn empty_unknown_account_closes_without_mint_metadata_or_liquidation_classification() {
    let owner = key(9);
    let mut empty = asset(owner, 1, 0);
    empty.kind = AssetKind::Unknown;
    empty.mint = None;
    empty.account.decimals = None;
    empty.account.state = "Frozen".into();
    assert!(close_instruction(&empty, &owner).is_ok());
    assert!(burn_instruction(&empty, &owner).is_err());
    empty.account.close_authority = Some(key(8).to_string());
    assert!(close_instruction(&empty, &owner).is_err());
}

#[tokio::test]
async fn complete_plan_covers_every_recorded_devnet_account_without_fake_no_route() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../docs/fixtures/devnet-account-audit.json"
    ))
    .unwrap();
    let owner: Pubkey = fixture["wallet"].as_str().unwrap().parse().unwrap();
    let mut assets = vec![];
    for (index, row) in fixture["accounts"].as_array().unwrap().iter().enumerate() {
        let amount = row["rawAmount"].as_str().unwrap().parse().unwrap();
        let mut source = asset(owner, (index + 1) as u8, amount);
        source.account.address = row["account"].as_str().unwrap().into();
        source.account.mint = row["mint"].as_str().unwrap().into();
        source.account.lamports = row["lamports"].as_str().unwrap().parse().unwrap();
        source.account.decimals = Some(row["decimals"].as_u64().unwrap() as u8);
        source.mint.as_mut().unwrap().mint = source.account.mint.clone();
        source.mint.as_mut().unwrap().supply = row["supply"].as_str().unwrap().parse().unwrap();
        assets.push(source);
    }
    let provider = dock_flints::app::categories::ScopedSwap::<dock_flints::infra::jupiter::Jupiter> {
        provider: None,
        mainnet: false,
    };
    let plan = build_plan(
        &owner,
        assets.clone(),
        ScanStatus::Complete,
        vec![],
        &provider,
        &options(),
    )
    .await;
    assert_eq!(plan.entries.len(), 26);
    assert_eq!(plan.summary.burnable, 20);
    assert_eq!(plan.summary.accounts_to_close, 26);
    assert!(
        plan.entries
            .iter()
            .filter(|e| e.category == CleanupCategory::Burnable)
            .all(|e| e.reason_code == CleanupReasonCode::RoutingUnavailable)
    );
    let expected: std::collections::BTreeMap<_, _> = fixture["accounts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| (row["account"].as_str().unwrap(), row))
        .collect();
    assert_eq!(
        plan.entries
            .iter()
            .map(|entry| entry.asset.account.address.as_str())
            .collect::<std::collections::BTreeSet<_>>(),
        expected.keys().copied().collect()
    );
    for entry in &plan.entries {
        let row = expected[entry.asset.account.address.as_str()];
        assert_eq!(entry.asset.account.mint, row["mint"]);
        assert_eq!(entry.asset.account.raw_amount.to_string(), row["rawAmount"]);
        assert_eq!(entry.asset.account.program.id().to_string(), row["program"]);
        assert_eq!(
            entry.asset.account.decimals.unwrap(),
            row["decimals"].as_u64().unwrap() as u8
        );
    }
    if let Ok(path) = std::env::var("DOCK_FLINTS_FIXTURE_PLAN_REPORT") {
        std::fs::write(path,serde_json::to_string_pretty(&json!({"source":"Historical RPC fixture replay through real core planner; not live inventory","capturedAt":fixture["capturedAt"],"network":fixture["network"],"wallet":owner.to_string(),"plan":plan,"transactionsSubmitted":0})).unwrap()).unwrap();
    }
    let mut options = options();
    options
        .selection
        .ignored_mints
        .insert(assets[0].account.mint.clone());
    let protected = build_plan(
        &owner,
        assets.clone(),
        ScanStatus::Complete,
        vec![],
        &provider,
        &options,
    )
    .await;
    assert_eq!(protected.summary.accounts_to_close, 25);
    assert_eq!(
        protected.entries[0].reason_code,
        CleanupReasonCode::IgnoredMint
    );
    options.selection.none = true;
    let none = build_plan(
        &owner,
        assets,
        ScanStatus::Complete,
        vec![],
        &provider,
        &options,
    )
    .await;
    assert_eq!(none.executable_count(), 0);
    assert!(!none.has_blocked_selection());
}

#[tokio::test]
async fn structurally_unavailable_burn_is_revalidated_and_never_degrades_a_saved_swap() {
    let signer = Keypair::new();
    let sources = vec![asset(signer.pubkey(), 1, 20)];
    let provider = dock_flints::app::categories::ScopedSwap::<dock_flints::infra::jupiter::Jupiter> {
        provider: None,
        mainnet: false,
    };
    let plan = build_plan(
        &signer.pubkey(),
        sources.clone(),
        ScanStatus::Complete,
        vec![],
        &provider,
        &options(),
    )
    .await;
    let executor = Executor::new(&sources);
    let report = execute_plan(&plan, &provider, &executor, &signer, &options())
        .await
        .unwrap();
    assert_eq!((report.completed, report.closed, report.failed), (1, 1, 0));
    assert_eq!(
        executor
            .events
            .lock()
            .unwrap()
            .iter()
            .map(|e| e.1)
            .collect::<Vec<_>>(),
        [CleanupOperation::Burn, CleanupOperation::Close]
    );
    // Capability changed after approval: a real route is not replaced with the old burn intent.
    let route = Provider::new(vec![Answer::Route]);
    let executor = Executor::new(&sources);
    let report = execute_plan(&plan, &route, &executor, &signer, &options())
        .await
        .unwrap();
    assert_eq!(report.failed, 1);
    assert!(executor.events.lock().unwrap().is_empty());
}

#[tokio::test]
async fn selected_wsol_unwraps_without_burn_or_route_lookup() {
    let signer = Keypair::new();
    let mut wsol = asset(signer.pubkey(), 1, 100_000);
    wsol.account.is_native = true;
    wsol.account.mint = WRAPPED_SOL.into();
    wsol.account.native_reserve_lamports = Some(2_039_280);
    let provider = Provider::new(vec![]);
    let plan = plan(&signer, vec![wsol.clone()], &provider).await;
    assert_eq!(plan.entries[0].reason_code, CleanupReasonCode::NativeUnwrap);
    assert_eq!(plan.summary.burnable, 0);
    let instruction = close_instruction(&wsol, &signer.pubkey()).unwrap();
    assert_eq!(instruction.data, [9]);
    assert_eq!(instruction.accounts[1].pubkey, signer.pubkey());
    assert!(burn_instruction(&wsol, &signer.pubkey()).is_err());
}

#[tokio::test]
async fn approved_wsol_unwrap_precedes_swaps_even_when_discovered_last() {
    let signer = Keypair::new();
    let token = asset(signer.pubkey(), 1, 5);
    let mut wsol = asset(signer.pubkey(), 2, 100_000);
    wsol.account.is_native = true;
    wsol.account.mint = WRAPPED_SOL.into();
    wsol.account.native_reserve_lamports = Some(2_039_280);
    let sources = vec![token.clone(), wsol.clone()];
    let provider = Provider::new(vec![Answer::Route, Answer::Route]);
    let plan = plan(&signer, sources.clone(), &provider).await;
    let executor = Executor::new(&sources);
    let report = execute_plan(&plan, &provider, &executor, &signer, &options())
        .await
        .unwrap();
    assert_eq!((report.completed, report.failed), (2, 0));
    assert_eq!(
        *executor.events.lock().unwrap(),
        vec![
            (wsol.account.address, CleanupOperation::Close),
            (token.account.address.clone(), CleanupOperation::Swap),
            (token.account.address, CleanupOperation::Close),
        ]
    );
}

#[tokio::test]
async fn unresolved_wsol_unwrap_blocks_dependent_swaps_without_resend() {
    let signer = Keypair::new();
    let token = asset(signer.pubkey(), 1, 5);
    let mut wsol = asset(signer.pubkey(), 2, 100_000);
    wsol.account.is_native = true;
    wsol.account.mint = WRAPPED_SOL.into();
    wsol.account.native_reserve_lamports = Some(2_039_280);
    let sources = vec![token, wsol.clone()];
    let provider = Provider::new(vec![Answer::Route]);
    let plan = plan(&signer, sources.clone(), &provider).await;
    let executor = Executor {
        fail_address: Some(wsol.account.address.clone()),
        fail_operation: Some(CleanupOperation::Close),
        uncertain: true,
        ..Executor::new(&sources)
    };
    let report = execute_plan(&plan, &provider, &executor, &signer, &options())
        .await
        .unwrap();
    assert_eq!((report.completed, report.failed, report.skipped), (0, 1, 1));
    assert!(!report.accounting_complete);
    assert_eq!(
        *executor.events.lock().unwrap(),
        vec![(wsol.account.address, CleanupOperation::Close)]
    );
    assert_eq!(
        report.results[0].uncertain_signature.as_deref(),
        Some("pending-signature")
    );
    assert!(
        report.results[1]
            .reason
            .contains("WSOL source was not safely unwrapped")
    );
    assert_eq!(
        provider.requests.lock().unwrap().len(),
        1,
        "No quote or send for a swap depending on unresolved WSOL"
    );
}
