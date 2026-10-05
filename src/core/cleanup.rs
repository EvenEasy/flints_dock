use crate::{
    core::swap::{SwapLimits, SwapQuote},
    core::*,
};
use serde::Serialize;
use std::time::Duration;

/// Classify an account into a close, swap, burn or skip path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum CleanupCategory {
    Empty,
    Swappable,
    Burnable,
    Unsupported,
}

/// Keep each backing account together with its verified mint and asset classification.
#[derive(Debug, Clone, Serialize)]
pub struct CleanupAsset {
    pub account: TokenAccount,
    pub mint: Option<MintInfo>,
    pub kind: AssetKind,
}

/// Record the proposed action, explanation and optional quote for one account.
#[derive(Debug, Clone, Serialize)]
pub struct CleanupEntry {
    pub asset: CleanupAsset,
    pub category: CleanupCategory,
    pub reason: String,
    pub quote: Option<SwapQuote>,
}

/// Bind the wallet, selected accounts, action estimates and discovery status to a preview.
#[derive(Debug, Serialize)]
pub struct CleanupPlan {
    pub wallet: String,
    pub discovery_status: ScanStatus,
    pub entries: Vec<CleanupEntry>,
    pub unparsed_accounts: Vec<UnknownAsset>,
    pub summary: CleanupSummary,
    pub selection: CleanupSelection,
}

/// Count planned actions and total estimated swap output and recoverable lamports.
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

/// Configure account selection, quote retries and transaction execution limits.
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

/// Identify the individual on-chain action recorded in a cleanup receipt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum CleanupOperation {
    Swap,
    Burn,
    Close,
}

/// Keep a confirmed signature and optional balance accounting for one operation.
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

/// Preserve the outcome and any completed operations for a single account.
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

/// Summarize completed, failed and skipped accounts with observed balance changes.
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

/// Return the first reason an account cannot safely follow the supported cleanup flow.
/// Requires verified fungible semantics, matching decimals and supported extensions.
/// Frozen nonempty balances and native-backed balances are excluded.
pub fn unsupported_reason(asset: &CleanupAsset, wallet: &str) -> Option<String> {
    let account = &asset.account;
    if account.owner != wallet {
        return Some("wallet is not token owner".into());
    }

    // Assess eventual closure authority independently of the current nonzero token balance.
    let mut empty = account.clone();
    empty.raw_amount = 0;
    match crate::core::asset::assess_closure(&empty, wallet) {
        ClosureAssessment::PotentiallyReclaimable => {}
        ClosureAssessment::NotReclaimable(reason) | ClosureAssessment::NeedsReview(reason) => {
            return Some(reason);
        }
    }

    // Exclude NFTs and uncertain classifications from automatic liquidation.
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

    // Allow only extensions whose cleanup semantics are explicitly supported.
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
    /// Return the selection rule that excludes an account, if any.
    /// Protected mints take precedence over the optional account allowlist, including for empty
    /// accounts.
    pub fn skip_reason(&self, account: &TokenAccount) -> Option<&'static str> {
        if self.ignored_mints.contains(&account.mint) {
            Some("mint protected by cleanup exclusion")
        } else if !self.accounts.is_empty() && !self.accounts.contains(&account.address) {
            Some("account outside cleanup selection")
        } else {
            None
        }
    }

    /// Report whether WSOL is protected, preventing swaps that could implicitly unwrap its ATA.
    pub fn protects_output(&self) -> bool {
        self.ignored_mints.contains(WRAPPED_SOL)
    }
}
