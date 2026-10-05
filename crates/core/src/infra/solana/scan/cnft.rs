use crate::core::ScanStatus;

/// Return the explicit cNFT discovery limitation for a standard-RPC-only deployment.
/// Bubblegum tree state does not expose an enumerable wallet-to-asset index.
pub fn support() -> ScanStatus {
    // Bubblegum ownership/metadata live in transaction events; tree accounts
    // contain hashes and a bounded changelog, not an enumerable owner map.
    // Wallet signature history can miss events where the owner is only in logs.
    // Complete replay requires retained history plus an index of all relevant
    // tree events, including transfers and burns; a short history scan is unsafe.
    ScanStatus::Unsupported("Compressed NFTs are not fully enumerable using standard RPC without an indexer. Bubblegum tree hashes cannot be searched by wallet owner; no DAS or history heuristic was used.".into())
}

/// Return the unsupported discovery status rather than an empty cNFT inventory.
/// The owner is reserved for a future indexed implementation; no RPC history heuristic is used.
pub fn get_compressed_nfts(_owner: &solana_pubkey::Pubkey) -> ScanStatus {
    support()
}
