use crate::error::concise_reason;
use dock_flints_core::core;
use serde::Serialize;

/// Independent IPC scanner status; incomplete categories retain a bounded diagnostic.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", content = "reason", rename_all = "camelCase")]
pub enum ScanStatusDto {
    Complete,
    Partial(String),
    Unsupported(String),
    Failed(String),
    Skipped(String),
}

impl From<core::ScanStatus> for ScanStatusDto {
    fn from(status: core::ScanStatus) -> Self {
        match status {
            core::ScanStatus::Complete => Self::Complete,
            core::ScanStatus::Partial(reason) => Self::Partial(concise_reason(&reason)),
            core::ScanStatus::Unsupported(reason) => Self::Unsupported(concise_reason(&reason)),
            core::ScanStatus::Failed(reason) => Self::Failed(concise_reason(&reason)),
            core::ScanStatus::Skipped(reason) => Self::Skipped(concise_reason(&reason)),
        }
    }
}

/// Distinguish a successful empty inventory from an unavailable inventory (`items: null`).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetListDto<T> {
    pub status: ScanStatusDto,
    pub items: Option<Vec<T>>,
}

impl<T> AssetListDto<T> {
    pub(crate) fn from_scan(status: core::ScanStatus, items: Vec<T>) -> Self {
        let available = matches!(
            status,
            core::ScanStatus::Complete | core::ScanStatus::Partial(_)
        );
        Self {
            status: status.into(),
            items: available.then_some(items),
        }
    }
}

/// Program identity is preserved separately from asset classification.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum TokenProgramDto {
    Legacy,
    Token2022,
}

impl From<core::TokenProgram> for TokenProgramDto {
    fn from(program: core::TokenProgram) -> Self {
        match program {
            core::TokenProgram::Legacy => Self::Legacy,
            core::TokenProgram::Token2022 => Self::Token2022,
        }
    }
}

/// Preserve every core classification, including unclassified token-account assets.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AssetKindDto {
    Fungible,
    FungibleAsset,
    NonFungible,
    NonFungibleEdition,
    ProgrammableNonFungible,
    ProgrammableNonFungibleEdition,
    Unknown,
}

impl From<core::AssetKind> for AssetKindDto {
    fn from(kind: core::AssetKind) -> Self {
        match kind {
            core::AssetKind::Fungible => Self::Fungible,
            core::AssetKind::FungibleAsset => Self::FungibleAsset,
            core::AssetKind::NonFungible => Self::NonFungible,
            core::AssetKind::NonFungibleEdition => Self::NonFungibleEdition,
            core::AssetKind::ProgrammableNonFungible => Self::ProgrammableNonFungible,
            core::AssetKind::ProgrammableNonFungibleEdition => Self::ProgrammableNonFungibleEdition,
            core::AssetKind::Unknown => Self::Unknown,
        }
    }
}

/// Optional on-chain labels are display data; mint/address fields remain asset identity.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataDto {
    pub name: Option<String>,
    pub symbol: Option<String>,
    pub uri: Option<String>,
    pub image_uri: Option<String>,
    pub token_standard: Option<String>,
    pub source: Option<String>,
    pub collection: Option<CollectionDto>,
}

impl From<core::TokenMetadata> for MetadataDto {
    fn from(metadata: core::TokenMetadata) -> Self {
        Self {
            name: metadata.name,
            symbol: metadata.symbol,
            uri: metadata.uri,
            image_uri: metadata.image_uri,
            token_standard: metadata.token_standard,
            source: metadata.source,
            collection: metadata.collection.map(Into::into),
        }
    }
}

/// Preserve collection verification without treating an unverified label as ownership proof.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectionDto {
    pub address: String,
    pub verified: bool,
}

impl From<core::CollectionInfo> for CollectionDto {
    fn from(collection: core::CollectionInfo) -> Self {
        Self {
            address: collection.address,
            verified: collection.verified,
        }
    }
}

/// Approximate USD prices use floats; their exact blockchain block ID remains a string.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PriceDto {
    pub usd: f64,
    pub source: String,
    pub block_id: Option<String>,
    pub decimals: u8,
}

impl From<core::Price> for PriceDto {
    fn from(price: core::Price) -> Self {
        Self {
            usd: price.usd,
            source: price.source,
            block_id: price.block_id.map(|block| block.to_string()),
            decimals: price.decimals,
        }
    }
}

/// A token holding in the aggregated token view or the per-account all-token view.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenAssetDto {
    pub mint: String,
    pub program: TokenProgramDto,
    pub total_raw_amount: String,
    pub decimals: Option<u8>,
    pub balance: Option<String>,
    pub accounts: Vec<String>,
    pub kind: AssetKindDto,
    pub metadata: MetadataDto,
    pub price: Option<PriceDto>,
    pub value_usd: Option<f64>,
}

impl From<core::TokenAsset> for TokenAssetDto {
    fn from(asset: core::TokenAsset) -> Self {
        Self {
            mint: asset.mint,
            program: asset.program.into(),
            total_raw_amount: asset.total_raw_amount.to_string(),
            decimals: asset.decimals,
            balance: asset.balance,
            accounts: asset.accounts,
            kind: asset.kind.into(),
            metadata: asset.metadata.into(),
            price: asset.price.map(Into::into),
            value_usd: asset.value_usd,
        }
    }
}

/// Read-only closure assessment; this DTO grants no permission to close an account.
#[derive(Debug, Serialize)]
#[serde(tag = "status", content = "reason", rename_all = "camelCase")]
pub enum ClosureDto {
    PotentiallyReclaimable,
    NotReclaimable(String),
    NeedsReview(String),
}

impl From<core::ClosureAssessment> for ClosureDto {
    fn from(closure: core::ClosureAssessment) -> Self {
        match closure {
            core::ClosureAssessment::PotentiallyReclaimable => Self::PotentiallyReclaimable,
            core::ClosureAssessment::NotReclaimable(reason) => {
                Self::NotReclaimable(concise_reason(&reason))
            }
            core::ClosureAssessment::NeedsReview(reason) => {
                Self::NeedsReview(concise_reason(&reason))
            }
        }
    }
}

/// Normalized token-account details with all exact amounts serialized as decimal strings.
/// Only extension names cross IPC; arbitrary extension JSON is not a precision-safe contract.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenAccountDto {
    pub address: String,
    pub mint: String,
    pub program: TokenProgramDto,
    pub program_id: String,
    pub owner: String,
    pub raw_amount: String,
    pub decimals: Option<u8>,
    pub lamports: String,
    pub data_len: String,
    pub state: String,
    pub delegate: Option<String>,
    pub delegated_amount: String,
    pub close_authority: Option<String>,
    pub is_native: bool,
    pub native_reserve_lamports: Option<String>,
    pub extension_types: Vec<String>,
    pub closure: ClosureDto,
}

impl From<core::TokenAccount> for TokenAccountDto {
    fn from(account: core::TokenAccount) -> Self {
        Self {
            address: account.address,
            mint: account.mint,
            program: account.program.into(),
            program_id: account.program_id,
            owner: account.owner,
            raw_amount: account.raw_amount.to_string(),
            decimals: account.decimals,
            lamports: account.lamports.to_string(),
            data_len: account.data_len.to_string(),
            state: account.state,
            delegate: account.delegate,
            delegated_amount: account.delegated_amount.to_string(),
            close_authority: account.close_authority,
            is_native: account.is_native,
            native_reserve_lamports: account
                .native_reserve_lamports
                .map(|value| value.to_string()),
            extension_types: account.extension_types,
            closure: account.closure.into(),
        }
    }
}

/// Verified mint details with exact supply and optional authorities.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MintDto {
    pub mint: String,
    pub program: TokenProgramDto,
    pub decimals: u8,
    pub supply: String,
    pub mint_authority: Option<String>,
    pub freeze_authority: Option<String>,
    pub metadata: MetadataDto,
    pub extension_types: Vec<String>,
}

impl From<core::MintInfo> for MintDto {
    fn from(mint: core::MintInfo) -> Self {
        Self {
            mint: mint.mint,
            program: mint.program.into(),
            decimals: mint.decimals,
            supply: mint.supply.to_string(),
            mint_authority: mint.mint_authority,
            freeze_authority: mint.freeze_authority,
            metadata: mint.metadata.into(),
            extension_types: mint.extension_types,
        }
    }
}

/// A classic or programmable NFT, retaining the core's verification evidence.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NftDto {
    pub mint: String,
    pub token_accounts: Vec<String>,
    pub metadata: MetadataDto,
    pub programmable: bool,
    pub edition: bool,
    pub evidence: String,
}

impl From<core::NftAsset> for NftDto {
    fn from(nft: core::NftAsset) -> Self {
        Self {
            mint: nft.mint,
            token_accounts: nft.token_accounts,
            metadata: nft.metadata.into(),
            programmable: nft.programmable,
            edition: nft.edition,
            evidence: nft.evidence,
        }
    }
}

/// MPL Core asset data remains separate from mint-backed NFTs.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CoreAssetDto {
    pub address: String,
    pub owner: String,
    pub name: String,
    pub uri: String,
    pub lamports: String,
    pub data_len: String,
    pub update_authority: String,
    pub collection: Option<CollectionDto>,
    pub plugins_status: ScanStatusDto,
}

impl From<core::CoreAsset> for CoreAssetDto {
    fn from(asset: core::CoreAsset) -> Self {
        Self {
            address: asset.address,
            owner: asset.owner,
            name: asset.name,
            uri: asset.uri,
            lamports: asset.lamports.to_string(),
            data_len: asset.data_len.to_string(),
            update_authority: asset.update_authority,
            collection: asset.collection.map(Into::into),
            plugins_status: asset.plugins_status.into(),
        }
    }
}

/// Classic and Core categories keep independent status instead of hiding partial NFT reads.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NftsDto {
    pub status: ScanStatusDto,
    pub classic: AssetListDto<NftDto>,
    pub core: AssetListDto<CoreAssetDto>,
}

/// Owner-verified DAS inventory; missing/failed discovery remains null, partial pages remain usable.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompressedNftsDto {
    pub status: ScanStatusDto,
    pub items: Option<Vec<dock_flints_core::core::categories::CompressedAsset>>,
}

/// Unrecognized assets retain addresses and diagnostics, not untyped raw account blobs.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnknownAssetDto {
    pub address: String,
    pub program_id: Option<String>,
    pub lamports: Option<String>,
    pub reason: String,
}

impl From<core::UnknownAsset> for UnknownAssetDto {
    fn from(asset: core::UnknownAsset) -> Self {
        Self {
            address: asset.address,
            program_id: asset.program_id,
            lamports: asset.lamports.map(|value| value.to_string()),
            reason: concise_reason(&asset.reason),
        }
    }
}

/// Counts describe returned inventory; lamport totals are exact strings, not JS numbers.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountSummaryDto {
    pub token_accounts: usize,
    pub empty_token_accounts: usize,
    pub token_account_lamports: String,
    pub potentially_reclaimable_lamports: String,
    pub closure_review_accounts: usize,
}

impl From<core::AccountSummary> for AccountSummaryDto {
    fn from(summary: core::AccountSummary) -> Self {
        Self {
            token_accounts: summary.token_accounts,
            empty_token_accounts: summary.empty_token_accounts,
            token_account_lamports: summary.token_account_lamports.to_string(),
            potentially_reclaimable_lamports: summary.potentially_reclaimable_lamports.to_string(),
            closure_review_accounts: summary.closure_review_accounts,
        }
    }
}
