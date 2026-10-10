//! Registered read-only Tauri IPC against a local chain fixture. No external wallet,
//! mainnet provider observations, signing or cleanup commands are used in these tests.
use base64::{Engine, engine::general_purpose::STANDARD};
use dock_flints_app::state::AppState;
use serde_json::{Value, json};
use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

const GENESIS: &str = "EtWTRABZaYq6iMfeYKouRu166VU2xqa1wcaWoxPkrZBG";

fn scan_with_genesis_failures(
    failures: usize,
    selection: Option<Value>,
    configure_das: bool,
) -> (Value, Vec<String>) {
    let owner = solana_pubkey::Pubkey::new_from_array([7; 32]);
    let core_id = solana_pubkey::Pubkey::new_from_array([9; 32]);
    let mut core = vec![1]; // AssetV1, verified owner, UpdateAuthority::None.
    core.extend(owner.to_bytes());
    core.push(0);
    for value in ["Owned Core fixture", "https://fixture.invalid/core.json"] {
        core.extend((value.len() as u32).to_le_bytes());
        core.extend(value.as_bytes());
    }
    core.push(0); // seq=None
    let core_account = json!({"lamports":2039280,"owner":"CoREENxT6tW1HoK8ypY1SxRMZTcVPm7R94rH4PZNhX7d",
        "data":[STANDARD.encode(core),"base64"],"executable":false,"rentEpoch":0});
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let stop = Arc::new(AtomicBool::new(false));
    let stopped = stop.clone();
    let methods = Arc::new(Mutex::new(Vec::new()));
    let captured = methods.clone();
    let server = std::thread::spawn(move || {
        let mut genesis_calls = 0;
        while !stopped.load(Ordering::Relaxed) {
            let (mut stream, _) = match listener.accept() {
                Ok(connection) => connection,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(2));
                    continue;
                }
                Err(error) => panic!("{error}"),
            };
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut bytes = Vec::new();
            let mut buffer = [0; 4096];
            let request: Value = loop {
                let count = stream.read(&mut buffer).unwrap();
                assert!(count > 0);
                bytes.extend_from_slice(&buffer[..count]);
                if let Some(end) = bytes.windows(4).position(|part| part == b"\r\n\r\n") {
                    let head = String::from_utf8_lossy(&bytes[..end]);
                    let length: usize = head
                        .lines()
                        .find_map(|line| {
                            line.split_once(':')
                                .filter(|(key, _)| key.eq_ignore_ascii_case("content-length"))
                                .map(|(_, value)| value.trim().parse().unwrap())
                        })
                        .unwrap();
                    if bytes.len() >= end + 4 + length {
                        break serde_json::from_slice(&bytes[end + 4..end + 4 + length]).unwrap();
                    }
                }
            };
            let method = request["method"].as_str().unwrap();
            captured.lock().unwrap().push(method.to_owned());
            let result = match method {
                "getGenesisHash" => {
                    genesis_calls += 1;
                    if genesis_calls <= failures {
                        None
                    } else {
                        Some(json!(GENESIS))
                    }
                }
                "getBalance" => Some(json!({"context":{"slot":1},"value":1234567890})),
                "getTokenAccountsByOwner" => Some(json!({"context":{"slot":1},"value":[]})),
                "getProgramAccounts" => {
                    Some(json!([{"pubkey":core_id.to_string(),"account":core_account}]))
                }
                _ => panic!("Unexpected read-only RPC {method}"),
            };
            let body = match result {
                Some(result) => json!({"jsonrpc":"2.0","id":request["id"],"result":result}),
                None => json!({"jsonrpc":"2.0","id":request["id"],"error":{"code":-32005,"message":"Temporary genesis failure"}}),
            }.to_string();
            write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).unwrap();
        }
    });
    let state = AppState::new(url.clone(), 2, None).unwrap();
    let state = if configure_das {
        state.with_das(url).unwrap()
    } else {
        state
    };
    let app = tauri::test::mock_builder()
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            dock_flints_app::commands::wallet::analyze_wallet
        ])
        .build(tauri::generate_context!())
        .unwrap();
    let window = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();
    let origin = if cfg!(windows) {
        "http://tauri.localhost"
    } else {
        "tauri://localhost"
    };
    let mut request = json!({"walletAddress":owner.to_string()});
    if let Some(selection) = selection {
        request["selection"] = selection;
    }
    let response = tauri::test::get_ipc_response(
        &window,
        tauri::webview::InvokeRequest {
            cmd: "analyze_wallet".into(),
            callback: tauri::ipc::CallbackFn(0),
            error: tauri::ipc::CallbackFn(1),
            url: origin.parse().unwrap(),
            body: tauri::ipc::InvokeBody::Json(json!({"request":request})),
            headers: Default::default(),
            invoke_key: tauri::test::INVOKE_KEY.into(),
        },
    )
    .unwrap()
    .deserialize()
    .unwrap();
    stop.store(true, Ordering::Relaxed);
    server.join().unwrap();
    let calls = methods.lock().unwrap().clone();
    (response, calls)
}

#[test]
fn transient_genesis_failure_retries_and_default_scan_preserves_owned_core_without_das() {
    let (response, calls) = scan_with_genesis_failures(2, None, false);
    assert_eq!(
        calls
            .iter()
            .filter(|method| *method == "getGenesisHash")
            .count(),
        3
    );
    assert_eq!(
        calls
            .iter()
            .filter(|method| *method == "getTokenAccountsByOwner")
            .count(),
        2
    );
    assert!(calls.iter().any(|method| method == "getProgramAccounts"));
    assert!(calls.iter().any(|method| method == "getBalance"));
    assert_eq!(
        response["selected"],
        json!({"balance":true,"tokens":true,"allTokens":false,"nfts":true,"cnfts":true})
    );
    assert_eq!(response["scanners"]["network"]["status"], "complete");
    assert_eq!(response["scanners"]["prices"]["status"], "unsupported");
    assert_eq!(response["categories"]["network"], GENESIS);
    let nft = &response["categories"]["categories"]["nft"];
    assert_eq!(nft["count"], 1);
    assert_eq!(nft["items"].as_array().unwrap().len(), 1);
    assert_eq!(nft["items"][0]["kind"], "core");
    assert_eq!(nft["status"]["status"], "partial");
    assert_eq!(
        response["nfts"]["core"]["items"][0]["address"],
        nft["items"][0]["id"]
    );
    for category in ["scam", "dust", "dead_token"] {
        assert_eq!(
            response["categories"]["categories"][category]["status"]["status"],
            "unsupported"
        );
    }
}

#[test]
fn exhausted_genesis_failure_is_failed_scope_and_does_not_discard_independent_nfts() {
    let (response, calls) = scan_with_genesis_failures(3, None, false);
    assert_eq!(
        calls
            .iter()
            .filter(|method| *method == "getGenesisHash")
            .count(),
        3
    );
    assert_eq!(response["scanners"]["network"]["status"], "failed");
    assert_eq!(response["scanners"]["prices"]["status"], "failed");
    assert_eq!(response["categories"]["network"], "unknown");
    assert_eq!(
        response["categories"]["providers"]["network"]["status"],
        "failed"
    );
    for category in ["scam", "dust", "dead_token"] {
        let category = &response["categories"]["categories"][category];
        assert_eq!(category["count"], 0);
        assert_eq!(category["status"]["status"], "failed");
        assert!(
            category["reason"]
                .as_str()
                .unwrap()
                .contains("getGenesisHash")
        );
    }
    let nft = &response["categories"]["categories"]["nft"];
    assert_eq!(nft["count"], 1);
    assert_eq!(nft["status"]["status"], "partial");
    assert_eq!(
        response["nfts"]["core"]["items"][0]["address"],
        nft["items"][0]["id"]
    );
    assert_eq!(response["balance"]["status"]["status"], "complete");
}

#[test]
fn deselected_compressed_discovery_skips_configured_das_without_hiding_core_nfts() {
    let (response, calls) = scan_with_genesis_failures(0, Some(json!({"nfts":true})), true);
    assert_eq!(
        calls
            .iter()
            .filter(|method| *method == "getGenesisHash")
            .count(),
        1
    );
    assert!(!calls.iter().any(|method| method == "getAssetsByOwner"));
    assert!(response["cnfts"].is_null());
    let nft = &response["categories"]["categories"]["nft"];
    assert_eq!(nft["count"], 1);
    assert_eq!(nft["status"]["status"], "partial");
    assert_eq!(nft["coverage"]["das"]["status"], "skipped");
    assert_eq!(
        response["nfts"]["core"]["items"][0]["address"],
        nft["items"][0]["id"]
    );
}
