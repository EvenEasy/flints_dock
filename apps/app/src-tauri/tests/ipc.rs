use dock_flints_app::{
    dto::{
        assets::ScanStatusDto,
        wallet::{AnalyzeWalletRequestDto, WalletAnalysisDto},
    },
    error::ErrorCode,
    state::AppState,
};
use dock_flints_core::core::*;
use serde_json::{Value, json};
use std::collections::BTreeMap;

fn request(selection: Value) -> AnalyzeWalletRequestDto {
    serde_json::from_value(json!({
        "walletAddress": "11111111111111111111111111111111",
        "selection": selection,
        "noPrices": true,
    }))
    .unwrap()
}

fn snapshot(selection: ScanSelection) -> WalletSnapshot {
    WalletSnapshot {
        selected: selection,
        owner: "11111111111111111111111111111111".into(),
        commitment: "confirmed".into(),
        native_sol: None,
        token_accounts: vec![],
        tokens: vec![],
        all_tokens: vec![],
        mints: vec![],
        classic_nfts: vec![],
        core_assets: vec![],
        unknown_assets: vec![],
        scanners: BTreeMap::new(),
        account_summary: amount::summarize_accounts(&[]),
    }
}

#[test]
fn selection_matches_cli_defaults_and_all_31_category_combinations() {
    let (_, defaults) = request(json!({})).into_core().unwrap();
    assert_eq!(defaults.selection, ScanSelection::ALL);
    assert!(defaults.no_prices);
    let omitted: AnalyzeWalletRequestDto = serde_json::from_value(json!({
        "walletAddress": "11111111111111111111111111111111",
    }))
    .unwrap();
    assert_eq!(omitted.into_core().unwrap().1.selection, ScanSelection::ALL);

    for mask in 1..32 {
        let expected = ScanSelection {
            balance: mask & 1 != 0,
            tokens: mask & 2 != 0,
            all_tokens: mask & 4 != 0,
            nfts: mask & 8 != 0,
            cnfts: mask & 16 != 0,
        };
        let selection = json!({"balance": expected.balance, "tokens": expected.tokens,
            "allTokens": expected.all_tokens, "nfts": expected.nfts, "cnfts": expected.cnfts});
        assert_eq!(
            request(selection).into_core().unwrap().1.selection,
            expected
        );
    }
    assert_eq!(
        request(json!({"all": true, "allTokens": true}))
            .into_core()
            .unwrap()
            .1
            .selection,
        ScanSelection {
            all_tokens: true,
            ..ScanSelection::ALL
        }
    );
}

#[test]
fn invalid_identity_has_stable_error_and_signing_fields_are_rejected() {
    let mut invalid = request(json!({"balance": true}));
    invalid.wallet_address = "not-a-public-key".into();
    let error = invalid.into_core().unwrap_err();
    assert_eq!(error.code, ErrorCode::InvalidWalletAddress);
    assert_eq!(
        serde_json::to_value(error).unwrap(),
        json!({
            "code":"invalid_wallet_address",
            "message":"walletAddress must be a valid Solana public key",
            "details":{"field":"walletAddress"},
        })
    );
    for field in ["seed", "keypair", "execute", "rpcUrl"] {
        let mut value = json!({"walletAddress":"11111111111111111111111111111111"});
        value[field] = json!("forbidden");
        assert!(serde_json::from_value::<AnalyzeWalletRequestDto>(value).is_err());
    }
    assert!(serde_json::from_value::<AnalyzeWalletRequestDto>(json!({})).is_err());
    assert!(
        serde_json::from_value::<AnalyzeWalletRequestDto>(json!({
            "walletAddress":"11111111111111111111111111111111", "selection":{"tokenz":true},
        }))
        .is_err()
    );
}

#[test]
fn blockchain_integers_and_decimal_balances_preserve_precision() {
    let mut model = snapshot(ScanSelection {
        all_tokens: true,
        ..ScanSelection::ALL
    });
    let metadata = TokenMetadata::default();
    model.native_sol = Some(NativeBalance {
        lamports: u64::MAX,
        price: Some(Price {
            usd: 123.45,
            source: "fixture".into(),
            block_id: Some(u64::MAX),
            decimals: 9,
        }),
        value_usd: None,
    });
    model.tokens = vec![TokenAsset {
        mint: "mint".into(),
        program: TokenProgram::Token2022,
        total_raw_amount: u128::MAX,
        decimals: Some(9),
        balance: Some(amount::exact_amount(u128::MAX, 9)),
        accounts: vec!["account".into()],
        kind: AssetKind::Unknown,
        metadata: metadata.clone(),
        price: None,
        value_usd: None,
    }];
    model.all_tokens = model.tokens.clone();
    model.token_accounts = vec![TokenAccount {
        address: "account".into(),
        mint: "mint".into(),
        program: TokenProgram::Token2022,
        program_id: TokenProgram::Token2022.id().to_string(),
        owner: model.owner.clone(),
        raw_amount: u64::MAX,
        decimals: None,
        lamports: u64::MAX,
        data_len: 165,
        state: "initialized".into(),
        delegate: None,
        delegated_amount: u64::MAX,
        close_authority: None,
        is_native: true,
        native_reserve_lamports: Some(u64::MAX),
        extensions: json!({"unsafeInteger":u64::MAX}),
        extension_types: vec!["ImmutableOwner".into()],
        closure: ClosureAssessment::NeedsReview("fixture".into()),
    }];
    model.mints = vec![MintInfo {
        mint: "mint".into(),
        program: TokenProgram::Token2022,
        decimals: 9,
        supply: u64::MAX,
        mint_authority: None,
        freeze_authority: None,
        metadata,
        extensions: json!({"unsafeInteger":u64::MAX}),
        extension_types: vec![],
    }];
    model.core_assets = vec![CoreAsset {
        address: "core".into(),
        owner: model.owner.clone(),
        name: "fixture".into(),
        uri: "".into(),
        lamports: u64::MAX,
        data_len: 100,
        update_authority: "authority".into(),
        collection: None,
        plugins_status: ScanStatus::Partial("plugin unavailable".into()),
    }];
    model.unknown_assets = vec![UnknownAsset {
        address: "unknown".into(),
        program_id: None,
        lamports: Some(u64::MAX),
        reason: "fixture".into(),
        data: Some(json!({"unsafeInteger":u64::MAX})),
    }];
    model.account_summary.token_account_lamports = u128::MAX;
    model.account_summary.potentially_reclaimable_lamports = u128::MAX;
    for name in [
        "native_sol",
        "tokens",
        "all_tokens",
        "classic_nfts",
        "core_asset_v1",
        "nfts",
    ] {
        model.scanners.insert(name.into(), ScanStatus::Complete);
    }
    model.scanners.insert(
        "compressed_nfts".into(),
        ScanStatus::Unsupported("historical index required".into()),
    );
    let value = serde_json::to_value(WalletAnalysisDto::from(model)).unwrap();
    for path in [
        "/balance/value/lamports",
        "/balance/value/price/blockId",
        "/tokenAccounts/0/rawAmount",
        "/tokenAccounts/0/lamports",
        "/tokenAccounts/0/delegatedAmount",
        "/tokenAccounts/0/nativeReserveLamports",
        "/mints/0/supply",
        "/nfts/core/items/0/lamports",
        "/unknownAssets/0/lamports",
    ] {
        assert_eq!(
            value.pointer(path).unwrap().as_str(),
            Some(u64::MAX.to_string().as_str()),
            "{path}"
        );
    }
    for path in [
        "/accountSummary/tokenAccountLamports",
        "/accountSummary/potentiallyReclaimableLamports",
    ] {
        assert_eq!(
            value.pointer(path).unwrap().as_str(),
            Some(u128::MAX.to_string().as_str()),
            "{path}"
        );
    }
    assert_eq!(
        value["tokens"]["items"][0]["totalRawAmount"],
        u64::MAX.to_string()
    );
    assert_eq!(
        value["allTokens"]["items"][0]["totalRawAmount"],
        u64::MAX.to_string()
    );
    assert_eq!(value["balance"]["value"]["sol"], "18446744073.709551615");
    assert!(
        value["tokens"]["items"][0]["balance"].is_null(),
        "Missing account decimals cannot be overwritten by a stale view"
    );
    assert!(value["tokenAccounts"][0]["decimals"].is_null());
    assert_eq!(value["tokens"]["items"][0]["kind"], "unknown");
    assert_eq!(value["tokenAccounts"][0]["program"], "token2022");
    assert_eq!(value["tokenAccounts"][0]["dataLen"], "165");
    assert_eq!(
        value["tokenAccounts"][0]["closure"]["status"],
        "needsReview"
    );
    assert!(value["tokenAccounts"][0].get("extensions").is_none());
    assert!(value["mints"][0].get("extensions").is_none());
    assert!(value["unknownAssets"][0].get("data").is_none());
    assert!(value["cnfts"]["items"].is_null());
    assert!(value["hasUsableResults"].as_bool().unwrap());

    fn check_numbers(value: &Value) {
        match value {
            Value::Number(number) if number.is_u64() => {
                assert!(number.as_u64().unwrap() <= 9_007_199_254_740_991)
            }
            Value::Array(values) => values.iter().for_each(check_numbers),
            Value::Object(values) => values.values().for_each(check_numbers),
            _ => {}
        }
    }
    check_numbers(&value);
}

#[test]
fn category_mapping_distinguishes_failed_empty_partial_and_unselected() {
    for (status, available) in [
        (ScanStatus::Complete, true),
        (ScanStatus::Partial("metadata unavailable".into()), true),
        (ScanStatus::Failed("RPC unavailable".into()), false),
        (ScanStatus::Unsupported("unsupported".into()), false),
        (ScanStatus::Skipped("skipped".into()), false),
    ] {
        let mut model = snapshot(ScanSelection {
            tokens: true,
            ..Default::default()
        });
        model.scanners.insert("tokens".into(), status);
        let value = serde_json::to_value(WalletAnalysisDto::from(model)).unwrap();
        assert_eq!(value["tokens"]["items"].is_array(), available);
        for field in ["balance", "allTokens", "nfts", "cnfts"] {
            assert!(value[field].is_null());
        }
        assert_eq!(value["hasUsableResults"], available);
        assert_eq!(value["tokenAccounts"].is_array(), available);
        assert_eq!(value["mints"].is_array(), available);
        assert_eq!(value["accountSummary"].is_object(), available);
    }
    let mut model = snapshot(ScanSelection {
        cnfts: true,
        ..Default::default()
    });
    model.scanners.insert(
        "compressed_nfts".into(),
        ScanStatus::Unsupported("historical index required".into()),
    );
    let value = serde_json::to_value(WalletAnalysisDto::from(model)).unwrap();
    assert_eq!(value["cnfts"]["status"]["status"], "unsupported");
    assert!(value["cnfts"]["items"].is_null());
    assert!(value["tokenAccounts"].is_null());
    assert!(value["accountSummary"].is_null());
    assert_eq!(value["hasUsableResults"], false);
}

#[test]
fn scanner_diagnostics_are_bounded_but_actionable() {
    let status = ScanStatusDto::from(ScanStatus::Failed(format!(
        "RPC unavailable\n{}",
        "x".repeat(2000)
    )));
    let value = serde_json::to_value(status).unwrap();
    let reason = value["reason"].as_str().unwrap();
    assert!(reason.starts_with("RPC unavailable"));
    assert_eq!(reason.chars().count(), 513);
    assert!(!reason.contains('\n'));
}

#[tokio::test]
async fn state_rejects_bad_configuration_and_prices_can_fail_independently() {
    for (url, timeout) in [
        ("not-a-url", 30),
        ("file:///tmp/rpc", 30),
        ("http://localhost", 0),
        ("http://localhost", 301),
    ] {
        assert!(
            matches!(AppState::new(url.into(), timeout, None), Err(error) if error.code == ErrorCode::InvalidConfiguration)
        );
    }
    assert!(AppState::new("http://127.0.0.1:8899".into(), 30, None).is_ok());
    assert!(
        AppState::new(
            "http://127.0.0.1:8899".into(),
            30,
            Some("invalid\nheader".into())
        )
        .is_ok()
    );
}

// Tauri's local custom-protocol origin uses an HTTP alias on Windows.
const LOCAL_ORIGIN: &str = if cfg!(windows) {
    "http://tauri.localhost"
} else {
    "tauri://localhost"
};

fn invoke(
    window: &tauri::WebviewWindow<tauri::test::MockRuntime>,
    command: &str,
    request: Value,
    url: &str,
) -> Result<Value, Value> {
    tauri::test::get_ipc_response(
        window,
        tauri::webview::InvokeRequest {
            cmd: command.into(),
            callback: tauri::ipc::CallbackFn(0),
            error: tauri::ipc::CallbackFn(1),
            url: url.parse().unwrap(),
            body: tauri::ipc::InvokeBody::Json(json!({"request":request})),
            headers: Default::default(),
            invoke_key: tauri::test::INVOKE_KEY.into(),
        },
    )
    .map(|body| body.deserialize().unwrap())
}

fn desktop(state: AppState) -> tauri::App<tauri::test::MockRuntime> {
    tauri::test::mock_builder()
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            dock_flints_app::commands::wallet::analyze_wallet,
            dock_flints_app::commands::identity::connect_wallet,
            dock_flints_app::commands::identity::disconnect_wallet,
            dock_flints_app::commands::cleanup::prepare_cleanup,
            dock_flints_app::commands::cleanup::execute_cleanup,
            dock_flints_app::commands::cleanup::get_cleanup_job
        ])
        .build(tauri::generate_context!())
        .unwrap()
}

#[test]
fn ipc_uses_registered_read_only_command_and_enforces_local_window_capability() {
    let app = desktop(AppState::new("http://127.0.0.1:1".into(), 1, None).unwrap());
    let main = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();
    let read_request =
        json!({"walletAddress":"11111111111111111111111111111111", "selection":{"cnfts":true}});
    let response = invoke(&main, "analyze_wallet", read_request.clone(), LOCAL_ORIGIN).unwrap();
    assert_eq!(response["selected"]["cnfts"], true);
    assert_eq!(response["cnfts"]["status"]["status"], "unsupported");
    assert!(response["cnfts"]["items"].is_null());
    assert!(response["balance"].is_null());

    let invalid = invoke(
        &main,
        "analyze_wallet",
        json!({"walletAddress":"invalid"}),
        LOCAL_ORIGIN,
    )
    .unwrap_err();
    assert_eq!(invalid["code"], "invalid_wallet_address");
    assert_eq!(invalid["details"]["field"], "walletAddress");
    for malformed in [
        json!({}),
        json!({"walletAddress": 123}),
        json!({"walletAddress":"11111111111111111111111111111111","seed":"forbidden"}),
        json!({"walletAddress":"11111111111111111111111111111111","selection":{"tokenz":true}}),
    ] {
        let rejection = invoke(&main, "analyze_wallet", malformed, LOCAL_ORIGIN).unwrap_err();
        assert_eq!(rejection["code"], "invalid_request");
        assert!(rejection["details"]["reason"].is_string());
    }

    for command in ["swap", "cleanup", "burn", "close_account"] {
        assert!(invoke(&main, command, read_request.clone(), LOCAL_ORIGIN).is_err());
    }
    let other = tauri::WebviewWindowBuilder::new(&app, "other", Default::default())
        .build()
        .unwrap();
    assert!(invoke(&other, "analyze_wallet", read_request.clone(), LOCAL_ORIGIN).is_err());
    assert!(
        invoke(
            &main,
            "analyze_wallet",
            read_request,
            "https://untrusted.example"
        )
        .is_err()
    );
}

#[test]
fn ipc_calls_existing_core_scanner_and_serializes_exact_rpc_lamports() {
    use std::{
        io::{Read, Write},
        net::TcpListener,
    };
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let server = std::thread::spawn(move || {
        for _ in 0..4 {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                .unwrap();
            let mut bytes = Vec::new();
            let mut buffer = [0; 1024];
            let (header_end, content_length) = loop {
                let read = stream.read(&mut buffer).unwrap();
                assert!(read > 0);
                bytes.extend_from_slice(&buffer[..read]);
                if let Some(end) = bytes.windows(4).position(|part| part == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&bytes[..end]);
                    let length = headers
                        .lines()
                        .find_map(|line| {
                            let (key, value) = line.split_once(':')?;
                            key.eq_ignore_ascii_case("content-length")
                                .then(|| value.trim().parse::<usize>().unwrap())
                        })
                        .unwrap();
                    break (end + 4, length);
                }
            };
            while bytes.len() < header_end + content_length {
                let read = stream.read(&mut buffer).unwrap();
                assert!(read > 0);
                bytes.extend_from_slice(&buffer[..read]);
            }
            let request: Value =
                serde_json::from_slice(&bytes[header_end..header_end + content_length]).unwrap();
            if request["method"] == "getGenesisHash" {
                let body = json!({"jsonrpc":"2.0","id":request["id"],"result":"EtWTRABZaYq6iMfeYKouRu166VU2xqa1wcaWoxPkrZBG"}).to_string();
                write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).unwrap();
                continue;
            }
            assert_eq!(request["method"], "getBalance");
            assert_eq!(request["params"][1]["commitment"], "confirmed");
            let body=json!({"jsonrpc":"2.0","id":request["id"],"result":{"context":{"slot":1},"value":u64::MAX}}).to_string();
            write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).unwrap();
        }
    });
    let app = desktop(AppState::new(url, 5, None).unwrap());
    let window = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();
    for _ in 0..2 {
        let response = invoke(
            &window,
            "analyze_wallet",
            json!({
                "walletAddress":"11111111111111111111111111111111",
                "selection":{"balance":true},"noPrices":true,
            }),
            LOCAL_ORIGIN,
        )
        .unwrap();
        assert_eq!(
            response["balance"]["value"]["lamports"],
            u64::MAX.to_string()
        );
        assert_eq!(response["balance"]["value"]["sol"], "18446744073.709551615");
        assert_eq!(response["balance"]["status"]["status"], "complete");
        assert_eq!(response["hasUsableResults"], true);
        assert!(response["tokens"].is_null());
    }
    server.join().unwrap();
}

#[test]
fn local_identity_sources_reuse_core_and_do_not_serialize_credentials() {
    use dock_flints_app::dto::identity::ConnectWalletRequestDto;
    let seed = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";
    let request: ConnectWalletRequestDto =
        serde_json::from_value(json!({"source":{"kind":"seed","base64":seed}})).unwrap();
    let identity = request.into_identity().unwrap();
    assert!(identity.signer().is_ok());
    let seed_address = identity.address;
    let public: ConnectWalletRequestDto = serde_json::from_value(
        json!({"source":{"kind":"publicKey","address": seed_address.to_string()}}),
    )
    .unwrap();
    assert!(public.into_identity().unwrap().signer().is_err());

    // Test fixtures are deterministic empty seeds, never real user wallet credentials.
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join(".cache/identity-tests");
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("fixture-keypair.json");
    std::fs::write(
        &file,
        serde_json::to_vec(&identity.signer().unwrap().to_bytes().to_vec()).unwrap(),
    )
    .unwrap();
    let file_request: ConnectWalletRequestDto = serde_json::from_value(
        json!({"source":{"kind":"keypairFile","path":file.to_str().unwrap()}}),
    )
    .unwrap();
    let loaded = file_request.into_identity().unwrap();
    assert_eq!(loaded.address, seed_address);
    assert!(loaded.signer().is_ok());
    std::fs::remove_file(file).unwrap();

    for bad in [
        json!({"source":{"kind":"seed","base64":"secret-value-not-base64"}}),
        json!({"source":{"kind":"keypairFile","path":"relative-keypair.json"}}),
    ] {
        let request: ConnectWalletRequestDto = serde_json::from_value(bad).unwrap();
        let error = request.into_identity().err().unwrap();
        assert_eq!(error.code, ErrorCode::InvalidWalletIdentity);
        let text = serde_json::to_string(&error).unwrap();
        assert!(!text.contains("secret-value-not-base64"));
        assert!(!text.contains("relative-keypair.json"));
    }
    assert!(
        serde_json::from_value::<ConnectWalletRequestDto>(
            json!({"source":{"kind":"seed","base64":seed,"address":"extra"}})
        )
        .is_err()
    );
}

#[test]
fn identity_ipc_returns_public_connection_and_enforces_local_capability() {
    let app = desktop(AppState::new("http://127.0.0.1:1".into(), 1, None).unwrap());
    let main = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();
    let seed = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";
    let request = json!({"source":{"kind":"seed","base64":seed}});
    let response = invoke(&main, "connect_wallet", request.clone(), LOCAL_ORIGIN).unwrap();
    assert_eq!(response["sourceKind"], "seed");
    assert_eq!(response["canSign"], true);
    assert_eq!(response.as_object().unwrap().len(), 4);
    assert!(response["walletAddress"].as_str().unwrap().len() >= 32);
    assert!(!serde_json::to_string(&response).unwrap().contains(seed));
    assert!(
        invoke(
            &main,
            "connect_wallet",
            request.clone(),
            "https://untrusted.example"
        )
        .is_err()
    );
    let other = tauri::WebviewWindowBuilder::new(&app, "other", Default::default())
        .build()
        .unwrap();
    assert!(invoke(&other, "connect_wallet", request, LOCAL_ORIGIN).is_err());
    let invalid = invoke(
        &main,
        "connect_wallet",
        json!({"source":{"kind":"seed","base64":1234}}),
        LOCAL_ORIGIN,
    )
    .unwrap_err();
    assert_eq!(invalid["code"], "invalid_wallet_identity");
    assert!(invalid["details"].is_null());
    let public = invoke(
        &main,
        "connect_wallet",
        json!({"source":{"kind":"publicKey","address":response["walletAddress"]}}),
        LOCAL_ORIGIN,
    )
    .unwrap();
    assert_eq!(public["canSign"], false);
    assert!(
        invoke(
            &main,
            "disconnect_wallet",
            json!({}),
            "https://untrusted.example"
        )
        .is_err()
    );
    assert!(invoke(&other, "disconnect_wallet", json!({}), LOCAL_ORIGIN).is_err());
    assert_eq!(
        invoke(&main, "disconnect_wallet", json!({}), LOCAL_ORIGIN).unwrap(),
        Value::Null
    );
}

#[test]
fn cleanup_commands_enforce_actual_ipc_input_and_window_permissions() {
    let app = desktop(AppState::new("http://127.0.0.1:1".into(), 1, None).unwrap());
    let main = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();
    let other = tauri::WebviewWindowBuilder::new(&app, "other", Default::default())
        .build()
        .unwrap();
    for command in ["prepare_cleanup", "execute_cleanup", "get_cleanup_job"] {
        let error = invoke(
            &main,
            command,
            json!({"arbitraryTransactions":"forbidden"}),
            LOCAL_ORIGIN,
        )
        .unwrap_err();
        assert_eq!(error["code"], "cleanup_rejected", "{command}: {error}");
        assert!(invoke(&other, command, json!({}), LOCAL_ORIGIN).is_err());
        assert!(invoke(&main, command, json!({}), "https://untrusted.example").is_err());
    }
}

/// Real Tauri IPC with a deterministic local JSON-RPC chain, never an external wallet or cluster.
#[test]
fn cleanup_ipc_preserves_mint_accounts_none_readonly_and_exact_confirmed_net() {
    run_cleanup_ipc(false, 0);
}
#[test]
fn unified_devnet_ipc_burns_confirmed_sources_then_closes_without_jupiter() {
    run_cleanup_ipc(true, 0);
}
#[test]
fn core_asset_only_ipc_selection_cannot_expand_to_all_spl_accounts_and_needs_no_close() {
    run_cleanup_ipc(false, 1);
}
#[test]
fn malformed_core_account_survives_normalization_selection_and_plan_without_submission() {
    run_cleanup_ipc(false, 2);
}
fn run_cleanup_ipc(explicit: bool, core_mode: u8) {
    let core_fixture = core_mode > 0;
    use base64::{Engine, engine::general_purpose::STANDARD};
    use solana_transaction::versioned::VersionedTransaction;
    use std::{
        io::{Read, Write},
        net::TcpListener,
        sync::{
            Arc, Mutex,
            atomic::{AtomicBool, Ordering},
        },
    };
    let fixture = dock_flints_core::infra::wallet::WalletIdentity::from_seed(
        "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=",
    )
    .unwrap();
    let owner = fixture.address.to_string();
    let mint = solana_pubkey::Pubkey::new_from_array([3; 32]).to_string();
    let accounts = [
        solana_pubkey::Pubkey::new_from_array([4; 32]).to_string(),
        solana_pubkey::Pubkey::new_from_array([5; 32]).to_string(),
    ];
    let mut token = vec![0u8; 165];
    token[..32].copy_from_slice(&solana_pubkey::Pubkey::new_from_array([3; 32]).to_bytes());
    token[32..64].copy_from_slice(&fixture.address.to_bytes());
    token[108] = 1;
    token[64..72].copy_from_slice(&(if explicit { 20u64 } else { 0u64 }).to_le_bytes());
    let mut empty_bytes = token.clone();
    empty_bytes[64..72].copy_from_slice(&0u64.to_le_bytes());
    let token = json!({"lamports":2039280,"owner":TokenProgram::Legacy.id().to_string(),"data":[STANDARD.encode(token),"base64"],"executable":false,"rentEpoch":0});
    let empty_token = json!({"lamports":2039280,"owner":TokenProgram::Legacy.id().to_string(),"data":[STANDARD.encode(empty_bytes),"base64"],"executable":false,"rentEpoch":0});
    let mut mint_bytes = vec![0u8; 82];
    mint_bytes[36..44].copy_from_slice(&100000000u64.to_le_bytes());
    mint_bytes[44] = 6;
    mint_bytes[45] = 1;
    let mint_value = json!({"lamports":1000000,"owner":TokenProgram::Legacy.id().to_string(),"data":[STANDARD.encode(mint_bytes),"base64"],"executable":false,"rentEpoch":0});
    let core_id = solana_pubkey::Pubkey::new_from_array([9; 32]).to_string();
    let core_program = "CoREENxT6tW1HoK8ypY1SxRMZTcVPm7R94rH4PZNhX7d";
    let mut core_bytes = vec![1]; // AssetV1, owner, UpdateAuthority::None, name, URI, seq=None.
    core_bytes.extend(fixture.address.to_bytes());
    core_bytes.push(0);
    for text in [
        "Owned Core IPC fixture",
        "https://fixture.invalid/core.json",
    ] {
        core_bytes.extend((text.len() as u32).to_le_bytes());
        core_bytes.extend(text.as_bytes());
    }
    core_bytes.push(0);
    if core_mode == 2 {
        core_bytes.truncate(33);
    }
    let core_value = json!({"lamports":2039280,"owner":core_program,"data":[STANDARD.encode(core_bytes),"base64"],"executable":false,"rentEpoch":0});
    let core_address = core_id.clone();
    let owner_bytes = fixture.address.to_bytes();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let stop = Arc::new(AtomicBool::new(false));
    let stopped = stop.clone();
    let sends = Arc::new(Mutex::new(Vec::<String>::new()));
    let sent = sends.clone();
    let wallet = owner.clone();
    let account_list = accounts.clone();
    let mint_address = mint.clone();
    let server = std::thread::spawn(move || {
        let mut burned = std::collections::BTreeSet::new();
        let mut closed = std::collections::BTreeSet::new();
        let mut signatures = BTreeMap::new();
        while !stopped.load(Ordering::Relaxed) {
            let (mut stream, _) = match listener.accept() {
                Ok(v) => v,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(std::time::Duration::from_millis(2));
                    continue;
                }
                Err(e) => panic!("{e}"),
            };
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                .unwrap();
            let mut bytes = Vec::new();
            let mut buffer = [0u8; 8192];
            let request: Value = loop {
                let count = stream.read(&mut buffer).unwrap();
                assert!(count > 0);
                bytes.extend_from_slice(&buffer[..count]);
                if let Some(end) = bytes.windows(4).position(|v| v == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&bytes[..end]);
                    let length: usize = headers
                        .lines()
                        .find_map(|l| {
                            l.split_once(':')
                                .filter(|(k, _)| k.eq_ignore_ascii_case("content-length"))
                                .map(|(_, v)| v.trim().parse().unwrap())
                        })
                        .unwrap();
                    if bytes.len() >= end + 4 + length {
                        break serde_json::from_slice(&bytes[end + 4..end + 4 + length]).unwrap();
                    }
                }
            };
            let result = match request["method"].as_str().unwrap() {
                "getGenesisHash" => json!("EtWTRABZaYq6iMfeYKouRu166VU2xqa1wcaWoxPkrZBG"),
                "getBalance" => {
                    json!({"context":{"slot":1},"value":if request["params"][0] == wallet {1_000_000_000u64}else{0}})
                }
                "getProgramAccounts" => {
                    let bytes = &request["params"][1]["filters"][1]["memcmp"]["bytes"];
                    let owns = *bytes == wallet || *bytes == json!(owner_bytes.to_vec());
                    json!(if core_fixture
                        && request["params"][0] == core_program
                        && owns
                        && !closed.contains(&core_address)
                    {
                        vec![json!({"pubkey":core_address,"account":core_value})]
                    } else {
                        vec![]
                    })
                }
                "getAssetsByOwner" => json!({"total":0,"items":[]}),
                "getTokenAccountsByOwner" => {
                    json!({"context":{"slot":1},"value":if request["params"][0] == wallet && request["params"][1]["programId"]==TokenProgram::Legacy.id().to_string(){account_list.iter().filter(|a| !closed.contains(*a)).map(|a|json!({"pubkey":a,"account":if burned.contains(a) { &empty_token } else { &token }})).collect::<Vec<_>>()}else{vec![]}})
                }
                "getMultipleAccounts" => {
                    json!({"context":{"slot":1},"value":request["params"][0].as_array().unwrap().iter().map(|key|if key==&mint_address {mint_value.clone()}else{Value::Null}).collect::<Vec<_>>()})
                }
                "getAccountInfo" => {
                    json!({"context":{"slot":1},"value":if core_fixture && request["params"][0] == core_address && !closed.contains(&core_address) {core_value.clone()}else if account_list.iter().any(|a|request["params"][0]==*a && !closed.contains(a)){if burned.contains(request["params"][0].as_str().unwrap()) { empty_token.clone() } else { token.clone() }}else{Value::Null}})
                }
                "getLatestBlockhash" => {
                    json!({"context":{"slot":1},"value":{"blockhash":"11111111111111111111111111111111","lastValidBlockHeight":1000}})
                }
                "getBlockHeight" => json!(10),
                "getVersion" => json!({"solana-core":"3.0.0","feature-set":0}),
                "simulateTransaction" => {
                    assert_eq!(request["params"][1]["sigVerify"], true);
                    json!({"context":{"slot":1},"value":{"err":null,"logs":[],"unitsConsumed":3000}})
                }
                "sendTransaction" => {
                    assert_eq!(request["params"][1]["skipPreflight"], false);
                    let transaction: VersionedTransaction = bincode::deserialize(
                        &STANDARD
                            .decode(request["params"][0].as_str().unwrap())
                            .unwrap(),
                    )
                    .unwrap();
                    let keys = transaction.message.static_account_keys();
                    let instructions = transaction.message.instructions();
                    assert_eq!(instructions.len(), 2);
                    let close = &instructions[1];
                    let is_core =
                        keys[usize::from(close.program_id_index)].to_string() == core_program;
                    if is_core {
                        assert!(core_fixture);
                        let source = keys[usize::from(close.accounts[0])].to_string();
                        assert_eq!(source, core_address);
                        assert!(closed.insert(source.clone()));
                        let signature = transaction.signatures[0].to_string();
                        sent.lock().unwrap().push(signature.clone());
                        signatures.insert(signature.clone(), (source, false));
                        let response =
                            json!({"jsonrpc":"2.0","id":request["id"],"result":signature})
                                .to_string();
                        write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{response}",response.len()).unwrap();
                        continue;
                    }
                    let is_burn = close.data.first() == Some(&15);
                    if is_burn {
                        assert!(explicit);
                        assert_eq!(u64::from_le_bytes(close.data[1..9].try_into().unwrap()), 20);
                        assert_eq!(close.data[9], 6);
                    } else {
                        assert_eq!(close.data, vec![9]);
                    }
                    assert_eq!(
                        keys[usize::from(close.program_id_index)].to_string(),
                        TokenProgram::Legacy.id().to_string()
                    );
                    assert_eq!(
                        keys[usize::from(close.accounts[1])].to_string(),
                        if is_burn {
                            mint_address.clone()
                        } else {
                            wallet.clone()
                        }
                    );
                    assert_eq!(keys[usize::from(close.accounts[2])].to_string(), wallet);
                    let source = keys[usize::from(close.accounts[0])].to_string();
                    if is_burn {
                        assert!(burned.insert(source.clone()));
                    } else {
                        if explicit {
                            assert!(burned.contains(&source));
                        }
                        assert!(closed.insert(source.clone()));
                    }
                    let signature = transaction.signatures[0].to_string();
                    sent.lock().unwrap().push(signature.clone());
                    signatures.insert(signature.clone(), (source, is_burn));
                    json!(signature)
                }
                "getSignatureStatuses" => {
                    json!({"context":{"slot":1},"value":request["params"][0].as_array().unwrap().iter().map(|_|json!({"slot":1,"confirmations":null,"err":null,"status":{"Ok":null},"confirmationStatus":"confirmed"})).collect::<Vec<_>>()})
                }
                "getTransaction" => {
                    let (source, is_burn) = &signatures[request["params"][0].as_str().unwrap()];
                    json!({"transaction":{"message":{"accountKeys":[wallet,source]}},"meta":{"err":null,"preBalances":[9007199254740993123u64,2039280],"postBalances":[if *is_burn {9007199254740988123u64} else {9007199254743027403u64},if *is_burn {2039280} else {0}]}})
                }
                method => panic!("Unexpected RPC {method}"),
            };
            let body = json!({"jsonrpc":"2.0","id":request["id"],"result":result}).to_string();
            write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).unwrap();
        }
    });
    let directory =
        std::env::temp_dir().join(format!("dock-ipc-{}", solana_pubkey::Pubkey::new_unique()));
    let mut state = AppState::new(url.clone(), 5, None)
        .unwrap()
        .with_journal(directory.join("signatures.json"))
        .unwrap();
    if core_fixture {
        state = state.with_das(url).unwrap();
    }
    let app = desktop(state);
    let main = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();
    let connection = invoke(
        &main,
        "connect_wallet",
        json!({"source":{"kind":"publicKey","address":owner}}),
        LOCAL_ORIGIN,
    )
    .unwrap();
    let readonly=invoke(&main,"prepare_cleanup",json!({"sessionId":connection["sessionId"],"revision":1,"selection":{"mode":"all"},"ignoredMints":[]}),LOCAL_ORIGIN).unwrap();
    assert_eq!(readonly["closeCount"], 2);
    assert_eq!(readonly["canExecute"], false);
    assert!(invoke(&main,"execute_cleanup",json!({"sessionId":connection["sessionId"],"planId":readonly["planId"],"approval":{"swap":true,"burn":true,"close":true}}),LOCAL_ORIGIN).is_err());
    let connection = invoke(
        &main,
        "connect_wallet",
        json!({"source":{"kind":"seed","base64":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA="}}),
        LOCAL_ORIGIN,
    )
    .unwrap();
    if core_fixture {
        if core_mode == 2 {
            let plan=invoke(&main,"prepare_cleanup",json!({"sessionId":connection["sessionId"],"revision":1,"selection":{"mode":"selected","assetIds":[core_id]},"ignoredMints":[]}),LOCAL_ORIGIN).unwrap();
            assert_eq!(plan["selectedAssets"], 1);
            assert_eq!(plan["canExecute"], false);
            assert_eq!(plan["undecodableAccounts"][0]["address"], core_id);
            assert_eq!(plan["undecodableAccounts"][0]["reasonCode"], "undecodable");
            assert_eq!(plan["closeCount"], 0);
            assert!(sends.lock().unwrap().is_empty());
            stop.store(true, Ordering::Relaxed);
            server.join().unwrap();
            let _ = std::fs::remove_dir_all(directory);
            return;
        }
        let protected = invoke(&main,"prepare_cleanup",json!({"sessionId":connection["sessionId"],"revision":1,"selection":{"mode":"selected","assetIds":[core_id]},"ignoredMints":[],"ignoredAssetIds":[core_id]}),LOCAL_ORIGIN).unwrap();
        assert_eq!(protected["canExecute"], false);
        let plan = invoke(&main,"prepare_cleanup",json!({"sessionId":connection["sessionId"],"revision":2,"selection":{"mode":"selected","assetIds":[core_id]},"ignoredMints":[]}),LOCAL_ORIGIN).unwrap();
        assert_eq!(plan["burnCount"], 1, "{plan}");
        assert_eq!(plan["closeCount"], 0);
        assert_eq!(plan["canExecute"], true);
        assert_eq!(plan["executableAccounts"], 1);
        assert_eq!(plan["selectedAssets"], 1);
        assert!(
            plan["entries"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|e| e["tokenAccount"].is_string())
                .all(|e| e["action"] == "skip")
        );
        let execute = json!({"sessionId":connection["sessionId"],"planId":plan["planId"],"approval":{"swap":false,"burn":true,"close":false}});
        let job = invoke(&main, "execute_cleanup", execute.clone(), LOCAL_ORIGIN).unwrap();
        assert_eq!(job["status"], "completed");
        assert_eq!(job["report"]["completed"], 1);
        assert_eq!(job["report"]["closed"], 0);
        assert_eq!(job["report"]["known_net_wallet_lamports"], "2034280");
        assert_eq!(
            invoke(&main, "execute_cleanup", execute, LOCAL_ORIGIN).unwrap()["jobId"],
            job["jobId"]
        );
        assert_eq!(sends.lock().unwrap().len(), 1);
        let analysis=invoke(&main,"analyze_wallet",json!({"walletAddress":owner,"selection":{"all":true,"allTokens":true},"noPrices":true}),LOCAL_ORIGIN).unwrap();
        assert_eq!(
            analysis["tokenAccounts"].as_array().unwrap().len(),
            2,
            "Core-only selection preserves both SPL accounts"
        );
        assert!(
            analysis["nfts"]["core"]["items"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        stop.store(true, Ordering::Relaxed);
        server.join().unwrap();
        let _ = std::fs::remove_dir_all(directory);
        return;
    }
    for (revision, selection, ignored) in [
        (1, json!({"mode":"none"}), json!([])),
        (2, json!({"mode":"selected","mints":[]}), json!([])),
        (3, json!({"mode":"all"}), json!([mint])),
    ] {
        let plan=invoke(&main,"prepare_cleanup",json!({"sessionId":connection["sessionId"],"revision":revision,"selection":selection,"ignoredMints":ignored}),LOCAL_ORIGIN).unwrap();
        assert_eq!(plan["closeCount"], 0);
        assert_eq!(plan["canExecute"], false);
        assert!(invoke(&main,"execute_cleanup",json!({"sessionId":connection["sessionId"],"planId":plan["planId"],"approval":{"swap":true,"burn":true,"close":true}}),LOCAL_ORIGIN).is_err());
    }
    assert!(sends.lock().unwrap().is_empty());
    let plan=invoke(&main,"prepare_cleanup",json!({"sessionId":connection["sessionId"],"revision":4,"selection":{"mode":"selected","assetIds":[format!("{}:{}",mint,TokenProgram::Legacy.id())]},"ignoredMints":[]}),LOCAL_ORIGIN).unwrap();
    if explicit {
        assert_eq!(plan["burnCount"], 2);
        assert_eq!(plan["executableAccounts"], 2);
        assert_eq!(plan["selectedAssets"], 1);
        assert!(
            plan["entries"]
                .as_array()
                .unwrap()
                .iter()
                .all(|e| e["rawAmount"] == "20" && e["reasonCode"] == "routing_unavailable")
        );
        assert!(invoke(&main,"execute_cleanup",json!({"sessionId":connection["sessionId"],"planId":plan["planId"],"approval":{"swap":true,"burn":false,"close":true}}),LOCAL_ORIGIN).is_err());
        assert!(sends.lock().unwrap().is_empty());
    }
    assert_eq!(plan["entries"].as_array().unwrap().len(), 2);
    assert_eq!(plan["closeCount"], 2);
    let execution = json!({"sessionId":connection["sessionId"],"planId":plan["planId"],"approval":{"swap":true,"burn":true,"close":true}});
    let job = invoke(&main, "execute_cleanup", execution.clone(), LOCAL_ORIGIN).unwrap();
    assert_eq!(job["status"], "completed");
    assert_eq!(job["report"]["closed"], 2);
    assert_eq!(
        job["report"]["known_net_wallet_lamports"],
        if explicit { "4058560" } else { "4068560" }
    );
    assert_eq!(job["report"]["known_reclaimed_lamports"], "4078560");
    assert_eq!(job["report"]["accounting_complete"], true);
    assert!(
        job["progress"]
            .as_array()
            .unwrap()
            .iter()
            .any(|event| event["stage"] == "confirmation")
    );
    let duplicate = invoke(&main, "execute_cleanup", execution, LOCAL_ORIGIN).unwrap();
    assert_eq!(duplicate["jobId"], job["jobId"]);
    assert_eq!(sends.lock().unwrap().len(), if explicit { 4 } else { 2 });
    assert_eq!(
        invoke(
            &main,
            "get_cleanup_job",
            json!({"sessionId":connection["sessionId"],"jobId":job["jobId"]}),
            LOCAL_ORIGIN
        )
        .unwrap()["report"],
        job["report"]
    );
    stop.store(true, Ordering::Relaxed);
    server.join().unwrap();
    let journal = std::fs::read_to_string(directory.join("signatures.json")).unwrap();
    assert!(!journal.contains("base64"));
    assert!(!journal.contains("seed"));
    std::fs::remove_dir_all(directory).unwrap();
}

/// Opt-in public RPC audit through registered Tauri commands; never obtains a signer.
#[test]
#[ignore = "Requires live devnet RPC; public scan/plan only"]
fn live_devnet_readonly_report() {
    let path = std::env::var("DOCK_FLINTS_LIVE_READONLY_REPORT")
        .expect("Set an output path for the public diagnostic report");
    let app = desktop(AppState::new("https://api.devnet.solana.com".into(), 30, None).unwrap());
    let main = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();
    let owner = "9FCR2PU1jZgCHyjWxzk2BNQHJxszAK24vBFiWmUyRNpv";
    let connection = invoke(
        &main,
        "connect_wallet",
        json!({"source":{"kind":"publicKey","address":owner}}),
        LOCAL_ORIGIN,
    )
    .unwrap();
    let analysis = invoke(
        &main,
        "analyze_wallet",
        json!({"walletAddress":owner,"selection":{"all":true,"allTokens":true},"noPrices":false}),
        LOCAL_ORIGIN,
    )
    .unwrap();
    let plan = invoke(&main, "prepare_cleanup", json!({"sessionId":connection["sessionId"],"revision":1,"selection":{"mode":"all"},"ignoredMints":[]}), LOCAL_ORIGIN).unwrap();
    assert_eq!(connection["canSign"], false);
    assert_eq!(plan["canExecute"], false);
    assert_eq!(
        plan["network"],
        "EtWTRABZaYq6iMfeYKouRu166VU2xqa1wcaWoxPkrZBG"
    );
    let accounts: std::collections::BTreeSet<_> = analysis["tokenAccounts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["address"].as_str().unwrap())
        .collect();
    let entries: std::collections::BTreeSet<_> = plan["entries"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|a| a["tokenAccount"].as_str())
        .collect();
    assert_eq!(
        entries, accounts,
        "Every decoded account must have a plan entry"
    );
    std::fs::write(
        path,
        serde_json::to_string_pretty(
            &json!({"wallet":owner,"analysis":analysis,"plan":plan,"transactionsSubmitted":0}),
        )
        .unwrap(),
    )
    .unwrap();
}
