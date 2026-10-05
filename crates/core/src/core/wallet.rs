use super::asset::*;
use super::error::ScanStatus;
use serde::Serialize;
use std::collections::BTreeMap;

/// Return decoded items alongside partial failures and unrecognized account data.
#[derive(Debug, Serialize)]
pub struct ScanCollection<T> {
    pub status: ScanStatus,
    pub items: Vec<T>,
    pub unknown: Vec<super::UnknownAsset>,
}

impl<T> ScanCollection<T> {
    pub fn complete(items: Vec<T>) -> Self {
        Self {
            status: ScanStatus::Complete,
            items,
            unknown: Vec::new(),
        }
    }

    pub fn failed(reason: impl Into<String>) -> Self {
        Self {
            status: ScanStatus::Failed(reason.into()),
            items: Vec::new(),
            unknown: Vec::new(),
        }
    }

    pub fn issue(&mut self, reason: impl Into<String>) {
        let reason = reason.into();
        match &mut self.status {
            ScanStatus::Partial(message) => {
                message.push_str("; ");
                message.push_str(&reason);
            }
            _ => self.status = ScanStatus::Partial(reason),
        }
    }
}

/// Choose which wallet categories need discovery and enrichment.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ScanSelection {
    pub balance: bool,
    pub tokens: bool,
    pub all_tokens: bool,
    pub nfts: bool,
    pub cnfts: bool,
}

impl ScanSelection {
    pub const ALL: Self = Self {
        balance: true,
        tokens: true,

        // The raw view overlaps the semantic categories and is explicitly opt-in.
        all_tokens: false,
        nfts: true,
        cnfts: true,
    };

    pub fn is_empty(self) -> bool {
        !self.balance && !self.tokens && !self.all_tokens && !self.nfts && !self.cnfts
    }

    /// Request shared token discovery only for categories that depend on backing accounts.
    pub fn needs_token_accounts(self) -> bool {
        self.tokens || self.all_tokens || self.nfts
    }

    /// Enable optional pricing only when selected categories contain priceable balances.
    pub fn needs_prices(self) -> bool {
        self.balance || self.tokens || self.all_tokens
    }
}

/// Configure discovery categories and whether external pricing is allowed.
#[derive(Debug, Clone, Copy)]
pub struct ScanOptions {
    pub selection: ScanSelection,
    pub no_prices: bool,
}

impl Default for ScanOptions {
    fn default() -> Self {
        Self {
            selection: ScanSelection::ALL,
            no_prices: false,
        }
    }
}

/// Collect normalized wallet holdings, enrichment results and per-scanner status.
#[derive(Debug)]
pub struct WalletSnapshot {
    pub selected: ScanSelection,
    pub owner: String,
    pub commitment: String,
    pub native_sol: Option<NativeBalance>,
    pub token_accounts: Vec<TokenAccount>,
    pub tokens: Vec<TokenAsset>,
    pub all_tokens: Vec<TokenAsset>,
    pub mints: Vec<MintInfo>,
    pub classic_nfts: Vec<NftAsset>,
    pub core_assets: Vec<CoreAsset>,
    pub unknown_assets: Vec<UnknownAsset>,
    pub scanners: BTreeMap<String, ScanStatus>,
    pub account_summary: AccountSummary,
}

impl WalletSnapshot {
    /// Unavailable-only scans exit unsuccessfully, while partial results remain useful.
    pub fn has_usable_results(&self) -> bool {
        ["native_sol", "tokens", "all_tokens", "nfts"]
            .iter()
            .any(|category| {
                self.scanners.get(*category).is_some_and(|status| {
                    matches!(status, ScanStatus::Complete | ScanStatus::Partial(_))
                })
            })
    }
}

/// Share account, mint and metadata discovery across token and NFT views.
pub struct TokenInventory {
    pub accounts: Vec<TokenAccount>,
    pub classification_accounts: Vec<TokenAccount>,
    pub mints: Vec<MintInfo>,
    pub records: Vec<MetadataRecord>,
    pub unknown: Vec<UnknownAsset>,
    pub status: ScanStatus,
    pub scanners: BTreeMap<String, ScanStatus>,
}

/// Return a combined discovery status while preserving partial success.
/// Component names are included in diagnostics; a wholly unusable set is reported as failed.
pub fn combine_statuses(statuses: &[(&str, &ScanStatus)]) -> ScanStatus {
    if statuses.iter().all(|(_, status)| status.is_complete()) {
        return ScanStatus::Complete;
    }
    let reason = statuses
        .iter()
        .filter(|(_, status)| !status.is_complete())
        .map(|(name, status)| {
            format!(
                "{name}: {}",
                match status {
                    ScanStatus::Partial(reason)
                    | ScanStatus::Failed(reason)
                    | ScanStatus::Unsupported(reason)
                    | ScanStatus::Skipped(reason) => reason,
                    ScanStatus::Complete => "",
                }
            )
        })
        .collect::<Vec<_>>()
        .join("; ");
    if statuses.iter().all(|(_, status)| {
        matches!(
            status,
            ScanStatus::Failed(_) | ScanStatus::Unsupported(_) | ScanStatus::Skipped(_)
        )
    }) {
        ScanStatus::Failed(reason)
    } else {
        ScanStatus::Partial(reason)
    }
}
