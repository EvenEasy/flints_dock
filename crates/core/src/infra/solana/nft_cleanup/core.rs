//! Core ownership, inherited plugins and Asset Signer safety.
use super::{account, blocked, invalid, io, key};
use crate::{
    app::{cleanup::*, swap::*},
    core::{nft_cleanup::*, *},
    infra::solana,
};
use serde_json::json;
use solana_instruction::Instruction;
use solana_pubkey::Pubkey;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
pub(super) fn core_plugins(
    plugins: &mpl_core::PluginsList,
    external: &mpl_core::ExternalPluginAdaptersList,
) -> Result<()> {
    if plugins
        .freeze_delegate
        .as_ref()
        .is_some_and(|p| p.freeze_delegate.frozen)
        || plugins
            .permanent_freeze_delegate
            .as_ref()
            .is_some_and(|p| p.permanent_freeze_delegate.frozen)
    {
        return Err(blocked(
            CleanupReasonCode::Frozen,
            "Core asset/collection freeze plugin blocks burn",
        ));
    }
    if !external.lifecycle_hooks.is_empty()
        || !external.linked_lifecycle_hooks.is_empty()
        || !external.oracles.is_empty()
    {
        return Err(blocked(
            CleanupReasonCode::UnsupportedAccountExtension,
            "Core external lifecycle hooks/oracles require unsupported remaining accounts",
        ));
    }
    Ok(())
}
/// Typed registry decoding rejects unknown future plugins rather than silently ignoring their authority rules.
pub(super) fn known_registry(
    data: &[u8],
    header: Option<&mpl_core::accounts::PluginHeaderV1>,
) -> Result<()> {
    if let Some(header) = header {
        let offset = usize::try_from(header.plugin_registry_offset).map_err(invalid)?;
        let bytes = data
            .get(offset..)
            .ok_or_else(|| invalid("Core plugin registry offset is invalid"))?;
        let registry = mpl_core::accounts::PluginRegistryV1::from_bytes(bytes).map_err(|_| {
            blocked(
                CleanupReasonCode::UnsupportedAccountExtension,
                "Core registry includes an unsupported plugin",
            )
        })?;
        if registry.key != mpl_core::types::Key::PluginRegistryV1 {
            return Err(invalid("Core plugin registry discriminator mismatch"));
        }
    }
    Ok(())
}

pub(super) async fn core_burn(
    rpc: &RpcClient,
    target: &NftTarget,
    owner: &Pubkey,
    das: Option<&crate::infra::solana::scan::das::DasClient>,
) -> Result<PreparedNftBurn> {
    let address = key(&target.id)?;
    let raw = account(
        rpc,
        &address,
        Pubkey::new_from_array(mpl_core::ID.to_bytes()),
    )
    .await?;
    let asset = mpl_core::Asset::from_bytes(&raw.data).map_err(invalid)?;
    if asset.base.owner.to_bytes() != owner.to_bytes() {
        return Err(invalid("Core owner changed"));
    }
    known_registry(&raw.data, asset.plugin_header.as_ref())?;
    core_plugins(&asset.plugin_list, &asset.external_plugin_adapter_list)?;
    let collection = match asset.base.update_authority {
        mpl_core::types::UpdateAuthority::Collection(key) => Some(key),
        _ => None,
    };
    let collection_data = if let Some(collection) = collection {
        let raw = account(
            rpc,
            &Pubkey::new_from_array(collection.to_bytes()),
            Pubkey::new_from_array(mpl_core::ID.to_bytes()),
        )
        .await?;
        let decoded = mpl_core::Collection::from_bytes(&raw.data).map_err(invalid)?;
        known_registry(&raw.data, decoded.plugin_header.as_ref())?;
        core_plugins(&decoded.plugin_list, &decoded.external_plugin_adapter_list)?;
        Some(
            json!({"address":collection.to_string(),"authority":decoded.base.update_authority.to_string(),"name":decoded.base.name,"uri":decoded.base.uri,"plugins":format!("{:?}",decoded.plugin_list),"external":format!("{:?}",decoded.external_plugin_adapter_list)}),
        )
    } else {
        None
    };
    // Burning disables Execute forever. Refuse to strand any SOL, token accounts or nested assets.
    let signer = Pubkey::new_from_array(
        mpl_core::accounts::AssetSigner::find_pda(&address.to_string().parse().map_err(invalid)?)
            .0
            .to_bytes(),
    );
    if rpc.get_balance(&signer).await.map_err(io)? > 0 {
        return Err(blocked(
            CleanupReasonCode::AssetSignerFunds,
            "Core Asset Signer still holds SOL; withdraw before burn",
        ));
    }
    for program in [TokenProgram::Legacy, TokenProgram::Token2022] {
        if !solana::token_accounts(rpc, &signer, &program.id())
            .await
            .map_err(io)?
            .is_empty()
        {
            return Err(blocked(
                CleanupReasonCode::AssetSignerFunds,
                "Core Asset Signer still owns token accounts; withdraw before burn",
            ));
        }
    }
    let nested = solana::scan::core::get_core_assets(rpc, &signer).await;
    if !nested.status.is_complete() || !nested.items.is_empty() {
        return Err(invalid(
            "Core Asset Signer nested asset coverage is incomplete or nonempty",
        ));
    }
    // Compressed holdings can belong to a PDA without a token account. Require indexed coverage before disabling its signer.
    let das = das.ok_or_else(|| {
        blocked(
            CleanupReasonCode::NftEvidenceUnavailable,
            "DAS required to verify Core Asset Signer has no compressed holdings",
        )
    })?;
    let network = rpc.get_genesis_hash().await.map_err(io)?.to_string();
    let compressed = das
        .compressed_on_network(&signer.to_string(), &network)
        .await;
    if !compressed.status.is_complete() {
        return Err(blocked(
            CleanupReasonCode::NftEvidenceUnavailable,
            "Core Asset Signer compressed coverage is incomplete",
        ));
    }
    if !compressed.items.is_empty() {
        return Err(blocked(
            CleanupReasonCode::AssetSignerFunds,
            "Core Asset Signer owns compressed NFTs; withdraw before burn",
        ));
    }
    let ix = mpl_core::instructions::BurnV1 {
        asset: address.to_string().parse().map_err(invalid)?,
        collection,
        payer: owner.to_string().parse().map_err(invalid)?,
        authority: Some(owner.to_string().parse().map_err(invalid)?),
        system_program: Some(Pubkey::default().to_string().parse().map_err(invalid)?),
        log_wrapper: None,
    }
    .instruction(mpl_core::instructions::BurnV1InstructionArgs {
        compression_proof: None,
    });
    let ix = Instruction {
        program_id: Pubkey::new_from_array(ix.program_id.to_bytes()),
        data: ix.data,
        accounts: ix
            .accounts
            .into_iter()
            .map(|a| solana_instruction::AccountMeta {
                pubkey: Pubkey::new_from_array(a.pubkey.to_bytes()),
                is_signer: a.is_signer,
                is_writable: a.is_writable,
            })
            .collect(),
    };
    Ok(PreparedNftBurn {
        identity: json!({"target":target,"asset":raw.data,"collection":collection_data}),
        instructions: vec![ix],
        edition_parent: None,
        edition_count: 0,
    })
}
