//! DAS pagination is read-only and deliberately returns only verified compressed holdings.
use crate::core::{ScanStatus, categories::*};
use serde_json::{Value, json};
use std::{collections::BTreeMap, time::Duration};

/// Backend-owned endpoint may carry credentials; errors never echo URLs or provider bodies.
#[derive(Clone)]
pub struct DasClient {
    http: reqwest::Client,
    endpoint: String,
}
impl DasClient {
    pub fn new(endpoint: String) -> anyhow::Result<Self> {
        let url = reqwest::Url::parse(&endpoint)?;
        anyhow::ensure!(
            matches!(url.scheme(), "http" | "https") && url.host_str().is_some(),
            "invalid DAS endpoint"
        );
        Ok(Self {
            endpoint,
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(20))
                .redirect(reqwest::redirect::Policy::none())
                .build()?,
        })
    }
    /// Ensure the configured indexer serves the same genesis hash as the Solana reader.
    pub async fn compressed_on_network(&self, owner: &str, network: &str) -> CompressedReport {
        let response = self
            .http
            .post(&self.endpoint)
            .json(&json!({"jsonrpc":"2.0","id":"network","method":"getGenesisHash"}))
            .send()
            .await;
        let same = match response {
            Ok(response) if response.status().is_success() => response
                .json::<Value>()
                .await
                .ok()
                .is_some_and(|value| value["result"].as_str() == Some(network)),
            _ => false,
        };
        if !same {
            return CompressedReport {
                items: vec![],
                status: ScanStatus::Unsupported(
                    "DAS network could not be verified against RPC genesis hash".into(),
                ),
            };
        }
        self.compressed(owner).await
    }
    /// Fetch every page with bounded retries; preserve earlier verified assets on a later failure.
    pub async fn compressed(&self, owner: &str) -> CompressedReport {
        let mut items = BTreeMap::new();
        let mut received = 0u64;
        let mut observed_ids = std::collections::BTreeSet::new();
        let mut uncertain_coverage = false;
        for page in 1..=1000 {
            let mut result = None;
            for attempt in 0..3 {
                let response = self.http.post(&self.endpoint).json(&json!({"jsonrpc":"2.0", "id":page, "method":"getAssetsByOwner", "params":{"ownerAddress":owner,"page":page,"limit":1000,"sortBy":{"sortBy":"created","sortDirection":"asc"}}})).send().await;
                match response {
                    Ok(response) if response.status().is_success() => {
                        if let Ok(value) = response.json::<Value>().await
                            && value.get("error").is_none()
                        {
                            result = Some(value["result"].clone());
                            break;
                        }
                    }
                    Ok(response)
                        if response.status().as_u16() == 429
                            || response.status().is_server_error() =>
                    {
                        let wait = response
                            .headers()
                            .get("retry-after")
                            .and_then(|v| v.to_str().ok())
                            .and_then(|v| v.parse::<u64>().ok())
                            .unwrap_or(1 << attempt);
                        if wait > 60 {
                            break;
                        }
                        tokio::time::sleep(Duration::from_secs(wait)).await;
                        continue;
                    }
                    Ok(_) => break,
                    Err(_) => {}
                }
                tokio::time::sleep(Duration::from_secs(1 << attempt)).await;
            }
            let Some(result) = result else {
                return report(items, "DAS page failed; indexed NFT inventory incomplete");
            };
            let Some(rows) = result["items"].as_array() else {
                return report(items, "Malformed DAS items");
            };
            let Some(total) = result["total"].as_u64() else {
                return report(items, "DAS total unavailable");
            };
            received += rows.len() as u64;
            let mut new_ids = 0;
            for row in rows {
                // Deduplication preserves known assets, but duplicate/malformed rows cannot prove full coverage.
                match row["id"].as_str() {
                    Some(id) => {
                        if !observed_ids.insert(id.to_owned()) {
                            uncertain_coverage = true;
                        }
                    }
                    None => uncertain_coverage = true,
                }
                if row["compression"]["compressed"].as_bool().is_none()
                    || row["ownership"]["owner"].as_str().is_none()
                    || row["burnt"].as_bool().is_none()
                    || (row["compression"]["compressed"] == true
                        && row["id"]
                            .as_str()
                            .is_none_or(|id| id.parse::<solana_pubkey::Pubkey>().is_err()))
                {
                    uncertain_coverage = true;
                }
                if let Some(asset) = parse_asset(row, owner)
                    && !items.contains_key(&asset.id)
                {
                    new_ids += 1;
                    items.insert(asset.id.clone(), asset);
                }
            }
            if received >= total {
                return CompressedReport {
                    items: items.into_values().collect(),
                    status: if uncertain_coverage {
                        ScanStatus::Partial("DAS duplicate or malformed rows; indexed coverage cannot be proved complete".into())
                    } else {
                        ScanStatus::Complete
                    },
                };
            }
            if rows.is_empty() {
                return report(items, "DAS ended before declared total");
            }
            // Only compare raw page IDs: a full page may legitimately contain no compressed assets.
            if page > 1
                && new_ids == 0
                && rows.iter().all(|r| r["compression"]["compressed"] == true)
            {
                return report(items, "DAS repeated page; inventory incomplete");
            }
        }
        report(items, "DAS pagination safety bound reached")
    }
}
fn report(items: BTreeMap<String, CompressedAsset>, reason: &str) -> CompressedReport {
    CompressedReport {
        status: if items.is_empty() {
            ScanStatus::Failed(reason.into())
        } else {
            ScanStatus::Partial(reason.into())
        },
        items: items.into_values().collect(),
    }
}
/// Ignore burned, foreign-owner, noncompressed and malformed indexed records.
pub fn parse_asset(value: &Value, owner: &str) -> Option<CompressedAsset> {
    if value["ownership"]["owner"].as_str()? != owner
        || value["burnt"].as_bool()?
        || !value["compression"]["compressed"].as_bool()?
    {
        return None;
    }
    let id = value["id"].as_str()?;
    id.parse::<solana_pubkey::Pubkey>().ok()?;
    Some(CompressedAsset {
        id: id.into(),
        owner: owner.into(),
        name: value["content"]["metadata"]["name"]
            .as_str()
            .unwrap_or(id)
            .into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{Read, Write},
        net::TcpListener,
    };
    fn row(n: u8, owner: &str) -> Value {
        json!({"id":solana_pubkey::Pubkey::new_from_array([n;32]).to_string(),"ownership":{"owner":owner},"compression":{"compressed":true},"burnt":false,"content":{"metadata":{"name":"fixture"}}})
    }
    fn server(responses: Vec<Value>) -> (String, std::thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        (
            url,
            std::thread::spawn(move || {
                for (index, result) in responses.into_iter().enumerate() {
                    let (mut stream, _) = listener.accept().unwrap();
                    stream
                        .set_read_timeout(Some(Duration::from_secs(5)))
                        .unwrap();
                    let mut bytes = Vec::new();
                    let mut buffer = [0; 4096];
                    loop {
                        let count = stream.read(&mut buffer).unwrap();
                        assert!(count > 0);
                        bytes.extend_from_slice(&buffer[..count]);
                        if let Some(end) = bytes.windows(4).position(|v| v == b"\r\n\r\n") {
                            let head = String::from_utf8_lossy(&bytes[..end]);
                            let length: usize = head
                                .lines()
                                .find_map(|line| {
                                    line.split_once(':')
                                        .filter(|(k, _)| k.eq_ignore_ascii_case("content-length"))
                                        .map(|(_, v)| v.trim().parse().unwrap())
                                })
                                .unwrap();
                            if bytes.len() >= end + 4 + length {
                                let req: Value =
                                    serde_json::from_slice(&bytes[end + 4..end + 4 + length])
                                        .unwrap();
                                assert_eq!(req["params"]["page"], index + 1);
                                break;
                            }
                        }
                    }
                    let body = json!({"jsonrpc":"2.0","id":index+1,"result":result}).to_string();
                    write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).unwrap();
                }
            }),
        )
    }
    #[tokio::test]
    async fn pagination_deduplicates_ids_checks_owner_burn_and_compression() {
        let owner = "wallet";
        let first = row(1, owner);
        let second = row(2, owner);
        let mut burned = row(3, owner);
        burned["burnt"] = true.into();
        let foreign = row(4, "other");
        let mut classic = row(5, owner);
        classic["compression"]["compressed"] = false.into();
        let (url, server) = server(vec![
            json!({"total":6,"items":[first.clone(), burned,foreign]}),
            json!({"total":6,"items":[first,second,classic]}),
        ]);
        let result = DasClient::new(url).unwrap().compressed(owner).await;
        server.join().unwrap();
        assert_eq!(result.items.len(), 2);
        assert!(matches!(result.status, ScanStatus::Partial(_)));
    }
    #[tokio::test]
    async fn later_malformed_page_preserves_partial_inventory() {
        let (url, server) = server(vec![
            json!({"total":2,"items":[row(1,"wallet")]}),
            json!({"total":2,"items":"malformed"}),
        ]);
        let result = DasClient::new(url).unwrap().compressed("wallet").await;
        server.join().unwrap();
        assert_eq!(result.items.len(), 1);
        assert!(matches!(result.status, ScanStatus::Partial(_)));
    }
}
