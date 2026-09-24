use crate::{models::*, rpc};
use solana_account_decoder::parse_nonce::{UiNonceState, parse_nonce};
use solana_client::{nonblocking::rpc_client::RpcClient, rpc_filter::RpcFilterType};
use solana_pubkey::Pubkey;

pub async fn get_nonce_accounts(
    rpc: &RpcClient,
    wallet: &Pubkey,
) -> ScanCollection<AssociatedAccount> {
    // Versioned nonce: u32 version + u32 state tag + 32-byte authority.
    // Unlike arbitrary PDA relationships this authority has a standard layout.
    let system = Pubkey::default();
    let accounts = match rpc::program_accounts(
        rpc,
        &system,
        vec![
            RpcFilterType::DataSize(80),
            rpc::memcmp(8, wallet.to_bytes().to_vec()),
        ],
    )
    .await
    {
        Ok(accounts) => accounts,
        Err(error) => return ScanCollection::failed(error.to_string()),
    };
    let mut result = ScanCollection::complete(Vec::new());
    for (address, account) in accounts {
        let parsed = account
            .data
            .decode()
            .and_then(|data| parse_nonce(&data).ok());
        if let Some(UiNonceState::Initialized(info)) = parsed
            && info.authority == wallet.to_string()
            && account.owner == system.to_string()
        {
            result.items.push(AssociatedAccount { address: address.to_string(), program_id: account.owner, lamports: account.lamports, kind: "durable_nonce".into(), details: serde_json::json!({"authority": info.authority, "blockhash": info.blockhash}) });
        } else {
            result.issue(format!("Nonce {address}: failed decoding/authority check"));
            result.unknown.push(UnknownAsset {
                address: address.to_string(),
                program_id: Some(account.owner),
                lamports: Some(account.lamports),
                reason: "Nonce candidate could not be verified".into(),
                data: serde_json::to_value(account.data).ok(),
            });
        }
    }
    result
}

pub fn generic_support() -> ScanStatus {
    ScanStatus::Partial("Durable nonce authority accounts are scanned separately. Solana account.owner is a program ID, not a wallet beneficiary; arbitrary PDAs, escrow, multisig-controlled assets and DeFi positions need program-specific adapters.".into())
}
