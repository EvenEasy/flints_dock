use super::*;
use crate::{
    app::categories::RiskProvider,
    app::pricing::PriceProvider,
    app::swap::{SwapError, SwapProvider, SwapRequest},
};
use serde_json::json;
use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::{Arc, Mutex},
    thread,
};

fn server(
    responses: Vec<(u16, String)>,
) -> (Jupiter, Arc<Mutex<Vec<String>>>, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let calls = Arc::new(Mutex::new(Vec::new()));
    let captured = calls.clone();
    let handle = thread::spawn(move || {
        for (status, body) in responses {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut request = Vec::new();
            let mut buf = [0; 1024];
            while !request.windows(4).any(|part| part == b"\r\n\r\n") {
                let count = stream.read(&mut buf).unwrap();
                assert!(count > 0);
                request.extend_from_slice(&buf[..count]);
            }
            captured
                .lock()
                .unwrap()
                .push(String::from_utf8(request).unwrap());
            write!(stream,"HTTP/1.1 {status} Fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).unwrap();
        }
    });
    (
        Jupiter::with_base_url("fixture-key".into(), url).unwrap(),
        calls,
        handle,
    )
}
fn request() -> SwapRequest {
    SwapRequest {
        mint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"
            .parse()
            .unwrap(),
        wallet: solana_pubkey::Pubkey::new_from_array([7; 32]),
        raw_amount: 1_000_000,
        slippage_bps: 50,
    }
}
#[tokio::test]
async fn unpriced_token_can_have_a_route_and_shared_client_sends_exact_v2_parameters() {
    let (client, calls, server) = server(vec![
        (200, "{}".into()),
        (
            200,
            include_str!("../../../../../tests/fixtures/jupiter_build.json").into(),
        ),
    ]);
    let req = request();
    let prices = client.get_prices(&[req.mint.to_string()]).await;
    assert!(prices.quotes.is_empty());
    let built = client.build_swap(&req).await.unwrap();
    assert!(built.quote.route_exists);
    server.join().unwrap();
    let calls = calls.lock().unwrap();
    assert_eq!(calls.len(), 2);
    assert!(calls[0].starts_with("GET /price/v3?"));
    assert!(calls[1].starts_with("GET /swap/v2/build?"));
    for part in [
        format!("inputMint={}", req.mint),
        format!("outputMint={WRAPPED_SOL}"),
        "amount=1000000".into(),
        format!("taker={}", req.wallet),
        format!("nativeDestinationAccount={}", req.wallet),
        "wrapAndUnwrapSol=true".into(),
        "slippageBps=50".into(),
        "x-api-key: fixture-key".into(),
    ] {
        assert!(calls[1].contains(&part), "Missing {part}");
    }
    assert!(!calls[1].contains("destinationTokenAccount="));
}
#[tokio::test]
async fn http_no_route_and_rate_limit_are_not_successful_quotes() {
    for (status, body, no_route) in [
        (400, json!({"error":"No routes found"}), true),
        (429, json!({"error":"rate limited"}), false),
    ] {
        let (client, calls, server) = server(vec![(status, body.to_string())]);
        let result = client.build_swap(&request()).await;
        assert!(if no_route {
            matches!(result, Err(SwapError::NoRoute(_)))
        } else {
            matches!(result, Err(SwapError::Api(_)))
        });
        server.join().unwrap();
        assert_eq!(calls.lock().unwrap().len(), 1);
    }
}

#[tokio::test]
async fn keyless_swap_requests_omit_the_authentication_header() {
    let (mut client, calls, server) = server(vec![(
        200,
        include_str!("../../../../../tests/fixtures/jupiter_build.json").into(),
    )]);
    client.api_key = reqwest::header::HeaderValue::from_static("");
    assert!(
        client
            .build_swap(&request())
            .await
            .unwrap()
            .quote
            .route_exists
    );
    server.join().unwrap();
    assert!(!calls.lock().unwrap()[0].contains("x-api-key:"));
}

#[tokio::test]
async fn non_json_gateway_response_retries_before_parsing_and_preserves_real_price() {
    let mint = request().mint.to_string();
    let (client, calls, server) = server(vec![
        (503, "temporary upstream outage".into()),
        (
            200,
            json!({mint.clone():{"usdPrice":1.0,"decimals":6,"blockId":123}}).to_string(),
        ),
    ]);
    let report = client.get_prices(&[mint.clone()]).await;
    server.join().unwrap();
    assert_eq!(calls.lock().unwrap().len(), 2);
    assert_eq!(report.status, crate::core::ScanStatus::Complete);
    assert_eq!(report.quotes[&mint].usd, 1.0);
}

#[tokio::test]
async fn risk_auth_failure_is_failed_and_does_not_become_unknown_partial_success() {
    let (client, calls, server) =
        server(vec![(401, json!({"error":"invalid API key"}).to_string())]);
    let (signals, status) = client.risks(&[request().mint.to_string()]).await;
    server.join().unwrap();
    assert_eq!(calls.lock().unwrap().len(), 1);
    assert!(signals.is_empty());
    assert!(
        matches!(status, crate::core::ScanStatus::Failed(reason) if reason.contains("HTTP 401"))
    );
}

#[tokio::test]
async fn later_risk_batch_failure_keeps_confirmed_signals_and_concrete_reason() {
    let mints = (0..101)
        .map(|index| format!("mint{index:03}"))
        .collect::<Vec<_>>();
    let (client, _, server) = server(vec![
        (
            200,
            json!([{"id":"mint000","audit":{"isSus":true}}]).to_string(),
        ),
        (403, json!({"error":"forbidden"}).to_string()),
    ]);
    let (signals, status) = client.risks(&mints).await;
    server.join().unwrap();
    assert_eq!(signals["mint000"].status, "suspicious");
    assert!(
        matches!(status, crate::core::ScanStatus::Partial(reason) if reason.contains("HTTP 403"))
    );
}
