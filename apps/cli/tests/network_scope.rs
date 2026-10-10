//! Test the CLI boundary: unsupported/unverified clusters must not initialize mainnet pricing.
use serde_json::{Value, json};
use std::{
    io::{BufRead, BufReader, Read, Write},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

#[test]
fn devnet_and_failed_genesis_keep_readonly_balance_without_mainnet_prices() {
    for verified in [true, false] {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        listener.set_nonblocking(true).unwrap();
        let done = Arc::new(AtomicBool::new(false));
        let stop = done.clone();
        let methods = Arc::new(Mutex::new(Vec::new()));
        let requests = methods.clone();
        let server = std::thread::spawn(move || {
            while !stop.load(Ordering::Relaxed) {
                let (mut stream, _) = match listener.accept() {
                    Ok(value) => value,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(5));
                        continue;
                    }
                    Err(error) => panic!("{error}"),
                };
                stream
                    .set_read_timeout(Some(Duration::from_secs(3)))
                    .unwrap();
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut length = 0;
                loop {
                    let mut line = String::new();
                    reader.read_line(&mut line).unwrap();
                    if line == "\r\n" {
                        break;
                    }
                    if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                        length = value.trim().parse().unwrap();
                    }
                }
                let mut body = vec![0; length];
                reader.read_exact(&mut body).unwrap();
                let request: Value = serde_json::from_slice(&body).unwrap();
                let method = request["method"].as_str().unwrap();
                requests.lock().unwrap().push(method.to_owned());
                let response = match method {
                    "getGenesisHash" if verified => {
                        json!({"result":dock_flints::app::test_observations::DEVNET_GENESIS})
                    }
                    "getGenesisHash" => {
                        json!({"error":{"code":-32601,"message":"Genesis unavailable"}})
                    }
                    "getBalance" => json!({"result":{"context":{"slot":1},"value":42}}),
                    _ => panic!("Unexpected RPC request {method}"),
                };
                let mut response = response;
                response["jsonrpc"] = json!("2.0");
                response["id"] = request["id"].clone();
                let body = response.to_string();
                write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
            }
        });
        let output = std::process::Command::new(env!("CARGO_BIN_EXE_dock_flints"))
            .args([
                "scan",
                "--pubkey",
                "11111111111111111111111111111111",
                "--balance",
                "--rpc-url",
                &url,
                "--format",
                "json",
            ])
            // Initialization would reject this header if the mainnet provider were constructed.
            .env("JUPITER_API_KEY", "invalid\nheader")
            .output()
            .unwrap();
        done.store(true, Ordering::Relaxed);
        server.join().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let report: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(report["sol"]["lamports"], 42);
        assert_eq!(report["sol"]["status"], "complete");
        assert_eq!(report["sol"]["pricing"]["status"], "unsupported");
        assert!(report["sol"]["price"].is_null());
        assert!(report["sol"]["usd_value"].is_null());
        assert_eq!(*methods.lock().unwrap(), ["getGenesisHash", "getBalance"]);
    }
}
