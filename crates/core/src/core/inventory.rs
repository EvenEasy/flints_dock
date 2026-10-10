//! Canonical inventory projection. Account addresses, rather than overlapping view rows, own balances.
use crate::core::{categories::CompressedReport, *};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

/// One selectable holding or NFT identity; all backing accounts remain explicit.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InventoryAsset {
    pub id: String,
    pub mint: Option<String>,
    pub program: String,
    pub kind: String,
    pub name: String,
    pub accounts: Vec<String>,
    pub raw_amount: Option<String>,
    pub decimals: Option<u8>,
    pub balance: Option<String>,
    pub value_usd: Option<f64>,
    pub evidence: String,
}

/// Normalize decoded accounts before aggregation. Prices and labels enrich balances, never define them.
/// A repeated account from tokens/allTokens is counted exactly once.
pub fn holdings(snapshot: &WalletSnapshot) -> Vec<TokenAsset> {
    if snapshot.token_accounts.is_empty() {
        // Selected semantic-only library scans may omit the raw projection.
        return amount::aggregate_holdings(if snapshot.all_tokens.is_empty() {
            &snapshot.tokens
        } else {
            &snapshot.all_tokens
        });
    }
    let views: Vec<_> = snapshot.all_tokens.iter().chain(&snapshot.tokens).collect();
    let mut seen = BTreeSet::new();
    let mut rows = vec![];
    for account in &snapshot.token_accounts {
        if !seen.insert(&account.address) {
            continue;
        }
        let view = views.iter().find(|a| {
            a.mint == account.mint
                && a.program == account.program
                && a.accounts.contains(&account.address)
        });
        let mint = snapshot
            .mints
            .iter()
            .find(|m| m.mint == account.mint && m.program == account.program);
        let price = views
            .iter()
            .find(|a| a.mint == account.mint && a.program == account.program && a.price.is_some())
            .and_then(|a| a.price.clone());
        // The classified view already merges verified Metaplex/Token-2022 enrichment.
        // Base legacy mint data has no name and must not overwrite that metadata.
        let metadata = view
            .map(|a| a.metadata.clone())
            .or_else(|| mint.map(|m| m.metadata.clone()))
            .unwrap_or_default();
        let decimals = account.decimals;
        let value_usd = decimals
            .zip(price.as_ref())
            .and_then(|(d, p)| amount::approximate_value(account.raw_amount.into(), d, p.usd));
        rows.push(TokenAsset {
            mint: account.mint.clone(),
            program: account.program,
            total_raw_amount: account.raw_amount.into(),
            decimals,
            balance: decimals.map(|d| amount::exact_amount(account.raw_amount.into(), d)),
            accounts: vec![account.address.clone()],
            kind: view
                .map(|a| a.kind)
                .or_else(|| {
                    snapshot
                        .classic_nfts
                        .iter()
                        .find(|n| {
                            n.mint == account.mint && n.token_accounts.contains(&account.address)
                        })
                        .map(|n| match (n.programmable, n.edition) {
                            (true, true) => AssetKind::ProgrammableNonFungibleEdition,
                            (true, false) => AssetKind::ProgrammableNonFungible,
                            (false, true) => AssetKind::NonFungibleEdition,
                            (false, false) => AssetKind::NonFungible,
                        })
                })
                .unwrap_or(AssetKind::Unknown),
            metadata,
            price,
            value_usd,
        });
    }
    amount::aggregate_holdings(&rows)
}

/// Return the shared selection/category inventory, including empty, unknown, Core and compressed assets.
pub fn normalize(snapshot: &WalletSnapshot, compressed: &CompressedReport) -> Vec<InventoryAsset> {
    let mut assets = BTreeMap::new();
    for token in holdings(snapshot) {
        let id = format!("{}:{}", token.mint, token.program.id());
        let evidence = if token.kind.is_nft() {
            snapshot
                .classic_nfts
                .iter()
                .find(|n| n.mint == token.mint)
                .map(|n| n.evidence.clone())
                .unwrap_or_else(|| "Verified on-chain NFT token standard".into())
        } else if token.kind.is_fungible() {
            "Verified mint and token-program semantics".into()
        } else {
            "Classification unresolved; on-chain standard or edition evidence required".into()
        };
        assets.insert(
            id.clone(),
            InventoryAsset {
                id,
                mint: Some(token.mint.clone()),
                program: token.program.id().to_string(),
                kind: if token.kind.is_nft() {
                    "nft"
                } else if token.kind.is_fungible() {
                    "fungible"
                } else {
                    "unknown"
                }
                .into(),
                name: token
                    .metadata
                    .name
                    .or(token.metadata.symbol)
                    .unwrap_or(token.mint),
                accounts: token.accounts,
                raw_amount: Some(token.total_raw_amount.to_string()),
                decimals: token.decimals,
                balance: token.balance,
                value_usd: token.value_usd,
                evidence,
            },
        );
    }
    for nft in &snapshot.classic_nfts {
        let id = format!("{}:{}", nft.mint, TokenProgram::Legacy.id());
        assets.entry(id.clone()).or_insert_with(|| InventoryAsset {
            id,
            mint: Some(nft.mint.clone()),
            program: TokenProgram::Legacy.id().to_string(),
            kind: "nft".into(),
            name: nft.metadata.name.clone().unwrap_or(nft.mint.clone()),
            accounts: nft.token_accounts.clone(),
            raw_amount: Some("1".into()),
            decimals: Some(0),
            balance: Some("1".into()),
            value_usd: None,
            evidence: nft.evidence.clone(),
        });
    }
    for core in &snapshot.core_assets {
        assets.insert(
            core.address.clone(),
            InventoryAsset {
                id: core.address.clone(),
                mint: None,
                program: mpl_core::ID.to_string(),
                kind: "core".into(),
                name: core.name.clone(),
                accounts: vec![],
                raw_amount: None,
                decimals: None,
                balance: None,
                value_usd: None,
                evidence: "Verified MPL Core AssetV1 ownership".into(),
            },
        );
    }
    for cnft in &compressed.items {
        assets
            .entry(cnft.id.clone())
            .or_insert_with(|| InventoryAsset {
                id: cnft.id.clone(),
                mint: None,
                program: mpl_bubblegum::ID.to_string(),
                kind: "compressed".into(),
                name: cnft.name.clone(),
                accounts: vec![],
                raw_amount: None,
                decimals: None,
                balance: None,
                value_usd: None,
                evidence: "Network-scoped DAS ownership; fresh Merkle proof required for burn"
                    .into(),
            });
    }
    let accounts: BTreeSet<_> = snapshot
        .token_accounts
        .iter()
        .map(|a| a.address.as_str())
        .collect();
    for unknown in &snapshot.unknown_assets {
        if unknown.lamports.is_none()
            || accounts.contains(unknown.address.as_str())
            || assets
                .values()
                .any(|a| a.accounts.contains(&unknown.address))
        {
            continue;
        }
        assets
            .entry(unknown.address.clone())
            .or_insert_with(|| InventoryAsset {
                id: unknown.address.clone(),
                mint: None,
                program: unknown.program_id.clone().unwrap_or_default(),
                kind: "undecodable".into(),
                name: unknown.address.clone(),
                accounts: vec![unknown.address.clone()],
                raw_amount: None,
                decimals: None,
                balance: None,
                value_usd: None,
                evidence: unknown.reason.clone(),
            });
    }
    assets.into_values().collect()
}
