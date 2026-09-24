use crate::{models::*, rpc};
use anyhow::{Context, Result, ensure};
use mpl_core::{
    accounts::BaseAssetV1,
    types::{Key, UpdateAuthority},
};
use solana_account_decoder::UiAccount;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_pubkey::Pubkey;

pub fn parse_asset(address: Pubkey, account: &UiAccount, owner: &Pubkey) -> Result<CoreAsset> {
    ensure!(
        account.owner == mpl_core::ID.to_string(),
        "Incorrect Core program owner"
    );
    let data = account.data.decode().context("Cannot decode Core data")?;
    let asset = BaseAssetV1::from_bytes(&data)?;
    ensure!(
        asset.key == Key::AssetV1 && asset.owner.to_bytes() == owner.to_bytes(),
        "Core discriminator/owner mismatch"
    );
    Ok(CoreAsset {
        address: address.to_string(),
        owner: asset.owner.to_string(),
        name: asset.name,
        uri: asset.uri,
        lamports: account.lamports,
        data_len: data.len(),
        update_authority: format!("{:?}", asset.update_authority),
        collection: match &asset.update_authority {
            UpdateAuthority::Collection(address) => Some(CollectionInfo {
                address: address.to_string(),
                verified: true,
            }),
            _ => None,
        },
        // Base ownership is discoverable even with unknown future plugins. Plugin
        // authorities/collection inheritance may restrict transfer or burning.
        plugins_status: ScanStatus::Unsupported(
            "Plugin permissions and collection inheritance are not evaluated".into(),
        ),
    })
}

pub async fn get_core_assets(rpc: &RpcClient, owner: &Pubkey) -> ScanCollection<CoreAsset> {
    // AssetV1: one-byte discriminator followed by the 32-byte owner. Do not
    // constrain dataSize: strings and plugin records make the size variable.
    let accounts = match rpc::program_accounts(
        rpc,
        &Pubkey::new_from_array(mpl_core::ID.to_bytes()),
        vec![
            rpc::memcmp(0, vec![Key::AssetV1 as u8]),
            rpc::memcmp(1, owner.to_bytes().to_vec()),
        ],
    )
    .await
    {
        Ok(accounts) => accounts,
        Err(error) => return ScanCollection::failed(error.to_string()),
    };
    let mut result = ScanCollection::complete(Vec::new());
    for (address, account) in accounts {
        match parse_asset(address, &account, owner) {
            Ok(asset) => result.items.push(asset),
            Err(error) => {
                result.issue(format!("{address}: {error}"));
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
