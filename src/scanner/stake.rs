use crate::{models::*, rpc};
use anyhow::{Context, Result, ensure};
use solana_account_decoder::{
    UiAccount,
    parse_stake::{StakeAccountType, parse_stake},
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_pubkey::Pubkey;
use std::collections::BTreeMap;

pub const STAKE_PROGRAM: Pubkey =
    Pubkey::from_str_const("Stake11111111111111111111111111111111111111");

pub fn parse_position(
    address: Pubkey,
    account: &UiAccount,
    wallet: &Pubkey,
) -> Result<StakePosition> {
    ensure!(
        account.owner == STAKE_PROGRAM.to_string(),
        "Wrong stake program owner"
    );
    let data = account.data.decode().context("Cannot decode stake data")?;
    let state = parse_stake(&data)?;
    let (StakeAccountType::Initialized(info) | StakeAccountType::Delegated(info)) = state else {
        anyhow::bail!("No stake authorities")
    };
    let wallet = wallet.to_string();
    ensure!(
        info.meta.authorized.staker == wallet || info.meta.authorized.withdrawer == wallet,
        "Stake authority mismatch"
    );
    let delegation = info.stake.map(|stake| stake.delegation);
    Ok(StakePosition {
        address: address.to_string(), lamports: account.lamports,
        wallet_can_withdraw: info.meta.authorized.withdrawer == wallet,
        stake_authority: info.meta.authorized.staker, withdraw_authority: info.meta.authorized.withdrawer,
        delegated_lamports: delegation.as_ref().map(|stake| stake.stake.parse::<u64>()).transpose()?,
        validator_vote_account: delegation.as_ref().map(|stake| stake.voter.clone()),
        activation_epoch: delegation.as_ref().map(|stake| stake.activation_epoch.clone()),
        deactivation_epoch: delegation.as_ref().map(|stake| stake.deactivation_epoch.clone()),
        activation_status: ScanStatus::Unsupported("Effective activation requires stake history and warmup/cooldown rules; epochs are reported without guessing activation".into()),
        lockup: serde_json::to_value(info.meta.lockup)?,
    })
}

pub async fn get_stake_accounts(rpc: &RpcClient, wallet: &Pubkey) -> ScanCollection<StakePosition> {
    // StakeStateV2 bincode: u32 tag, u64 rent reserve, then Authorized's
    // staker at 12 and withdrawer at 44. Re-check decoded authority, because
    // memcmp filters alone aren't a type/ownership proof.
    let (staker, withdrawer) = tokio::join!(
        rpc::program_accounts(
            rpc,
            &STAKE_PROGRAM,
            vec![rpc::memcmp(12, wallet.to_bytes().to_vec())]
        ),
        rpc::program_accounts(
            rpc,
            &STAKE_PROGRAM,
            vec![rpc::memcmp(44, wallet.to_bytes().to_vec())]
        )
    );
    let mut result = ScanCollection::complete(Vec::new());
    let mut accounts = BTreeMap::new();
    let mut successes = 0;
    for (role, response) in [("staker", staker), ("withdrawer", withdrawer)] {
        match response {
            Ok(found) => {
                successes += 1;
                accounts.extend(found);
            }
            Err(error) => result.issue(format!("{role} discovery: {error}")),
        }
    }
    if successes == 0 {
        if let ScanStatus::Partial(reason) = result.status {
            result.status = ScanStatus::Failed(reason);
        }
        return result;
    }
    for (address, account) in accounts {
        match parse_position(address, &account, wallet) {
            Ok(position) => result.items.push(position),
            Err(error) => {
                result.issue(format!("Stake {address}: {error}"));
                result.unknown.push(UnknownAsset {
                    address: address.to_string(),
                    program_id: Some(account.owner),
                    lamports: Some(account.lamports),
                    reason: error.to_string(),
                    data: serde_json::to_value(account.data).ok(),
                });
            }
        }
    }
    result
}
