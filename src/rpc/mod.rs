pub mod swap;
use serde_json::json;
use solana_account::Account;
use solana_account_decoder::{UiAccount, UiAccountEncoding};
use solana_client::{
    nonblocking::rpc_client::RpcClient,
    rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig},
    rpc_filter::{Memcmp, RpcFilterType},
    rpc_request::RpcRequest,
    rpc_response::{Response, RpcKeyedAccount},
};
use solana_pubkey::Pubkey;
use std::collections::BTreeMap;

pub fn to_rpc(key: solana_pubkey_v2::Pubkey) -> Pubkey {
    // Token Metadata 5.1.1 uses Pubkey v2; the RPC client uses v4.
    // Core uses another version and converts its program ID directly by bytes.
    Pubkey::new_from_array(key.to_bytes())
}
pub fn to_metaplex(key: Pubkey) -> solana_pubkey_v2::Pubkey {
    solana_pubkey_v2::Pubkey::new_from_array(key.to_bytes())
}

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
        .await?;
    Ok(response.value)
}

pub fn memcmp(offset: usize, bytes: Vec<u8>) -> RpcFilterType {
    RpcFilterType::Memcmp(Memcmp::new_raw_bytes(offset, bytes))
}

pub async fn program_accounts(
    rpc: &RpcClient,
    program: &Pubkey,
    filters: Vec<RpcFilterType>,
) -> anyhow::Result<Vec<(Pubkey, UiAccount)>> {
    Ok(rpc
        .get_program_ui_accounts_with_config(
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
        .await?)
}

pub type AccountBatch = BTreeMap<Pubkey, Result<Option<Account>, String>>;

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
            Err(error) => output.extend(chunk.iter().map(|key| (*key, Err(error.to_string())))),
        }
    }
    output
}

/// Shared confirmed RPC configuration for scanners and transaction execution.
pub fn client(
    url: String,
    timeout_seconds: u64,
) -> solana_client::nonblocking::rpc_client::RpcClient {
    solana_client::nonblocking::rpc_client::RpcClient::new_with_timeout_and_commitment(
        url,
        std::time::Duration::from_secs(timeout_seconds),
        solana_commitment_config::CommitmentConfig::confirmed(),
    )
}
