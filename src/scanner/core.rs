
use anyhow::{Context, Result};
use mpl_core::{
    accounts::BaseAssetV1,
    types::Key,
    ID as CORE_PROGRAM_ID,
};
use solana_account_decoder::UiAccountEncoding;
use solana_client::{
    nonblocking::rpc_client::RpcClient,
    rpc_config::{
        RpcAccountInfoConfig,
        RpcProgramAccountsConfig,
    },
    rpc_filter::{Memcmp, RpcFilterType},
};
use solana_pubkey::Pubkey;

#[derive(Debug)]
pub struct CoreNft {
    pub address: Pubkey,
    pub name: String,
    pub uri: String,
    pub lamports: u64,
}

pub async fn get_core_nfts(
    rpc: &RpcClient,
    owner: &Pubkey,
) -> Result<Vec<CoreNft>> {
    let program_id = Pubkey::new_from_array(
        CORE_PROGRAM_ID.to_bytes(),
    );

    let config = RpcProgramAccountsConfig {
        filters: Some(vec![
            // Перший байт — тип Core-акаунта.
            RpcFilterType::Memcmp(
                Memcmp::new_raw_bytes(
                    0,
                    vec![Key::AssetV1 as u8],
                ),
            ),

            // Наступні 32 байти — власник.
            RpcFilterType::Memcmp(
                Memcmp::new_raw_bytes(
                    1,
                    owner.to_bytes().to_vec(),
                ),
            ),
        ]),

        account_config: RpcAccountInfoConfig {
            encoding: Some(UiAccountEncoding::Base64),
            ..Default::default()
        },

        ..Default::default()
    };

    let accounts = rpc
        .get_program_ui_accounts_with_config(
            &program_id,
            config,
        )
        .await?;

    println!("Core candidates: {}", accounts.len());

    let mut nfts = Vec::with_capacity(accounts.len());

    for (address, account) in accounts {
        // UiAccountData -> Vec<u8>
        let data = account
            .data
            .decode()
            .with_context(|| {
                format!(
                    "Cannot decode account data: {}",
                    address
                )
            })?;

        // Сирі байти -> Metaplex Core Asset.
        let asset = match BaseAssetV1::from_bytes(&data) {
            Ok(asset) => asset,

            Err(err) => {
                eprintln!(
                    "Cannot parse Core NFT {}: {}",
                    address, err
                );

                continue;
            }
        };

        // Перевіряємо власника.
        if asset.owner.to_bytes() != owner.to_bytes() {
            continue;
        }

        nfts.push(CoreNft {
            address,
            name: asset.name,
            uri: asset.uri,
            lamports: account.lamports,
        });
    }

    Ok(nfts)
}