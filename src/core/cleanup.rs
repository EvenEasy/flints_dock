use crate::{
    core::swap::{SwapLimits, SwapQuote},
    core::*,
};
use serde::Serialize;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum CleanupCategory {
    Empty,
    Swappable,
    Burnable,
    Unsupported,
}
#[derive(Debug, Clone, Serialize)]
pub struct CleanupAsset {
    pub account: TokenAccount,
    pub mint: Option<MintInfo>,
    pub kind: AssetKind,
}
#[derive(Debug, Clone, Serialize)]
pub struct CleanupEntry {
    pub asset: CleanupAsset,
    pub category: CleanupCategory,
    pub reason: String,
    pub quote: Option<SwapQuote>,
}
#[derive(Debug, Serialize)]
pub struct CleanupPlan {
    pub wallet: String,
    pub discovery_status: ScanStatus,
    pub entries: Vec<CleanupEntry>,
    pub unparsed_accounts: Vec<UnknownAsset>,
    pub summary: CleanupSummary,
    pub selection: CleanupSelection,
}
#[derive(Debug, Default, Serialize)]
pub struct CleanupSummary {
    pub token_accounts: usize,
    pub empty: usize,
    pub swappable: usize,
    pub burnable: usize,
    pub unsupported: usize,
    pub accounts_to_close: usize,
    #[serde(serialize_with = "integer_string")]
    pub estimated_swap_lamports: u128,
    #[serde(serialize_with = "integer_string")]
    pub estimated_reclaimed_lamports: u128,
}
#[derive(Debug, Clone)]
pub struct CleanupOptions {
    pub selection: CleanupSelection,
    pub swap_limits: SwapLimits,
    pub slippage_bps: u16,
    pub quote_interval: Duration,
    pub quote_attempts: usize,
}
impl Default for CleanupOptions {
    fn default() -> Self {
        Self {
            selection: CleanupSelection::default(),
            swap_limits: SwapLimits::default(),
            slippage_bps: 50,
            quote_interval: Duration::from_millis(2100),
            quote_attempts: 2,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum CleanupOperation {
    Swap,
    Burn,
    Close,
}
#[derive(Debug, Clone, Serialize)]
pub struct OperationReceipt {
    pub operation: CleanupOperation,
    pub signature: String,
    #[serde(serialize_with = "optional_integer")]
    pub wallet_delta_lamports: Option<i128>,
    #[serde(serialize_with = "optional_integer")]
    pub reclaimed_lamports: Option<u64>,
}
fn optional_integer<T: ToString, S: serde::Serializer>(
    value: &Option<T>,
    serializer: S,
) -> std::result::Result<S::Ok, S::Error> {
    value
        .as_ref()
        .map(ToString::to_string)
        .serialize(serializer)
}
#[derive(Debug, Serialize)]
pub struct AccountCleanupResult {
    pub token_account: String,
    pub mint: String,
    pub category: CleanupCategory,
    pub status: String,
    pub reason: String,
    pub operations: Vec<OperationReceipt>,
    pub uncertain_signature: Option<String>,
}
#[derive(Debug, Default, Serialize)]
pub struct CleanupReport {
    pub results: Vec<AccountCleanupResult>,
    pub closed: usize,
    pub failed: usize,
    pub skipped: usize,
    #[serde(serialize_with = "integer_string")]
    pub known_swap_net_lamports: i128,
    #[serde(serialize_with = "integer_string")]
    pub known_reclaimed_lamports: u128,
    pub accounting_complete: bool,
}

/// Conservative extension support: no confidential/withheld balances, transfer hooks,
/// CPI guards, pausing or other mutable extension semantics are guessed.
pub fn unsupported_reason(asset: &CleanupAsset, wallet: &str) -> Option<String> {
    let account = &asset.account;
    if account.owner != wallet {
        return Some("wallet is not token owner".into());
    }
    let mut empty = account.clone();
    empty.raw_amount = 0;
    match crate::core::asset::assess_closure(&empty, wallet) {
        ClosureAssessment::PotentiallyReclaimable => {}
        ClosureAssessment::NotReclaimable(reason) | ClosureAssessment::NeedsReview(reason) => {
            return Some(reason);
        }
    }
    if !asset.kind.is_fungible() {
        return Some("NFT or unverified classification; cleanup excluded".into());
    }
    if account.decimals.is_none()
        || asset.mint.as_ref().is_none_or(|mint| {
            mint.mint != account.mint
                || mint.program != account.program
                || Some(mint.decimals) != account.decimals
        })
    {
        return Some("mint/decimals unavailable or inconsistent".into());
    }
    if asset.mint.as_ref().is_some_and(|mint| {
        mint.extension_types.iter().any(|ext| {
            !matches!(
                ext.as_str(),
                "MetadataPointer"
                    | "TokenMetadata"
                    | "MintCloseAuthority"
                    | "GroupPointer"
                    | "TokenGroup"
                    | "GroupMemberPointer"
                    | "TokenGroupMember"
            )
        })
    }) {
        return Some("unsupported mint extension".into());
    }
    if !matches!(account.state.as_str(), "Initialized" | "Frozen") {
        return Some("uninitialized account".into());
    }
    if account.raw_amount > 0 && account.state == "Frozen" {
        return Some("frozen balance cannot be swapped or burned".into());
    }
    if account.raw_amount > 0
        && (account.is_native || account.mint == crate::core::asset::WRAPPED_SOL)
    {
        return Some("nonempty WSOL requires explicit unwrap; never burn native SOL".into());
    }
    None
}

/// Selection is saved in the approved plan and checked again at execution.
/// Mint protection wins over account selection, including empty accounts.
#[derive(Debug, Clone, Default, Serialize)]
pub struct CleanupSelection {
    pub accounts: std::collections::BTreeSet<String>,
    pub ignored_mints: std::collections::BTreeSet<String>,
}
impl CleanupSelection {
    pub fn skip_reason(&self, account: &TokenAccount) -> Option<&'static str> {
        if self.ignored_mints.contains(&account.mint) {
            Some("mint protected by cleanup exclusion")
        } else if !self.accounts.is_empty() && !self.accounts.contains(&account.address) {
            Some("account outside cleanup selection")
        } else {
            None
        }
    }
    pub fn protects_output(&self) -> bool {
        self.ignored_mints.contains(WRAPPED_SOL)
    }
}
