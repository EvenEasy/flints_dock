use crate::{core::*, infra::solana as rpc};
use anyhow::{Context, Result, ensure};
use solana_account_decoder::{
    parse_account_data::SplTokenAdditionalDataV2, parse_token::parse_token_v3,
};
use solana_pubkey::Pubkey;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_rpc_client_api::response::RpcKeyedAccount;
use spl_token_2022_interface::{
    extension::{BaseStateWithExtensions, StateWithExtensions},
    state::Account,
};

pub use crate::core::asset::assess_closure;

/// Return raw token state with authorities, extensions and conditional closure eligibility.
/// Validates the selected program and wallet owner; mint decimals remain unset until enrichment.
pub fn parse_account(
    account: &RpcKeyedAccount,
    program: TokenProgram,
    wallet: &Pubkey,
) -> Result<TokenAccount> {
    let _: Pubkey = account.pubkey.parse()?;
    ensure!(
        account.account.owner == program.id().to_string(),
        "Wrong token program owner"
    );
    let data = account
        .account
        .data
        .decode()
        .context("Cannot decode token account")?;

    // StateWithExtensions also accepts the legacy 165-byte base account. Never
    // impose that length on Token-2022 accounts: their TLV tail is variable.
    let state = StateWithExtensions::<Account>::unpack(&data)?;
    ensure!(
        state.base.owner.to_bytes() == wallet.to_bytes(),
        "Token authority mismatch"
    );
    let extension_types = state
        .get_extension_types()?
        .iter()
        .map(|kind| format!("{kind:?}"))
        .collect();

    // Decimals are unknown until the batched mint lookup. This decoder call is
    // used only for extension details; its UI amount is deliberately discarded.
    let decoded = serde_json::to_value(parse_token_v3(
        &data,
        Some(&SplTokenAdditionalDataV2::with_decimals(0)),
    )?)?;
    let base = state.base;
    let mut result = TokenAccount {
        address: account.pubkey.clone(),
        mint: base.mint.to_string(),
        program,
        program_id: account.account.owner.clone(),
        owner: base.owner.to_string(),
        raw_amount: base.amount,
        decimals: None,
        lamports: account.account.lamports,
        data_len: data.len(),
        state: format!("{:?}", base.state),
        delegate: Option::<Pubkey>::from(base.delegate).map(|key| key.to_string()),
        delegated_amount: base.delegated_amount,
        close_authority: Option::<Pubkey>::from(base.close_authority).map(|key| key.to_string()),
        is_native: base.is_native(),
        native_reserve_lamports: base.is_native.into(),
        extensions: decoded["info"]["extensions"].clone(),
        extension_types,
        closure: ClosureAssessment::NeedsReview("Not assessed".into()),
    };
    result.closure = assess_closure(&result, &wallet.to_string());
    Ok(result)
}

/// Return independently decoded accounts from the selected token program.
/// Unreadable accounts remain in `unknown`; confidential balances are marked incomplete, not
/// decrypted.
pub async fn get_token_accounts(
    rpc: &RpcClient,
    owner: &Pubkey,
    program: TokenProgram,
) -> ScanCollection<TokenAccount> {
    let accounts = match rpc::token_accounts(rpc, owner, &program.id()).await {
        Ok(accounts) => accounts,
        Err(error) => return ScanCollection::failed(error.to_string()),
    };
    let mut result = ScanCollection::complete(Vec::new());

    // Decode accounts independently and retain failures with their original addresses.
    for account in accounts {
        match parse_account(&account, program, owner) {
            Ok(account) => {
                if account
                    .extension_types
                    .iter()
                    .any(|extension| extension.starts_with("Confidential"))
                {
                    result.issue(format!(
                        "{}: confidential balances cannot be decrypted with a public key",
                        account.address
                    ));
                }
                result.items.push(account);
            }
            Err(error) => {
                result.issue(format!("{}: {error}", account.pubkey));
                result.unknown.push(UnknownAsset {
                    address: account.pubkey,
                    program_id: Some(account.account.owner),
                    lamports: Some(account.account.lamports),
                    reason: error.to_string(),
                    data: serde_json::to_value(account.account.data).ok(),
                });
            }
        }
    }
    result
}
