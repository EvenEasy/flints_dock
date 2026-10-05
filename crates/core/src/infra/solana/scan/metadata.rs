use crate::core::*;
use anyhow::{Context, Result, ensure};
use solana_account::Account;
use solana_account_decoder::parse_token::{TokenAccountType, parse_token_v3};
use solana_pubkey::Pubkey;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use spl_token_2022_interface::{
    extension::{BaseStateWithExtensions, StateWithExtensions},
    state::Mint,
};
use std::collections::BTreeMap;

/// Return verified mint supply, decimals, authorities and embedded Token-2022 metadata.
/// Rejects wrong program owners or invalid layouts; external metadata pointers are not fetched.
pub fn parse_mint(key: &Pubkey, account: &Account, program: TokenProgram) -> Result<MintInfo> {
    ensure!(
        account.owner == program.id(),
        "Mint program does not match token account"
    );
    let state = StateWithExtensions::<Mint>::unpack(&account.data)?;
    let extension_types = state
        .get_extension_types()?
        .iter()
        .map(|kind| format!("{kind:?}"))
        .collect();
    let TokenAccountType::Mint(mint) = parse_token_v3(&account.data, None)? else {
        anyhow::bail!("Expected mint account")
    };
    let extensions = serde_json::to_value(&mint.extensions)?;
    let mut metadata = TokenMetadata::default();
    for extension in extensions.as_array().into_iter().flatten() {
        if extension["extension"] == "tokenMetadata" {
            let info = &extension["state"];
            ensure!(
                info["mint"].as_str() == Some(key.to_string().as_str()),
                "Token metadata mint mismatch"
            );
            metadata = TokenMetadata {
                name: info["name"].as_str().map(str::to_owned),
                symbol: info["symbol"].as_str().map(str::to_owned),
                uri: info["uri"].as_str().map(str::to_owned),
                source: Some("token_2022_extension".into()),
                ..Default::default()
            };
            for pair in info["additionalMetadata"].as_array().into_iter().flatten() {
                if pair[0] == "image" {
                    metadata.image_uri = pair[1].as_str().map(str::to_owned);
                }
            }
        }
    }
    Ok(MintInfo {
        mint: key.to_string(),
        program,
        decimals: mint.decimals,
        supply: mint.supply.parse().context("Invalid mint supply")?,
        mint_authority: mint.mint_authority,
        freeze_authority: mint.freeze_authority,
        metadata,
        extensions,
        extension_types,
    })
}

/// Fetch unique mint records and update account decimals only from verified matches.
/// Returns partial status for failed or invalid mints while preserving successfully decoded
/// records.
pub async fn get_mints(rpc: &RpcClient, accounts: &mut [TokenAccount]) -> ScanCollection<MintInfo> {
    let keys: BTreeMap<Pubkey, TokenProgram> = accounts
        .iter()
        .filter_map(|account| account.mint.parse().ok().map(|key| (key, account.program)))
        .collect();

    // Deduplicate mint reads while preserving absence separately from failed RPC batches.
    let fetched =
        crate::infra::solana::multiple_accounts(rpc, &keys.keys().copied().collect::<Vec<_>>())
            .await;
    let mut result = ScanCollection::complete(Vec::new());
    for (key, program) in keys {
        let parsed = match fetched.get(&key) {
            Some(Ok(Some(account))) => parse_mint(&key, account, program),
            Some(Err(error)) => Err(anyhow::anyhow!(error.clone())),
            _ => Err(anyhow::anyhow!("Mint missing at scan commitment")),
        };
        match parsed {
            Ok(mint) => {
                for extension in mint.extensions.as_array().into_iter().flatten() {
                    if extension["extension"] == "metadataPointer"
                        && let Some(pointer) = extension["state"]["metadataAddress"].as_str()
                        && pointer != mint.mint
                    {
                        result.issue(format!(
                            "{}: external metadata pointer {} requires its program's decoder",
                            mint.mint, pointer
                        ));
                    }
                }
                for account in accounts
                    .iter_mut()
                    .filter(|account| account.mint == mint.mint && account.program == mint.program)
                {
                    account.decimals = Some(mint.decimals);
                }
                result.items.push(mint);
            }
            Err(error) => result.issue(format!("Mint {key}: {error}")),
        }
    }
    result
}
