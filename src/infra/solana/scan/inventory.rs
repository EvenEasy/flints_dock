use crate::{app::scan_wallet::WalletReader, core::*};
use solana_pubkey::Pubkey;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use std::collections::BTreeMap;
async fn scan_tokens(rpc: &RpcClient, owner: &Pubkey, nfts_only: bool) -> TokenInventory {
    // Discover both token programs independently so one failure does not erase the other.
    let (mut legacy, mut token2022) = tokio::join!(
        super::tokens::get_token_accounts(rpc, owner, TokenProgram::Legacy),
        super::tokens::get_token_accounts(rpc, owner, TokenProgram::Token2022),
    );
    let discovery = combine_statuses(&[
        ("legacy", &legacy.status),
        ("token_2022", &token2022.status),
    ]);
    let mut accounts = std::mem::take(&mut legacy.items);
    accounts.append(&mut token2022.items);
    accounts.sort_by(|a, b| a.address.cmp(&b.address));

    // NFT-only runs need mint/metadata lookups only for possible NFT holdings.
    // Keep all raw accounts internally, without guessing decimals for unqueried mints.
    let mut candidates: Vec<_> = accounts
        .iter()
        .filter(|account| !nfts_only || account.raw_amount == 1)
        .cloned()
        .collect();
    let mint_scan = super::metadata::get_mints(rpc, &mut candidates).await;

    // Reuse a single index for account decimals and NFT candidate selection.
    let mints: BTreeMap<_, _> = mint_scan
        .items
        .iter()
        .map(|mint| ((mint.mint.as_str(), mint.program), mint))
        .collect();
    for account in &mut accounts {
        account.decimals = mints
            .get(&(account.mint.as_str(), account.program))
            .map(|mint| mint.decimals);
    }
    if nfts_only {
        candidates.retain(|account| {
            account.decimals == Some(0)
                && mints
                    .get(&(account.mint.as_str(), account.program))
                    .is_some_and(|mint| mint.supply == 1)
        });
    }

    // Enrich classification candidates with shared Metaplex evidence.
    let metadata_scan = super::nft::get_metadata(rpc, &candidates, &mint_scan.items).await;

    // Preserve total discovery failure and otherwise expose incomplete enrichment as partial.
    let status = if matches!(discovery, ScanStatus::Failed(_)) {
        discovery.clone()
    } else {
        combine_statuses(&[
            ("discovery", &discovery),
            ("mints", &mint_scan.status),
            ("metadata", &metadata_scan.status),
        ])
    };
    let scanners = BTreeMap::from([
        ("legacy_tokens".into(), legacy.status),
        ("token_2022".into(), token2022.status),
        ("mint_metadata".into(), mint_scan.status),
        ("metaplex_metadata_and_nfts".into(), metadata_scan.status),
    ]);
    let mut unknown = legacy.unknown;
    unknown.extend(token2022.unknown);
    TokenInventory {
        accounts,
        classification_accounts: candidates,
        mints: mint_scan.items,
        records: metadata_scan.items,
        unknown,
        status,
        scanners,
    }
}

impl WalletReader for RpcClient {
    fn commitment(&self) -> String {
        format!("{:?} (independent requests)", self.commitment().commitment)
    }
    async fn native_balance(&self, owner: &Pubkey) -> Result<u64, String> {
        self.get_balance(owner).await.map_err(|e| e.to_string())
    }
    async fn token_inventory(&self, owner: &Pubkey, nfts_only: bool) -> TokenInventory {
        scan_tokens(self, owner, nfts_only).await
    }
    async fn core_assets(&self, owner: &Pubkey) -> ScanCollection<CoreAsset> {
        super::core::get_core_assets(self, owner).await
    }
    fn compressed_nfts(&self, owner: &Pubkey) -> ScanStatus {
        super::cnft::get_compressed_nfts(owner)
    }
}
