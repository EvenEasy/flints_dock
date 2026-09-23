use tokio;
use anyhow::{Ok, Context, Result};
use solana_client::{
    nonblocking::rpc_client::RpcClient,
    rpc_request::TokenAccountsFilter,
    rpc_response::RpcKeyedAccount,
};
use solana_pubkey::Pubkey;

#[derive(Debug)]
pub struct TokenAccount {
    pub address: String,
    pub mint: String,
    pub program_id: String,
    pub amount: String,
    pub decimals: u8,
    pub lamports: u64,
}

fn parse_account(
    account: RpcKeyedAccount,
) -> Result<TokenAccount> {
    let data = match account.account.data {
        solana_account_decoder::UiAccountData::Json(data) => data,
        _ => anyhow::bail!("Unexpected account encoding"),
    };

    let info = data.parsed
        .get("info")
        .context("Missing token info")?;

    let token_amount = &info["tokenAmount"];

    Ok(TokenAccount {
        address: account.pubkey,
        program_id: account.account.owner,
        mint: info["mint"]
            .as_str()
            .context("Missing mint")?
            .to_owned(),
        amount: token_amount["amount"]
            .as_str()
            .context("Missing amount")?
            .to_owned(),
        decimals: token_amount["decimals"]
            .as_u64()
            .context("Missing decimals")? as u8,
        lamports: account.account.lamports,
    })
}

pub async fn get_account_tokens(
    rpc: &RpcClient,
    owner: &Pubkey,
) -> Result<Vec<TokenAccount>>{
    let (legacy, token_2022) = tokio::try_join!(
        rpc.get_token_accounts_by_owner(
            &owner,
            TokenAccountsFilter::ProgramId(spl_token_interface::ID)
        ),
        rpc.get_token_accounts_by_owner(
            &owner,
            TokenAccountsFilter::ProgramId(spl_token_2022_interface::ID)
        )

    )?;

    legacy
        .into_iter()
        .chain(token_2022)
        .map(parse_account)
        .collect()
}
