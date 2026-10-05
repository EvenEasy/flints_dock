use super::assets::*;
use crate::error::AppError;
use dock_flints_core::core::{self, ScanOptions, ScanSelection, ScanStatus, WalletSnapshot};
use serde::{Deserialize, Serialize};
use solana_pubkey::Pubkey;
use std::collections::BTreeMap;

/// Structured equivalents of CLI scan selectors; omitted/empty selection means `--all`.
/// `allTokens` remains opt-in even when `all` is true, matching existing core semantics.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct CategorySelectionDto {
    pub balance: bool,
    pub tokens: bool,
    pub all_tokens: bool,
    pub nfts: bool,
    pub cnfts: bool,
    pub all: bool,
}

impl From<CategorySelectionDto> for ScanSelection {
    fn from(selection: CategorySelectionDto) -> Self {
        let categories = Self {
            balance: selection.balance,
            tokens: selection.tokens,
            all_tokens: selection.all_tokens,
            nfts: selection.nfts,
            cnfts: selection.cnfts,
        };
        if selection.all || categories.is_empty() {
            Self {
                all_tokens: selection.all_tokens,
                ..Self::ALL
            }
        } else {
            categories
        }
    }
}

/// Public-key-only read request. Unknown fields (including signing inputs) are rejected.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AnalyzeWalletRequestDto {
    pub wallet_address: String,
    #[serde(default)]
    pub selection: CategorySelectionDto,
    #[serde(default)]
    pub no_prices: bool,
}

impl AnalyzeWalletRequestDto {
    /// Validate the required public key and return the existing core scan options.
    pub fn into_core(self) -> Result<(Pubkey, ScanOptions), AppError> {
        let owner = self
            .wallet_address
            .parse()
            .map_err(|_| AppError::invalid_wallet())?;
        Ok((
            owner,
            ScanOptions {
                selection: self.selection.into(),
                no_prices: self.no_prices,
            },
        ))
    }
}

/// Echo the expanded category selection so frontend consumers can distinguish omitted views.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectedCategoriesDto {
    pub balance: bool,
    pub tokens: bool,
    pub all_tokens: bool,
    pub nfts: bool,
    pub cnfts: bool,
}

impl From<ScanSelection> for SelectedCategoriesDto {
    fn from(selection: ScanSelection) -> Self {
        Self {
            balance: selection.balance,
            tokens: selection.tokens,
            all_tokens: selection.all_tokens,
            nfts: selection.nfts,
            cnfts: selection.cnfts,
        }
    }
}

/// Native SOL balance preserves exact lamports and the full decimal SOL representation.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeBalanceDto {
    pub lamports: String,
    pub sol: String,
    pub price: Option<PriceDto>,
    pub value_usd: Option<f64>,
}

impl From<core::NativeBalance> for NativeBalanceDto {
    fn from(balance: core::NativeBalance) -> Self {
        Self {
            lamports: balance.lamports.to_string(),
            sol: core::amount::exact_amount(u128::from(balance.lamports), 9),
            price: balance.price.map(Into::into),
            value_usd: balance.value_usd,
        }
    }
}

/// Selected native balance category; a failed read has a status but no fabricated balance.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BalanceDto {
    pub status: ScanStatusDto,
    pub value: Option<NativeBalanceDto>,
}

/// Frontend wallet snapshot, independent of core models and terminal JSON formatting.
/// Unselected categories are null; selected failures carry explicit scanner status.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WalletAnalysisDto {
    pub owner: String,
    pub commitment: String,
    pub selected: SelectedCategoriesDto,
    pub has_usable_results: bool,
    pub balance: Option<BalanceDto>,
    pub tokens: Option<AssetListDto<TokenAssetDto>>,
    pub all_tokens: Option<AssetListDto<TokenAssetDto>>,
    pub nfts: Option<NftsDto>,
    pub cnfts: Option<CompressedNftsDto>,
    pub token_accounts: Option<Vec<TokenAccountDto>>,
    pub mints: Option<Vec<MintDto>>,
    pub unknown_assets: Vec<UnknownAssetDto>,
    pub account_summary: Option<AccountSummaryDto>,
    pub scanners: BTreeMap<String, ScanStatusDto>,
}

impl From<WalletSnapshot> for WalletAnalysisDto {
    fn from(snapshot: WalletSnapshot) -> Self {
        let has_usable_results = snapshot.has_usable_results();
        let selected = snapshot.selected;
        let status = |name: &str| {
            snapshot
                .scanners
                .get(name)
                .cloned()
                .unwrap_or_else(|| ScanStatus::Skipped("Scanner did not run".into()))
        };

        // A failed discovery must not look like a successfully empty account inventory.
        let accounts_available = selected.needs_token_accounts()
            && [
                "legacy_tokens",
                "token_2022",
                "tokens",
                "all_tokens",
                "classic_nfts",
            ]
            .iter()
            .any(|name| {
                matches!(
                    snapshot.scanners.get(*name),
                    Some(ScanStatus::Complete | ScanStatus::Partial(_))
                )
            });
        let mints_available = accounts_available
            && snapshot.scanners.get("mint_metadata").is_none_or(|status| {
                matches!(status, ScanStatus::Complete | ScanStatus::Partial(_))
            });

        // Project existing category results without applying new filtering or classification.
        let balance = selected.balance.then(|| BalanceDto {
            status: status("native_sol").into(),
            value: snapshot.native_sol.map(Into::into),
        });
        let tokens = selected.tokens.then(|| {
            AssetListDto::from_scan(
                status("tokens"),
                snapshot.tokens.into_iter().map(Into::into).collect(),
            )
        });
        let all_tokens = selected.all_tokens.then(|| {
            AssetListDto::from_scan(
                status("all_tokens"),
                snapshot.all_tokens.into_iter().map(Into::into).collect(),
            )
        });
        let nfts = selected.nfts.then(|| NftsDto {
            status: status("nfts").into(),
            classic: AssetListDto::from_scan(
                status("classic_nfts"),
                snapshot.classic_nfts.into_iter().map(Into::into).collect(),
            ),
            core: AssetListDto::from_scan(
                status("core_asset_v1"),
                snapshot.core_assets.into_iter().map(Into::into).collect(),
            ),
        });
        let cnfts = selected.cnfts.then(|| CompressedNftsDto {
            status: status("compressed_nfts").into(),
            items: (),
        });

        // Expose exact diagnostics only for inventory that was actually available.
        Self {
            owner: snapshot.owner,
            commitment: snapshot.commitment,
            selected: selected.into(),
            has_usable_results,
            balance,
            tokens,
            all_tokens,
            nfts,
            cnfts,
            token_accounts: accounts_available.then(|| {
                snapshot
                    .token_accounts
                    .into_iter()
                    .map(Into::into)
                    .collect()
            }),
            mints: mints_available.then(|| snapshot.mints.into_iter().map(Into::into).collect()),
            unknown_assets: snapshot
                .unknown_assets
                .into_iter()
                .map(Into::into)
                .collect(),
            account_summary: accounts_available.then(|| snapshot.account_summary.into()),
            scanners: snapshot
                .scanners
                .into_iter()
                .map(|(name, status)| (name, status.into()))
                .collect(),
        }
    }
}
