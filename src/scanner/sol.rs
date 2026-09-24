use anyhow::Result;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_pubkey::Pubkey;

pub async fn get_solana_balance(rpc: &RpcClient, owner: &Pubkey) -> Result<u64> {
    Ok(rpc.get_balance(owner).await?)
}
