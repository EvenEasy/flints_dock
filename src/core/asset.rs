use super::error::ScanStatus;
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;

// Addresses are validated at RPC/decoder boundaries and exposed as base58 strings in JSON.
// Raw integer strings preserve precision for JSON consumers (including JavaScript).
pub fn integer_string<T: ToString, S: serde::Serializer>(
    value: &T,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&value.to_string())
}

/// Distinguish legacy SPL accounts from Token-2022 accounts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TokenProgram {
    Legacy,
    Token2022,
}

impl TokenProgram {
    pub fn id(self) -> solana_pubkey::Pubkey {
        match self {
            Self::Legacy => spl_token_interface::ID,
            Self::Token2022 => spl_token_2022_interface::ID,
        }
    }
}

/// Retain raw balance, authorities, extensions and closure eligibility for one account.
#[derive(Debug, Clone, Serialize)]
pub struct TokenAccount {
    pub address: String,
    pub mint: String,
    pub program: TokenProgram,
    pub program_id: String,
    pub owner: String,
    #[serde(serialize_with = "integer_string")]
    pub raw_amount: u64,

    // A missing/undecodable mint must never manufacture a decimal count.
    pub decimals: Option<u8>,
    pub lamports: u64,
    pub data_len: usize,
    pub state: String,
    pub delegate: Option<String>,
    pub delegated_amount: u64,
    pub close_authority: Option<String>,
    pub is_native: bool,
    pub native_reserve_lamports: Option<u64>,
    pub extensions: Value,
    pub extension_types: Vec<String>,
    pub closure: ClosureAssessment,
}

impl TokenAccount {
    pub fn is_empty(&self) -> bool {
        self.raw_amount == 0
    }
}

/// Explain whether the wallet can potentially reclaim an account balance.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", content = "reason", rename_all = "snake_case")]
pub enum ClosureAssessment {
    PotentiallyReclaimable,
    NotReclaimable(String),
    NeedsReview(String),
}

/// Store optional labels and collection information without using them as asset identity.
#[derive(Debug, Clone, Default, Serialize)]
pub struct TokenMetadata {
    pub name: Option<String>,
    pub symbol: Option<String>,
    pub uri: Option<String>,
    pub image_uri: Option<String>,
    pub token_standard: Option<String>,
    pub source: Option<String>,
    pub collection: Option<CollectionInfo>,
}

/// Keep verified mint decimals, supply, authorities and extension information.
#[derive(Debug, Clone, Serialize)]
pub struct MintInfo {
    pub mint: String,
    pub program: TokenProgram,
    pub decimals: u8,
    #[serde(serialize_with = "integer_string")]
    pub supply: u64,
    pub mint_authority: Option<String>,
    pub freeze_authority: Option<String>,
    pub metadata: TokenMetadata,
    pub extensions: Value,
    pub extension_types: Vec<String>,
}

/// Describe fungible and NFT semantics established by on-chain classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetKind {
    Fungible,
    FungibleAsset,
    NonFungible,
    NonFungibleEdition,
    ProgrammableNonFungible,
    ProgrammableNonFungibleEdition,
    Unknown,
}

impl AssetKind {
    pub fn is_fungible(self) -> bool {
        matches!(self, Self::Fungible | Self::FungibleAsset)
    }
    pub fn is_nft(self) -> bool {
        matches!(
            self,
            Self::NonFungible
                | Self::NonFungibleEdition
                | Self::ProgrammableNonFungible
                | Self::ProgrammableNonFungibleEdition
        )
    }
    pub fn is_programmable(self) -> bool {
        matches!(
            self,
            Self::ProgrammableNonFungible | Self::ProgrammableNonFungibleEdition
        )
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Fungible => "Fungible",
            Self::FungibleAsset => "FungibleAsset",
            Self::NonFungible => "NFT",
            Self::NonFungibleEdition => "NFT edition",
            Self::ProgrammableNonFungible => "Programmable",
            Self::ProgrammableNonFungibleEdition => "Programmable edition",
            Self::Unknown => "Unknown",
        }
    }
}

/// Represent a classified token holding while retaining its backing account addresses.
#[derive(Debug, Clone, Serialize)]
pub struct TokenAsset {
    pub mint: String,
    pub program: TokenProgram,
    #[serde(serialize_with = "integer_string")]
    pub total_raw_amount: u128,
    pub decimals: Option<u8>,
    pub balance: Option<String>,
    pub accounts: Vec<String>,
    pub kind: AssetKind,
    pub metadata: TokenMetadata,
    pub price: Option<Price>,
    pub value_usd: Option<f64>,
}

/// Keep native SOL holdings separate from optional USD estimates.
#[derive(Debug, Serialize)]
pub struct NativeBalance {
    pub lamports: u64,
    pub price: Option<Price>,
    pub value_usd: Option<f64>,
}

/// Describe a verified classic or programmable NFT and its backing accounts.
#[derive(Debug, Clone, Serialize)]
pub struct NftAsset {
    pub mint: String,
    pub token_accounts: Vec<String>,
    pub metadata: TokenMetadata,
    pub programmable: bool,
    pub edition: bool,
    pub evidence: String,
}

/// Retain the decoded MPL Core asset and its collection and plugin discovery status.
#[derive(Debug, Serialize)]
pub struct CoreAsset {
    pub address: String,
    pub owner: String,
    pub name: String,
    pub uri: String,
    pub lamports: u64,
    pub data_len: usize,
    pub update_authority: String,
    pub collection: Option<CollectionInfo>,
    pub plugins_status: ScanStatus,
}

/// Preserve unrecognized or undecodable data with an explicit explanation.
#[derive(Debug, Clone, Serialize)]
pub struct UnknownAsset {
    pub address: String,
    pub program_id: Option<String>,
    pub lamports: Option<u64>,
    pub reason: String,
    pub data: Option<Value>,
}

/// Represent an approximate unit price with its source and expected token decimals.
#[derive(Debug, Clone, Serialize)]
pub struct Price {
    // Prices and valuations are approximate; canonical holdings remain integers.
    pub usd: f64,
    pub source: String,
    pub block_id: Option<u64>,
    pub decimals: u8,
}

/// Return available mint prices alongside the provider lookup status.
#[derive(Debug, Serialize)]
pub struct PriceReport {
    pub status: ScanStatus,
    pub quotes: BTreeMap<String, Price>,
}

/// Expose raw account counts and conditional lamport recovery estimates.
#[derive(Debug, Serialize)]
pub struct AccountSummary {
    pub token_accounts: usize,
    pub empty_token_accounts: usize,
    #[serde(serialize_with = "integer_string")]
    pub token_account_lamports: u128,
    #[serde(serialize_with = "integer_string")]
    pub potentially_reclaimable_lamports: u128,
    pub closure_review_accounts: usize,
}

/// Associate verified mint metadata with optional NFT evidence.
#[derive(Debug, Serialize)]
pub struct MetadataRecord {
    pub mint: String,
    pub metadata: TokenMetadata,
    pub nft: Option<NftAsset>,
}

/// Preserve the collection address and whether its relationship was verified.
#[derive(Debug, Clone, Serialize)]
pub struct CollectionInfo {
    pub address: String,
    pub verified: bool,
}

pub const WRAPPED_SOL: &str = "So11111111111111111111111111111111111111112";

/// Return whether the wallet may reclaim an empty account under supported token rules.
/// Checks effective close authority and extensions; nonempty WSOL is not counted as recoverable
/// rent.
pub fn assess_closure(account: &TokenAccount, wallet: &str) -> ClosureAssessment {
    if account.close_authority.as_deref().unwrap_or(&account.owner) != wallet {
        return ClosureAssessment::NotReclaimable("Wallet is not the close authority".into());
    }
    if account.raw_amount != 0 {
        return ClosureAssessment::NotReclaimable(if account.is_native {
            "Wrapped SOL principal is not empty-account rent; closing unwraps SOL".into()
        } else {
            "Nonzero token balance".into()
        });
    }
    if account
        .extension_types
        .iter()
        .any(|kind| kind != "ImmutableOwner")
    {
        return ClosureAssessment::NeedsReview(
            "Token-2022 extensions may restrict closure or contain withheld/confidential balances"
                .into(),
        );
    }
    ClosureAssessment::PotentiallyReclaimable
}
