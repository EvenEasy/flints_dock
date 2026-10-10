use crate::{
    core::swap::{SwapLimits, SwapQuote},
    core::*,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Destruction intent is separate from account selection and is saved in the server plan.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CleanupPolicy {
    #[default]
    Auto,
    ExplicitDiscard,
}

/// Stable explanations for planner decisions; UI never parses provider error prose.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CleanupReasonCode {
    EmptyAccount,
    SwapRoute,
    NoRoute,
    ExplicitDiscard,
    IgnoredMint,
    NotSelected,
    OwnerMismatch,
    CloseAuthorityMismatch,
    UnsupportedAccountExtension,
    UnsupportedMintExtension,
    InvalidState,
    Frozen,
    NativeBalance,
    UnknownClassification,
    NftUnsupported,
    MintUnavailable,
    RoutingUnavailable,
    ProviderFailure,
    InsufficientLiquidity,
    PriceImpact,
    QuoteExpired,
    Undecodable,
}

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
    pub reason_code: CleanupReasonCode,
    pub quote: Option<SwapQuote>,
}

/// Bind the wallet, selected accounts, action estimates and discovery status to a preview.
#[derive(Debug, Clone, Serialize)]
pub struct CleanupPlan {
    pub wallet: String,
    pub discovery_status: ScanStatus,
    pub entries: Vec<CleanupEntry>,
    pub unparsed_accounts: Vec<UnknownAsset>,
    pub summary: CleanupSummary,
    pub selection: CleanupSelection,
    pub policy: CleanupPolicy,
}

/// Count planned actions and total estimated swap output and recoverable lamports.
#[derive(Debug, Clone, Default, Serialize)]
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
    pub policy: CleanupPolicy,
    pub selection: CleanupSelection,
    pub swap_limits: SwapLimits,
    pub slippage_bps: u16,
    pub quote_interval: Duration,
    pub quote_attempts: usize,
}
impl Default for CleanupOptions {
    fn default() -> Self {
        Self {
            policy: CleanupPolicy::Auto,
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
#[derive(Debug, Clone, Serialize)]
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
#[derive(Debug, Clone, Default, Serialize)]
pub struct CleanupReport {
    pub results: Vec<AccountCleanupResult>,
    pub closed: usize,
    pub failed: usize,
    pub skipped: usize,
    #[serde(serialize_with = "integer_string")]
    pub known_swap_net_lamports: i128,
    #[serde(serialize_with = "integer_string")]
    pub known_reclaimed_lamports: u128,
    /// Signed wallet delta across every observed job transaction, including standalone fees.
    #[serde(serialize_with = "integer_string")]
    pub known_net_wallet_lamports: i128,
    pub accounting_complete: bool,
}

/// Return the first reason an account cannot safely follow the supported cleanup flow.
/// Nonempty liquidation requires verified fungible semantics, decimals and supported extensions.
/// Empty closure only requires valid account state, ownership, authority and supported extensions.
/// Frozen nonempty balances and native-backed balances are excluded.
pub fn eligibility_issue(
    asset: &CleanupAsset,
    wallet: &str,
) -> Option<(CleanupReasonCode, String)> {
    use CleanupReasonCode::*;
    let account = &asset.account;
    let issue = |code, reason: &str| Some((code, reason.to_owned()));
    if account.owner != wallet {
        return issue(OwnerMismatch, "Wallet is not the token owner");
    }
    if account.program_id != account.program.id().to_string() {
        return issue(InvalidState, "Token program identity is inconsistent");
    }
    if account.close_authority.as_deref().unwrap_or(&account.owner) != wallet {
        return issue(CloseAuthorityMismatch, "Wallet is not the close authority");
    }
    if !matches!(account.state.as_str(), "Initialized" | "Frozen") {
        return issue(InvalidState, "Account is not initialized");
    }
    if account
        .extension_types
        .iter()
        .any(|ext| ext != "ImmutableOwner")
    {
        return issue(
            UnsupportedAccountExtension,
            "Account extensions require an unsupported cleanup flow",
        );
    }

    // Closing a decoded empty account does not depend on mint supply, metadata or tradability.
    // In particular, burning the last unit must not make its now-empty source uncloseable.
    if account.raw_amount == 0 {
        return None;
    }
    if account.state == "Frozen" {
        return issue(Frozen, "Frozen balance cannot be swapped or burned");
    }
    if account.is_native || account.mint == crate::core::asset::WRAPPED_SOL {
        return issue(
            NativeBalance,
            "WSOL requires explicit unwrap; native SOL cannot be burned",
        );
    }
    if asset.kind.is_nft() {
        return issue(
            NftUnsupported,
            "NFT liquidation is unsupported; generic token burn is excluded",
        );
    }
    if !asset.kind.is_fungible() {
        return issue(
            UnknownClassification,
            if asset
                .mint
                .as_ref()
                .is_some_and(|mint| mint.decimals == 0 && mint.supply <= 1)
            {
                "Zero-decimal single-unit mint needs TokenStandard or verified legacy edition evidence; labels are insufficient"
            } else {
                "Classification is unknown; verified fungible evidence is required"
            },
        );
    }
    if account.decimals.is_none()
        || asset.mint.as_ref().is_none_or(|mint| {
            mint.mint != account.mint
                || mint.program != account.program
                || Some(mint.decimals) != account.decimals
                || account.raw_amount > mint.supply
        })
    {
        return issue(
            MintUnavailable,
            "Mint, supply or decimals are unavailable or inconsistent",
        );
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
        return issue(
            UnsupportedMintExtension,
            "Mint extensions require an unsupported cleanup flow",
        );
    }
    None
}

/// CLI-compatible prose wrapper around structured, operation-aware eligibility.
pub fn unsupported_reason(asset: &CleanupAsset, wallet: &str) -> Option<String> {
    eligibility_issue(asset, wallet).map(|(_, reason)| reason)
}

/// Selection is saved in the approved plan and checked again at execution.
/// Mint protection wins over account selection, including empty accounts.
#[derive(Debug, Clone, Default, Serialize)]
pub struct CleanupSelection {
    /// IPC NONE is explicit; the CLI empty account allowlist still means ALL.
    pub none: bool,
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
        } else if self.none
            || (!self.accounts.is_empty() && !self.accounts.contains(&account.address))
        {
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
