pub mod cleanup;
pub mod nft_cleanup;
pub mod scan;
pub mod swap;
pub mod transactions;
use serde_json::json;
use solana_account::Account;
use solana_account_decoder::{UiAccount, UiAccountEncoding};
use solana_pubkey::Pubkey;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_rpc_client_api::{
    config::{RpcAccountInfoConfig, RpcProgramAccountsConfig},
    filter::{Memcmp, RpcFilterType},
    request::RpcRequest,
    response::{Response, RpcKeyedAccount},
};
use std::collections::BTreeMap;

/// Convert a Metaplex SDK public key into the RPC SDK type without changing its bytes.
/// The SDK versions use distinct Rust types for the same on-chain address.
pub fn to_rpc(key: solana_pubkey_v2::Pubkey) -> Pubkey {
    Pubkey::new_from_array(key.to_bytes())
}

/// Convert an RPC SDK key into the public-key version expected by Metaplex PDA helpers.
pub fn to_metaplex(key: Pubkey) -> solana_pubkey_v2::Pubkey {
    solana_pubkey_v2::Pubkey::new_from_array(key.to_bytes())
}

/// Return raw owner token accounts for the specified legacy or Token-2022 program.
/// Uses base64 and confirmed commitment so unavailable RPC mint parsing cannot hide accounts.
/// RPC errors propagate; an empty vector means successful discovery with no matches.
pub async fn token_accounts(
    rpc: &RpcClient,
    owner: &Pubkey,
    program: &Pubkey,
) -> anyhow::Result<Vec<RpcKeyedAccount>> {
    // The convenience method forces jsonParsed. Base64 retains accounts even when
    // RPC's mint parser cannot resolve decimals or recognize newer extensions.
    // RpcClient::send still provides the shared Solana JSON-RPC transport/envelope.
    let response: Response<Vec<RpcKeyedAccount>> = rpc
        .send(
            RpcRequest::GetTokenAccountsByOwner,
            json!([
                owner.to_string(), {"programId": program.to_string()},
                {"encoding": "base64", "commitment": "confirmed"}
            ]),
        )
        .await
        .map_err(|error| anyhow::anyhow!(safe_error(error)))?;
    Ok(response.value)
}

pub fn memcmp(offset: usize, bytes: Vec<u8>) -> RpcFilterType {
    RpcFilterType::Memcmp(Memcmp::new_raw_bytes(offset, bytes))
}

/// Return base64 program accounts matching the supplied byte filters.
/// Uses the client commitment and propagates RPC failures instead of reporting an empty scan.
pub async fn program_accounts(
    rpc: &RpcClient,
    program: &Pubkey,
    filters: Vec<RpcFilterType>,
) -> anyhow::Result<Vec<(Pubkey, UiAccount)>> {
    rpc.get_program_ui_accounts_with_config(
        program,
        RpcProgramAccountsConfig {
            filters: Some(filters),
            account_config: RpcAccountInfoConfig {
                encoding: Some(UiAccountEncoding::Base64),
                ..Default::default()
            },
            ..Default::default()
        },
    )
    .await
    .map_err(|error| anyhow::anyhow!(safe_error(error)))
}

/// Keep each requested address mapped to an account, absence or retrieval error.
pub type AccountBatch = BTreeMap<Pubkey, Result<Option<Account>, String>>;

/// Return one result per unique address, fetched in batches of at most 100 accounts.
/// `Ok(None)` means absent; `Err` means unreadable. A failed batch does not erase other results.
pub async fn multiple_accounts(rpc: &RpcClient, keys: &[Pubkey]) -> AccountBatch {
    let keys: Vec<_> = keys
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    let mut output = BTreeMap::new();

    // Standard getMultipleAccounts accepts at most 100 addresses. A failed batch
    // must not discard successful batches or turn unavailable data into absence.
    for chunk in keys.chunks(100) {
        match rpc.get_multiple_accounts(chunk).await {
            Ok(accounts) if accounts.len() == chunk.len() => {
                output.extend(chunk.iter().copied().zip(accounts.into_iter().map(Ok)))
            }
            Ok(_) => output.extend(
                chunk
                    .iter()
                    .map(|key| (*key, Err("RPC returned wrong account count".into()))),
            ),
            Err(error) => output.extend(chunk.iter().map(|key| (*key, Err(safe_error(&error))))),
        }
    }
    output
}

/// Shared confirmed RPC configuration for scanners and transaction execution.
pub fn client(
    url: String,
    timeout_seconds: u64,
) -> solana_rpc_client::nonblocking::rpc_client::RpcClient {
    solana_rpc_client::nonblocking::rpc_client::RpcClient::new_with_timeout_and_commitment(
        url,
        std::time::Duration::from_secs(timeout_seconds),
        solana_commitment_config::CommitmentConfig::confirmed(),
    )
}

/// Remove endpoint URLs from transport errors, including path/query credentials, before reporting.
pub fn safe_error(reason: impl ToString) -> String {
    let mut value = reason.to_string();
    while let Some(start) = value.find("https://").or_else(|| value.find("http://")) {
        let end = value[start..]
            .char_indices()
            .find(|(_, ch)| ch.is_whitespace() || matches!(ch, '"' | '\'' | ')' | '>'))
            .map(|(i, _)| start + i)
            .unwrap_or(value.len());
        value.replace_range(start..end, "[RPC endpoint]");
    }
    value
}
#[cfg(test)]
mod credential_tests {
    #[test]
    fn rpc_errors_do_not_expose_url_credentials() {
        let error = super::safe_error(
            "error for url (https://user:secret@rpc.invalid/private-key?api-key=secret): HTTP 403",
        );
        assert!(!error.contains("secret"));
        assert!(!error.contains("private-key"));
        assert!(error.contains("HTTP 403"));
    }
}
