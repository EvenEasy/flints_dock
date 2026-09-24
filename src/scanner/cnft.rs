use crate::models::ScanStatus;

pub fn support() -> ScanStatus {
    // Bubblegum ownership/metadata live in transaction events; tree accounts
    // contain hashes and a bounded changelog, not an enumerable owner map.
    // Wallet signature history can miss events where the owner is only in logs.
    // Complete replay requires retained history plus an index of all relevant
    // tree events, including transfers and burns; a short history scan is unsafe.
    ScanStatus::Unsupported("Compressed NFTs are not fully enumerable using standard RPC without an indexer. Bubblegum tree hashes cannot be searched by wallet owner; no DAS or history heuristic was used.".into())
}
