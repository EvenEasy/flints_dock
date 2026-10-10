//! Fresh DAS proof validation and Bubblegum v1/v2 burn preparation.
use super::core::{core_plugins, known_registry};
use super::{account, blocked, invalid, io, key};
use crate::infra::solana::scan::das::DasClient;
use crate::{
    app::swap::*,
    core::{cleanup::CleanupReasonCode, nft_cleanup::*},
};
use serde_json::Value;
use serde_json::json;
use solana_instruction::Instruction;
use solana_pubkey::Pubkey;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
fn hash_field(value: &Value, field: &str) -> Result<[u8; 32]> {
    Ok(key(value[field]
        .as_str()
        .ok_or_else(|| invalid(format!("Missing compressed NFT {field}")))?)?
    .to_bytes())
}

/// Verify the full leaf/proof locally; on-chain simulation verifies that root is still accepted by the tree.
fn proof_root(mut node: [u8; 32], proof: &[[u8; 32]], index: u32) -> [u8; 32] {
    for (level, sibling) in proof.iter().enumerate() {
        node = if (u64::from(index) >> level) & 1 == 0 {
            solana_keccak_hasher::hashv(&[&node, sibling]).to_bytes()
        } else {
            solana_keccak_hasher::hashv(&[sibling, &node]).to_bytes()
        };
    }
    node
}

/// Decode the stable compression header and derive canopy size without deforming/trusting the DAS proof.
fn canopy(data: &[u8], proof_length: usize) -> Result<usize> {
    if data.len() < 56 || data[0] != 1 || data[1] != 0 {
        return Err(invalid("Unsupported concurrent Merkle tree header"));
    }
    let buffer = u32::from_le_bytes(data[2..6].try_into().expect("slice")) as usize;
    let depth = u32::from_le_bytes(data[6..10].try_into().expect("slice")) as usize;
    if depth == 0 || depth > 32 || buffer == 0 || proof_length != depth {
        return Err(invalid("Merkle proof depth does not match tree header"));
    }
    let path = 32 * (depth + 1) + 8;
    let base = 56usize
        .checked_add(24)
        .and_then(|v| buffer.checked_mul(path).and_then(|b| v.checked_add(b)))
        .and_then(|v| v.checked_add(path))
        .ok_or_else(|| invalid("Invalid tree size"))?;
    let bytes = data
        .len()
        .checked_sub(base)
        .ok_or_else(|| invalid("Truncated Merkle tree"))?;
    if bytes % 32 != 0 {
        return Err(invalid("Invalid canopy size"));
    }
    let nodes = bytes / 32;
    let size = nodes
        .checked_add(2)
        .ok_or_else(|| invalid("Canopy size overflow"))?;
    if !size.is_power_of_two() {
        return Err(invalid("Invalid canopy node count"));
    }
    let canopy = size.trailing_zeros() as usize - 1;
    if canopy > depth {
        return Err(invalid("Canopy exceeds tree depth"));
    }
    Ok(canopy)
}

pub(super) async fn compressed_burn(
    rpc: &RpcClient,
    das: &DasClient,
    target: &NftTarget,
    owner: &Pubkey,
) -> Result<PreparedNftBurn> {
    use mpl_bubblegum::{
        accounts::TreeConfig,
        instructions::*,
        types::{LeafSchema, Version},
    };
    let genesis = rpc.get_genesis_hash().await.map_err(io)?.to_string();
    if das.call("getGenesisHash", json!([])).await?.as_str() != Some(genesis.as_str()) {
        return Err(invalid("DAS/RPC network mismatch"));
    }
    let asset = das.asset(&target.id).await?;
    let proof = das.call("getAssetProof", json!({"id":target.id})).await?;
    if asset["id"].as_str() != Some(target.id.as_str())
        || asset["ownership"]["owner"].as_str() != Some(owner.to_string().as_str())
        || asset["burnt"] != false
        || asset["compression"]["compressed"] != true
    {
        return Err(invalid(
            "Compressed NFT identity, owner or burned state changed",
        ));
    }
    let compression = &asset["compression"];
    let tree = key(compression["tree"]
        .as_str()
        .ok_or_else(|| invalid("Compressed NFT tree missing"))?)?;
    if proof["tree_id"].as_str() != Some(tree.to_string().as_str()) {
        return Err(invalid("DAS proof belongs to a different tree"));
    }
    let nonce = compression["leaf_id"]
        .as_u64()
        .ok_or_else(|| invalid("Compressed NFT nonce missing"))?;
    // DAS node_index locates the leaf; nonce identifies it. They must not be conflated.
    let depth = proof["proof"]
        .as_array()
        .ok_or_else(|| invalid("Merkle proof missing"))?
        .len();
    if depth == 0 || depth > 32 {
        return Err(invalid("Invalid proof depth"));
    }
    let node = proof["node_index"]
        .as_u64()
        .ok_or_else(|| invalid("Leaf node index missing"))?;
    let index = node
        .checked_sub(1u64 << depth)
        .filter(|index| *index < (1u64 << depth))
        .and_then(|index| u32::try_from(index).ok())
        .ok_or_else(|| invalid("Invalid proof leaf index"))?;
    let id = Pubkey::find_program_address(
        &[b"asset", tree.as_ref(), &nonce.to_le_bytes()],
        &Pubkey::new_from_array(mpl_bubblegum::ID.to_bytes()),
    )
    .0;
    if id.to_string() != target.id {
        return Err(invalid("Compressed NFT asset ID does not match tree/nonce"));
    }
    let delegate = asset["ownership"]["delegate"]
        .as_str()
        .map(key)
        .transpose()?
        .unwrap_or(*owner);
    if asset["ownership"]["delegated"] == true && asset["ownership"]["delegate"].as_str().is_none()
    {
        return Err(invalid("Compressed NFT delegate evidence missing"));
    }
    let config_address = TreeConfig::find_pda(&tree.to_string().parse().map_err(invalid)?).0;
    let raw_config = account(
        rpc,
        &Pubkey::new_from_array(config_address.to_bytes()),
        Pubkey::new_from_array(mpl_bubblegum::ID.to_bytes()),
    )
    .await?;
    let config = TreeConfig::from_bytes(&raw_config.data).map_err(invalid)?;
    if config.discriminator != [122, 245, 175, 248, 171, 34, 0, 207]
        || nonce >= config.num_minted
        || u64::from(index) >= config.total_mint_capacity
    {
        return Err(invalid("Invalid Bubblegum TreeConfig or leaf index"));
    }
    let v2 = config.version == Version::V2;
    let compression_program: Pubkey = if v2 {
        "mcmt6YrQEMKw8Mw43FmpRLmf7BqRnFMKmAcbxE3xkAW"
    } else {
        "cmtDvXumGCrqC1Age74AVPhSRVXJMd8PJS91L8KbNCK"
    }
    .parse()
    .expect("compression program");
    let raw_tree = account(rpc, &tree, compression_program).await?;
    let root = hash_field(&proof, "root")?;
    let data_hash = hash_field(compression, "data_hash")?;
    let creator_hash = hash_field(compression, "creator_hash")?;
    let core_collection = asset["grouping"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|g| g["group_key"] == "collection")
        .and_then(|g| g["group_value"].as_str())
        .map(key)
        .transpose()?;
    let flags = if v2 {
        Some(
            u8::try_from(
                compression["flags"]
                    .as_u64()
                    .ok_or_else(|| invalid("V2 leaf flags unavailable"))?,
            )
            .map_err(invalid)?,
        )
    } else {
        None
    };
    if flags.is_some_and(|flags| flags & 3 != 0) {
        return Err(blocked(
            CleanupReasonCode::Frozen,
            "Compressed NFT is frozen; owner burn is blocked",
        ));
    }
    let asset_data_hash = if v2 {
        Some(hash_field(compression, "asset_data_hash")?)
    } else {
        None
    };
    let leaf = if v2 {
        LeafSchema::V2 {
            id: target.id.parse().map_err(invalid)?,
            owner: owner.to_string().parse().map_err(invalid)?,
            delegate: delegate.to_string().parse().map_err(invalid)?,
            nonce,
            data_hash,
            creator_hash,
            collection_hash: hash_field(compression, "collection_hash")?,
            asset_data_hash: asset_data_hash.expect("v2"),
            flags: flags.expect("v2"),
        }
    } else {
        LeafSchema::V1 {
            id: target.id.parse().map_err(invalid)?,
            owner: owner.to_string().parse().map_err(invalid)?,
            delegate: delegate.to_string().parse().map_err(invalid)?,
            nonce,
            data_hash,
            creator_hash,
        }
    };
    if v2 {
        let collection_hash = mpl_bubblegum::hash::hash_collection_option(
            core_collection
                .map(|c| c.to_string().parse())
                .transpose()
                .map_err(invalid)?,
        )
        .map_err(invalid)?;
        if collection_hash != hash_field(compression, "collection_hash")? {
            return Err(invalid("V2 Core collection hash mismatch"));
        }
        if let Some(collection) = core_collection {
            let raw = account(
                rpc,
                &collection,
                Pubkey::new_from_array(mpl_core::ID.to_bytes()),
            )
            .await?;
            let collection = mpl_core::Collection::from_bytes(&raw.data).map_err(invalid)?;
            known_registry(&raw.data, collection.plugin_header.as_ref())?;
            core_plugins(
                &collection.plugin_list,
                &collection.external_plugin_adapter_list,
            )?;
        }
    }
    let nodes: Vec<_> = proof["proof"]
        .as_array()
        .ok_or_else(|| invalid("Merkle proof missing"))?
        .iter()
        .map(|v| {
            key(v.as_str().ok_or_else(|| invalid("Invalid proof node"))?).map(|k| k.to_bytes())
        })
        .collect::<Result<_>>()?;
    if nodes.len() > 32
        || hash_field(&proof, "leaf")? != leaf.hash()
        || proof_root(leaf.hash(), &nodes, index) != root
    {
        return Err(invalid("Invalid compressed NFT leaf/Merkle proof"));
    }
    if proof["node_index"].as_u64() != Some((1u64 << nodes.len()) + u64::from(index)) {
        return Err(invalid(
            "Compressed NFT proof leaf index does not match approved nonce",
        ));
    }
    let canopy = canopy(&raw_tree.data, nodes.len())?;
    let remaining: Vec<_> = nodes[..nodes.len() - canopy]
        .iter()
        .map(|n| solana_instruction::AccountMeta::new_readonly(Pubkey::new_from_array(*n), false))
        .collect();
    let ix = if v2 {
        BurnV2 {
            tree_config: config_address,
            payer: owner.to_string().parse().map_err(invalid)?,
            authority: Some(owner.to_string().parse().map_err(invalid)?),
            leaf_owner: owner.to_string().parse().map_err(invalid)?,
            leaf_delegate: Some(delegate.to_string().parse().map_err(invalid)?),
            merkle_tree: tree.to_string().parse().map_err(invalid)?,
            core_collection: core_collection
                .map(|k| k.to_string().parse())
                .transpose()
                .map_err(invalid)?,
            mpl_core_cpi_signer: Some(
                Pubkey::find_program_address(
                    &[b"mpl_core_cpi_signer"],
                    &Pubkey::new_from_array(mpl_bubblegum::ID.to_bytes()),
                )
                .0
                .to_string()
                .parse()
                .map_err(invalid)?,
            ),
            log_wrapper: "mnoopTCrg4p8ry25e4bcWA9XZjbNjMTfgYVGGEdRsf3"
                .parse()
                .expect("noop"),
            compression_program: compression_program.to_string().parse().map_err(invalid)?,
            mpl_core_program: mpl_core::ID.to_string().parse().map_err(invalid)?,
            system_program: Pubkey::default().to_string().parse().map_err(invalid)?,
        }
        .instruction(BurnV2InstructionArgs {
            root,
            data_hash,
            creator_hash,
            asset_data_hash,
            flags,
            nonce,
            index,
        })
    } else {
        Burn {
            tree_config: config_address,
            leaf_owner: (owner.to_string().parse().map_err(invalid)?, true),
            leaf_delegate: (delegate.to_string().parse().map_err(invalid)?, false),
            merkle_tree: tree.to_string().parse().map_err(invalid)?,
            log_wrapper: "noopb9bkMVfRPU8AsbpTUg8AQkHtKwMYZiFUjNRtMmV"
                .parse()
                .expect("noop"),
            compression_program: compression_program.to_string().parse().map_err(invalid)?,
            system_program: Pubkey::default().to_string().parse().map_err(invalid)?,
        }
        .instruction(BurnInstructionArgs {
            root,
            data_hash,
            creator_hash,
            nonce,
            index,
        })
    };
    let mut ix = Instruction {
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
    ix.accounts.extend(remaining);
    // Root/proof can change; approved owner, tree, leaf identity and immutable data cannot.
    Ok(PreparedNftBurn {
        identity: json!({"target":target,"tree":tree.to_string(),"nonce":nonce,"index":index,"delegate":delegate.to_string(),"compression":compression,"version":v2}),
        instructions: vec![ix],
        edition_parent: None,
        edition_count: 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn merkle_proof_orientation_index_and_canopy_are_not_interchangeable() {
        let leaf = [1; 32];
        let siblings = [[2; 32], [3; 32], [4; 32]];
        assert_ne!(
            proof_root(leaf, &siblings, 0),
            proof_root(leaf, &siblings, 1)
        );
        let depth = 3usize;
        let buffer = 8usize;
        let path = 32 * (depth + 1) + 8;
        let mut tree = vec![0; 56 + 24 + buffer * path + path + 32 * 6];
        tree[0] = 1;
        tree[2..6].copy_from_slice(&(buffer as u32).to_le_bytes());
        tree[6..10].copy_from_slice(&(depth as u32).to_le_bytes());
        assert_eq!(canopy(&tree, depth).unwrap(), 2);
        assert!(canopy(&tree, depth - 1).is_err());
        tree.pop();
        assert!(canopy(&tree, depth).is_err());
    }
    #[test]
    fn bubblegum_versions_use_the_official_distinct_burn_discriminators() {
        use mpl_bubblegum::instructions::{BurnBuilder, BurnV2Builder};
        let key = solana_pubkey::Pubkey::new_from_array([1; 32]).to_string();
        let key = key.parse().unwrap();
        let v1 = BurnBuilder::new()
            .tree_config(key)
            .leaf_owner(key, true)
            .leaf_delegate(key, false)
            .merkle_tree(key)
            .root([1; 32])
            .data_hash([2; 32])
            .creator_hash([3; 32])
            .nonce(0)
            .index(0)
            .instruction();
        let v2 = BurnV2Builder::new()
            .tree_config(key)
            .payer(key)
            .leaf_owner(key)
            .merkle_tree(key)
            .root([1; 32])
            .data_hash([2; 32])
            .creator_hash([3; 32])
            .asset_data_hash([4; 32])
            .flags(0)
            .nonce(0)
            .index(0)
            .instruction();
        assert_eq!(&v1.data[..8], &[116, 110, 29, 56, 107, 219, 42, 93]);
        assert_eq!(&v2.data[..8], &[115, 210, 34, 240, 232, 143, 183, 16]);
        assert_ne!(v1.accounts.len(), v2.accounts.len());
    }
}
