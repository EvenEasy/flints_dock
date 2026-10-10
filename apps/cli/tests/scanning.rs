use async_trait::async_trait;
use base64::{Engine, engine::general_purpose::STANDARD};
use borsh::BorshSerialize;
use dock_flints::{
    core::amount::*,
    core::*,
    infra::jupiter::parse_quote,
    infra::solana as rpc,
    infra::solana::scan::{
        core::parse_asset,
        metadata::parse_mint,
        nft::{decode_metadata, edition_evidence, get_metadata},
        tokens::{assess_closure, parse_account},
    },
};
use mpl_token_metadata::{
    accounts::{MasterEdition, Metadata},
    types::{Key, TokenStandard},
};
use serde_json::{Value, json};
use solana_account::Account;
use solana_account_decoder::{UiAccount, UiAccountData, UiAccountEncoding};
use solana_pubkey::Pubkey;
use solana_rpc_client::{
    nonblocking::rpc_client::RpcClient,
    rpc_sender::{RpcSender, RpcTransportStats},
};
use solana_rpc_client_api::{request::RpcRequest, response::RpcKeyedAccount};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

fn aggregate_tokens(
    accounts: &[TokenAccount],
    mints: &[MintInfo],
    records: &[MetadataRecord],
) -> Vec<TokenAsset> {
    dock_flints::core::amount::aggregate_tokens(
        &dock_flints::core::classification::classify_token_accounts(accounts, mints, records),
    )
}

fn key(byte: u8) -> Pubkey {
    Pubkey::new_from_array([byte; 32])
}
fn ui(data: Vec<u8>, owner: Pubkey, lamports: u64) -> UiAccount {
    UiAccount {
        lamports,
        data: UiAccountData::Binary(STANDARD.encode(&data), UiAccountEncoding::Base64),
        owner: owner.to_string(),
        executable: false,
        rent_epoch: 0,
        space: Some(data.len() as u64),
    }
}
fn raw_token(mint: Pubkey, owner: Pubkey, amount: u64) -> Vec<u8> {
    let mut data = vec![0; 165];
    data[..32].copy_from_slice(&mint.to_bytes());
    data[32..64].copy_from_slice(&owner.to_bytes());
    data[64..72].copy_from_slice(&amount.to_le_bytes());
    data[108] = 1;
    data
}
fn raw_mint(decimals: u8, supply: u64) -> Vec<u8> {
    let mut data = vec![0; 82];
    data[36..44].copy_from_slice(&supply.to_le_bytes());
    data[44] = decimals;
    data[45] = 1;
    data
}
fn token(amount: u64) -> TokenAccount {
    let keyed = RpcKeyedAccount {
        pubkey: key(3).to_string(),
        account: ui(
            raw_token(key(2), key(1), amount),
            TokenProgram::Legacy.id(),
            2_039_280,
        ),
    };
    let mut token = parse_account(&keyed, TokenProgram::Legacy, &key(1)).unwrap();
    token.decimals = Some(6);
    token
}
fn mint_info(decimals: u8, supply: u64) -> MintInfo {
    parse_mint(
        &key(2),
        &Account {
            lamports: 1,
            data: raw_mint(decimals, supply),
            owner: TokenProgram::Legacy.id(),
            executable: false,
            rent_epoch: 0,
        },
        TokenProgram::Legacy,
    )
    .unwrap()
}
fn metadata(standard: Option<TokenStandard>) -> Metadata {
    Metadata {
        key: Key::MetadataV1,
        update_authority: rpc::to_metaplex(key(1)),
        mint: rpc::to_metaplex(key(2)),
        name: "Fixture NFT\0".into(),
        symbol: "FIX".into(),
        uri: "https://example.invalid/nft.json".into(),
        seller_fee_basis_points: 0,
        creators: None,
        primary_sale_happened: false,
        is_mutable: true,
        edition_nonce: None,
        token_standard: standard,
        collection: None,
        uses: None,
        collection_details: None,
        programmable_config: None,
    }
}
fn metadata_account(data: Vec<u8>) -> Account {
    Account {
        lamports: 1,
        data,
        owner: rpc::to_rpc(mpl_token_metadata::ID),
        executable: false,
        rent_epoch: 0,
    }
}

#[test]
fn decimal_format_is_exact_at_boundaries() {
    assert_eq!(exact_amount(1_000_000_001, 9), "1.000000001");
    assert_eq!(exact_amount(u64::MAX.into(), 6), "18446744073709.551615");
    assert_eq!(exact_amount(u128::MAX, 0), u128::MAX.to_string());
    assert_eq!(exact_amount(0, 255), "0");
    assert_eq!(exact_amount(1, 255), format!("0.{}1", "0".repeat(254)));
}

#[test]
fn aggregation_preserves_accounts_zeros_and_program_boundaries() {
    let mut second = token(2_000_001);
    second.address = key(4).to_string();
    let mut other_program = token(7);
    other_program.program = TokenProgram::Token2022;
    let assets = aggregate_tokens(
        &[token(1_000_000), second, token(0), other_program],
        &[mint_info(6, 9_000_000)],
        &[],
    );
    assert_eq!(assets.len(), 2);
    assert_eq!(assets[0].balance.as_deref(), Some("3.000001"));
    assert_eq!(assets[0].accounts.len(), 3);
    assert_eq!(assets[1].total_raw_amount, 7);
    assert_eq!(
        serde_json::to_value(&assets[0]).unwrap()["total_raw_amount"],
        "3000001"
    );
    let assets = aggregate_tokens(
        &[token(u64::MAX), token(u64::MAX)],
        &[mint_info(6, u64::MAX)],
        &[],
    );
    assert_eq!(assets[0].total_raw_amount, u128::from(u64::MAX) * 2);
}

#[test]
fn closure_separates_authority_extensions_and_wrapped_principal() {
    let mut account = token(0);
    assert!(account.is_empty());
    assert!(matches!(
        assess_closure(&account, &key(1).to_string()),
        ClosureAssessment::PotentiallyReclaimable
    ));
    account.close_authority = Some(key(8).to_string());
    assert!(matches!(
        assess_closure(&account, &key(1).to_string()),
        ClosureAssessment::NotReclaimable(_)
    ));
    account.close_authority = None;
    account.extension_types = vec!["TransferFeeAmount".into()];
    assert!(matches!(
        assess_closure(&account, &key(1).to_string()),
        ClosureAssessment::NeedsReview(_)
    ));
    account.raw_amount = 50;
    account.is_native = true;
    assert!(matches!(
        assess_closure(&account, &key(1).to_string()),
        ClosureAssessment::NotReclaimable(_)
    ));
    let summary = summarize_accounts(&[token(0), token(1)]);
    assert_eq!(summary.empty_token_accounts, 1);
    assert_eq!(summary.potentially_reclaimable_lamports, 2_039_280);
    assert_eq!(summary.token_account_lamports, 4_078_560);
}

#[test]
fn token2022_variable_length_and_wrong_authority() {
    let mut data = raw_token(key(2), key(1), 0);
    data.push(2); // AccountType::Account
    data.extend_from_slice(&7_u16.to_le_bytes()); // ImmutableOwner
    data.extend_from_slice(&0_u16.to_le_bytes());
    let keyed = RpcKeyedAccount {
        pubkey: key(3).to_string(),
        account: ui(data, TokenProgram::Token2022.id(), 8_123_456),
    };
    let account = parse_account(&keyed, TokenProgram::Token2022, &key(1)).unwrap();
    assert_eq!(account.data_len, 170);
    assert_eq!(account.extension_types, ["ImmutableOwner"]);
    assert_eq!(account.lamports, 8_123_456);
    assert_eq!(account.decimals, None);
    assert!(parse_account(&keyed, TokenProgram::Token2022, &key(9)).is_err());
}

#[test]
fn pubkey_versions_roundtrip_without_casts() {
    assert_eq!(rpc::to_rpc(rpc::to_metaplex(key(222))), key(222));
    assert_eq!(
        rpc::to_rpc(mpl_token_metadata::ID).to_string(),
        mpl_token_metadata::ID.to_string()
    );
    assert_eq!(
        Pubkey::new_from_array(mpl_core::ID.to_bytes()).to_string(),
        mpl_core::ID.to_string()
    );
}

#[test]
fn metadata_and_editions_validate_discriminators_and_owners() {
    let bytes = metadata(Some(TokenStandard::NonFungible))
        .try_to_vec()
        .unwrap();
    let mut account = metadata_account(bytes);
    assert!(decode_metadata(&account, &key(2)).is_ok());
    assert!(decode_metadata(&account, &key(8)).is_err());
    account.data[0] = Key::EditionV1 as u8;
    assert!(decode_metadata(&account, &key(2)).is_err());
    let edition = MasterEdition {
        key: Key::MasterEditionV2,
        supply: 0,
        max_supply: Some(0),
    };
    let mut account = metadata_account(edition.try_to_vec().unwrap());
    assert_eq!(edition_evidence(&account), Some(false));
    account.data[0] = Key::MetadataV1 as u8;
    assert_eq!(edition_evidence(&account), None);
    account.data[0] = Key::MasterEditionV2 as u8;
    account.owner = key(55);
    assert_eq!(edition_evidence(&account), None);
}

#[test]
fn core_variable_length_base_and_plugin_tail_do_not_hide_asset() {
    let mut data = vec![mpl_core::types::Key::AssetV1 as u8];
    data.extend_from_slice(&key(1).to_bytes());
    data.push(0); // UpdateAuthority::None
    for text in ["Core fixture", "https://example.invalid/core.json"] {
        data.extend_from_slice(&(text.len() as u32).to_le_bytes());
        data.extend_from_slice(text.as_bytes());
    }
    data.push(0); // seq=None
    data.extend_from_slice(&[3, 0, 0, 0]); // unknown plugin tail doesn't change base ownership
    let account = ui(
        data,
        Pubkey::new_from_array(mpl_core::ID.to_bytes()),
        4_000_000,
    );
    let asset = parse_asset(key(6), &account, &key(1)).unwrap();
    assert_eq!(asset.name, "Core fixture");
    assert!(matches!(asset.plugins_status, ScanStatus::Unsupported(_)));
    assert!(parse_asset(key(6), &account, &key(7)).is_err());
}

#[test]
fn prices_missing_invalid_and_partial_sums_are_not_false_zeroes() {
    assert!(parse_quote(&Value::Null).is_none());
    assert!(parse_quote(&json!({"usdPrice": -1, "decimals": 6})).is_none());
    assert!(parse_quote(&json!({"usdPrice": 1, "decimals": 256})).is_none());
    let price = parse_quote(&json!({"usdPrice": 2.5, "decimals": 6, "blockId": 123})).unwrap();
    assert_eq!(approximate_value(3_000_000, 6, price.usd), Some(7.5));
    assert_eq!(sum_known_values([None, None]), None);
    assert_eq!(sum_known_values([Some(7.5), None, Some(2.5)]), Some(10.0));
    assert_eq!(sum_known_values([Some(0.0)]), Some(0.0));
}

#[test]
fn scan_status_distinguishes_empty_from_failed() {
    let mut scan = ScanCollection::<TokenAccount>::complete(vec![]);
    assert!(scan.status.is_complete());
    scan.issue("one program unavailable");
    scan.issue("mint unavailable");
    assert!(!scan.status.is_complete());
    assert!(
        matches!(scan.status, ScanStatus::Partial(message) if message.contains("mint unavailable"))
    );
    assert!(matches!(
        dock_flints::infra::solana::scan::cnft::support(),
        ScanStatus::Unsupported(_)
    ));
}

#[derive(Clone, Default)]
struct FixtureRpc {
    accounts: BTreeMap<String, UiAccount>,
    calls: Arc<Mutex<Vec<(RpcRequest, Value)>>>,
    fail_core: bool,
    fail_2022: bool,
    malformed_token: bool,
    fail_all: bool,
    token_amount: Option<u64>,
    include_empty: bool,
    extra_tokens: Vec<Value>,
}
#[async_trait]
impl RpcSender for FixtureRpc {
    async fn send(
        &self,
        request: RpcRequest,
        params: Value,
    ) -> solana_rpc_client_api::client_error::Result<Value> {
        self.calls.lock().unwrap().push((request, params.clone()));
        let fail = || {
            solana_rpc_client_api::client_error::Error::from(std::io::Error::other(
                "fixture RPC unavailable",
            ))
        };
        if self.fail_all {
            return Err(fail());
        }
        match request {
            RpcRequest::GetBalance => {
                Ok(json!({"context":{"slot":123}, "value": 1_000_000_001u64}))
            }
            RpcRequest::GetTokenAccountsByOwner => {
                assert_eq!(params[2]["encoding"], "base64");
                assert_eq!(params[2]["commitment"], "confirmed");
                if params[1]["programId"] == TokenProgram::Token2022.id().to_string() {
                    if self.fail_2022 {
                        return Err(fail());
                    }
                    return Ok(json!({"context":{"slot":123}, "value":[]}));
                }
                let mut accounts = vec![json!(RpcKeyedAccount {
                    pubkey: key(3).to_string(),
                    account: ui(
                        raw_token(key(2), key(1), self.token_amount.unwrap_or(1_000_000)),
                        TokenProgram::Legacy.id(),
                        2_039_280
                    )
                })];
                accounts.extend(self.extra_tokens.clone());
                if self.include_empty {
                    accounts.push(json!(RpcKeyedAccount {
                        pubkey: key(5).to_string(),
                        account: ui(
                            raw_token(key(4), key(1), 0),
                            TokenProgram::Legacy.id(),
                            2_039_280
                        )
                    }));
                }
                if self.malformed_token {
                    accounts.push(json!(RpcKeyedAccount {
                        pubkey: key(99).to_string(),
                        account: ui(vec![1, 2], TokenProgram::Legacy.id(), 12345)
                    }));
                }
                Ok(json!({"context":{"slot":123}, "value":accounts}))
            }
            RpcRequest::GetMultipleAccounts => {
                let keys = params[0].as_array().unwrap();
                assert!(keys.len() <= 100);
                let values: Vec<_> = keys
                    .iter()
                    .map(|key| self.accounts.get(key.as_str().unwrap()))
                    .collect();
                Ok(json!({"context":{"slot":123}, "value":values}))
            }
            RpcRequest::GetProgramAccounts => {
                assert_eq!(
                    params[0],
                    mpl_core::ID.to_string(),
                    "Unrelated program scan"
                );
                if self.fail_core && params[0] == mpl_core::ID.to_string() {
                    return Err(fail());
                }
                Ok(json!([]))
            }
            _ => panic!("Unexpected RPC request: {request:?}"),
        }
    }
    fn get_transport_stats(&self) -> RpcTransportStats {
        RpcTransportStats::default()
    }
    fn url(&self) -> String {
        "http://fixture.invalid".into()
    }
}
fn client(fixture: FixtureRpc) -> RpcClient {
    RpcClient::new_sender(fixture, Default::default())
}

#[tokio::test]
async fn portfolio_keeps_successes_unknowns_and_clean_json_on_partial_failures() {
    let mut fixture = FixtureRpc {
        fail_core: true,
        fail_2022: true,
        malformed_token: true,
        ..Default::default()
    };
    fixture.accounts.insert(
        key(2).to_string(),
        ui(raw_mint(6, 10_000_000), TokenProgram::Legacy.id(), 1),
    );
    let calls = fixture.calls.clone();
    let mut portfolio = scan_wallet(
        &client(fixture),
        &key(1),
        &ScanOptions {
            no_prices: true,
            ..Default::default()
        },
        None::<&()>,
    )
    .await;
    assert_eq!(portfolio.tokens[0].balance.as_deref(), Some("1"));
    assert_eq!(portfolio.tokens[0].kind, AssetKind::Fungible);
    assert!(portfolio.classic_nfts.is_empty()); // UI 1 with decimals 6 is not an NFT
    assert_eq!(
        portfolio.native_sol.as_ref().unwrap().lamports,
        1_000_000_001
    );
    assert!(matches!(
        portfolio.scanners["core_asset_v1"],
        ScanStatus::Failed(_)
    ));
    assert!(matches!(
        portfolio.scanners["token_2022"],
        ScanStatus::Failed(_)
    ));
    assert!(matches!(
        portfolio.scanners["tokens"],
        ScanStatus::Partial(_)
    ));
    assert!(
        portfolio
            .unknown_assets
            .iter()
            .any(|asset| asset.address == key(99).to_string() && asset.lamports == Some(12345))
    );
    assert!(portfolio.tokens[0].value_usd.is_none());
    portfolio.tokens[0].metadata.symbol = Some("untrusted\u{1b}[2J".into());
    let mut console = Vec::new();
    dock_flints::cli::output::console::write_portfolio(
        &mut console,
        &portfolio,
        &Default::default(),
    )
    .unwrap();
    assert!(!console.contains(&0x1b));
    let console = String::from_utf8(console).unwrap();
    assert!(console.contains("cNFTs: unavailable (requires historical index)"));
    assert!(!console.contains("PORTFOLIO"));
    let value = dock_flints::cli::output::json::portfolio_json(&portfolio, &Default::default());
    assert_eq!(value["cnfts"]["status"], "unsupported");
    assert_eq!(
        calls
            .lock()
            .unwrap()
            .iter()
            .filter(|(request, _)| *request == RpcRequest::GetTokenAccountsByOwner)
            .count(),
        2
    );
}

#[tokio::test]
async fn complete_rpc_outage_is_fatal() {
    let portfolio = scan_wallet(
        &client(FixtureRpc {
            fail_all: true,
            ..Default::default()
        }),
        &key(1),
        &ScanOptions {
            no_prices: true,
            ..Default::default()
        },
        None::<&()>,
    )
    .await;
    assert!(!portfolio.has_usable_results());
    let value = dock_flints::cli::output::json::portfolio_json(&portfolio, &Default::default());
    for category in ["sol", "tokens", "nfts"] {
        assert_eq!(value[category]["status"], "failed");
    }
}

#[tokio::test]
async fn metadata_batches_deduplicate_and_verify_programmable_and_legacy_nfts() {
    for standard in [Some(TokenStandard::ProgrammableNonFungibleEdition), None] {
        let mut fixture = FixtureRpc::default();
        let pda = Metadata::find_pda(&rpc::to_metaplex(key(2))).0.to_string();
        fixture.accounts.insert(
            pda,
            ui(
                metadata(standard).try_to_vec().unwrap(),
                rpc::to_rpc(mpl_token_metadata::ID),
                1,
            ),
        );
        let edition = MasterEdition {
            key: Key::MasterEditionV2,
            supply: 0,
            max_supply: Some(0),
        };
        fixture.accounts.insert(
            MasterEdition::find_pda(&rpc::to_metaplex(key(2)))
                .0
                .to_string(),
            ui(
                edition.try_to_vec().unwrap(),
                rpc::to_rpc(mpl_token_metadata::ID),
                1,
            ),
        );
        let calls = fixture.calls.clone();
        let mut token = token(1);
        token.decimals = Some(0);
        let scan = get_metadata(
            &client(fixture),
            &[token.clone(), token],
            &[mint_info(0, 1)],
        )
        .await;
        assert!(scan.status.is_complete());
        assert_eq!(scan.items.len(), 1);
        let nft = scan.items[0].nft.as_ref().unwrap();
        assert_eq!(nft.programmable, standard.is_some());
        assert_eq!(nft.edition, standard.is_some());
        assert_eq!(
            calls.lock().unwrap().len(),
            if standard.is_some() { 1 } else { 2 }
        );
        assert_eq!(calls.lock().unwrap()[0].1[0].as_array().unwrap().len(), 1);
    }
}

#[tokio::test]
async fn rpc_batches_are_capped_and_missing_is_distinct_from_failed() {
    let fixture = FixtureRpc::default();
    let calls = fixture.calls.clone();
    let keys: Vec<_> = (1..=205).map(key).collect();
    let fetched = rpc::multiple_accounts(&client(fixture), &keys).await;
    assert_eq!(fetched.len(), 205);
    assert!(fetched.values().all(|account| matches!(account, Ok(None))));
    assert_eq!(calls.lock().unwrap().len(), 3);
    let failed = rpc::multiple_accounts(
        &client(FixtureRpc {
            fail_all: true,
            ..Default::default()
        }),
        &keys[..1],
    )
    .await;
    assert!(failed.values().all(Result::is_err));
}

#[test]
fn token2022_embedded_metadata_and_image_are_decoded() {
    use spl_token_2022_interface::extension::ExtensionType;
    let mut payload = vec![0; 32]; // update_authority=None
    payload.extend_from_slice(&key(2).to_bytes());
    for value in ["Extended token", "EXT", "https://example.invalid/meta.json"] {
        payload.extend_from_slice(&(value.len() as u32).to_le_bytes());
        payload.extend_from_slice(value.as_bytes());
    }
    payload.extend_from_slice(&1_u32.to_le_bytes());
    for value in ["image", "https://example.invalid/image.png"] {
        payload.extend_from_slice(&(value.len() as u32).to_le_bytes());
        payload.extend_from_slice(value.as_bytes());
    }
    let mut data = raw_mint(9, 1_000_000_000);
    data.resize(165, 0);
    data.push(1); // AccountType::Mint
    data.extend_from_slice(&(ExtensionType::TokenMetadata as u16).to_le_bytes());
    data.extend_from_slice(&(payload.len() as u16).to_le_bytes());
    data.extend_from_slice(&payload);
    let mut account = metadata_account(data);
    account.owner = TokenProgram::Token2022.id();
    let mint = parse_mint(&key(2), &account, TokenProgram::Token2022).unwrap();
    assert_eq!(mint.metadata.symbol.as_deref(), Some("EXT"));
    assert_eq!(
        mint.metadata.image_uri.as_deref(),
        Some("https://example.invalid/image.png")
    );
    assert!(parse_mint(&key(7), &account, TokenProgram::Token2022).is_err());
}

#[test]
fn contradictory_nft_metadata_is_not_priced_as_fungible() {
    let record = MetadataRecord {
        mint: key(2).to_string(),
        metadata: TokenMetadata {
            token_standard: Some("NonFungible".into()),
            ..Default::default()
        },
        nft: None,
    };
    let assets = aggregate_tokens(&[token(1_000_000)], &[mint_info(6, 9_000_000)], &[record]);
    assert_eq!(assets[0].kind, AssetKind::Unknown);
}

#[test]
fn master_edition_v1_allows_absent_max_supply_but_requires_printing_keys() {
    let master = MasterEdition {
        key: Key::MasterEditionV1,
        supply: 0,
        max_supply: None,
    };
    let mut account = metadata_account(master.try_to_vec().unwrap());
    assert_eq!(edition_evidence(&account), None);
    account.data.extend_from_slice(&[0; 64]);
    assert_eq!(edition_evidence(&account), Some(false));
}

use clap::Parser;
use dock_flints::{
    app::pricing::PriceProvider,
    app::scan_wallet::scan_wallet,
    cli::Cli,
    cli::output::{OutputOptions, console::write_portfolio, json::portfolio_json},
    core::asset::WRAPPED_SOL,
};

fn options(selection: ScanSelection) -> ScanOptions {
    ScanOptions {
        selection,
        no_prices: true,
    }
}
fn fixture() -> FixtureRpc {
    let mut fixture = FixtureRpc::default();
    fixture.accounts.insert(
        key(2).to_string(),
        ui(raw_mint(6, 10_000_000), TokenProgram::Legacy.id(), 1),
    );
    fixture.accounts.insert(
        key(4).to_string(),
        ui(raw_mint(6, 1_000_000), TokenProgram::Legacy.id(), 1),
    );
    fixture
}
fn text_output(portfolio: &WalletSnapshot, options: &OutputOptions) -> String {
    let mut out = Vec::new();
    write_portfolio(&mut out, portfolio, options).unwrap();
    String::from_utf8(out).unwrap()
}

#[test]
fn category_flags_default_to_all_and_combine_in_any_order() {
    let parse = |flags: &[&str]| {
        let wallet = key(1).to_string();
        let args = ["dock_flints", "-p", wallet.as_str()]
            .into_iter()
            .chain(flags.iter().copied());
        Cli::try_parse_from(args).unwrap().scan_options().selection
    };
    assert_eq!(parse(&[]), ScanSelection::ALL);
    assert_eq!(parse(&["--all"]), ScanSelection::ALL);
    assert_eq!(parse(&["--tokens", "--all"]), ScanSelection::ALL);
    assert_eq!(
        parse(&["--balance", "--tokens"]),
        parse(&["--tokens", "--balance"])
    );
    assert_eq!(
        parse(&["--nfts", "--cnfts"]),
        ScanSelection {
            nfts: true,
            cnfts: true,
            ..Default::default()
        }
    );
    assert_eq!(
        parse(&["--all-tokens"]),
        ScanSelection {
            all_tokens: true,
            ..Default::default()
        }
    );
    assert_eq!(
        parse(&["--all", "--all-tokens"]),
        ScanSelection {
            all_tokens: true,
            ..ScanSelection::ALL
        }
    );
    // Presentation flags alone must not alter category selection.
    assert_eq!(
        parse(&[
            "--details",
            "--include-empty",
            "--show-price",
            "--show-mint"
        ]),
        ScanSelection::ALL
    );
}

#[tokio::test]
async fn every_category_combination_avoids_unrelated_rpc_and_json_categories() {
    for bits in 1..32 {
        let selected = ScanSelection {
            balance: bits & 1 != 0,
            tokens: bits & 2 != 0,
            nfts: bits & 4 != 0,
            cnfts: bits & 8 != 0,
            all_tokens: bits & 16 != 0,
        };
        let fixture = fixture();
        let calls = fixture.calls.clone();
        let portfolio =
            scan_wallet(&client(fixture), &key(1), &options(selected), None::<&()>).await;
        let calls = calls.lock().unwrap();
        let count = |wanted| {
            calls
                .iter()
                .filter(|(request, _)| *request == wanted)
                .count()
        };
        assert_eq!(count(RpcRequest::GetBalance), usize::from(selected.balance));
        assert_eq!(
            count(RpcRequest::GetTokenAccountsByOwner),
            if selected.needs_token_accounts() {
                2
            } else {
                0
            }
        );
        assert_eq!(
            count(RpcRequest::GetProgramAccounts),
            usize::from(selected.nfts)
        );
        // This fixture has no raw=1 NFT candidates: NFT-only needs no mint/metadata batches.
        assert_eq!(
            count(RpcRequest::GetMultipleAccounts),
            if selected.tokens || selected.all_tokens {
                2
            } else {
                0
            }
        );
        let json = portfolio_json(&portfolio, &Default::default());
        for (category, wanted) in [
            ("sol", selected.balance),
            ("tokens", selected.tokens),
            ("all_tokens", selected.all_tokens),
            ("nfts", selected.nfts),
            ("cnfts", selected.cnfts),
        ] {
            assert_eq!(json.get(category).is_some(), wanted, "{bits}: {category}");
        }
        assert!(json.get("scanners").is_none());
        assert!(json.get("stake_accounts").is_none());
        assert_eq!(portfolio.has_usable_results(), bits != 8);
    }
}

#[tokio::test]
async fn include_empty_and_details_are_output_choices_that_preserve_raw_accounts() {
    let mut fixture = fixture();
    fixture.include_empty = true;
    let portfolio = scan_wallet(
        &client(fixture),
        &key(1),
        &options(ScanSelection {
            tokens: true,
            ..Default::default()
        }),
        None::<&()>,
    )
    .await;
    assert_eq!(portfolio.token_accounts.len(), 2);
    let compact = portfolio_json(&portfolio, &Default::default());
    assert_eq!(compact["tokens"]["items"].as_array().unwrap().len(), 1);
    assert!(compact["tokens"].get("discovered_accounts").is_none());
    assert_eq!(compact["tokens"]["items"][0]["total_raw_amount"], "1000000");
    let detailed = portfolio_json(
        &portfolio,
        &OutputOptions {
            include_empty: true,
            details: true,
            ..Default::default()
        },
    );
    assert_eq!(detailed["tokens"]["items"].as_array().unwrap().len(), 2);
    assert_eq!(
        detailed["tokens"]["discovered_accounts"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    let text = text_output(&portfolio, &Default::default());
    assert!(text.contains("TOKENS (1)"));
    assert!(!text.contains(&key(2).to_string()));
    for hidden in [
        "Raw",
        "Decimals",
        "Accounts",
        "Lamports",
        "NFTs",
        "SOL",
        "PORTFOLIO",
        "SCAN STATUS",
    ] {
        assert!(!text.contains(hidden), "Unexpected {hidden}");
    }
    let text = text_output(
        &portfolio,
        &OutputOptions {
            include_empty: true,
            show_mint: true,
            show_price: true,
            details: true,
        },
    );
    assert!(text.contains("TOKENS (2)"));
    for wanted in ["Raw", "Decimals", "Accounts", "Lamports", "Mint", "Price"] {
        assert!(text.contains(wanted));
    }
    assert!(text.contains(&key(2).to_string()));
}

#[derive(Default)]
struct FixturePrices {
    calls: Mutex<Vec<Vec<String>>>,
    fail: bool,
}
impl PriceProvider for FixturePrices {
    async fn get_prices(&self, mints: &[String]) -> PriceReport {
        self.calls.lock().unwrap().push(mints.to_vec());
        if self.fail {
            return PriceReport {
                status: ScanStatus::Failed("fixture price outage".into()),
                quotes: BTreeMap::new(),
            };
        }
        PriceReport {
            status: ScanStatus::Complete,
            quotes: mints
                .iter()
                .map(|mint| {
                    (
                        mint.clone(),
                        Price {
                            usd: 2.0,
                            source: "fixture".into(),
                            block_id: Some(123),
                            decimals: if mint == WRAPPED_SOL { 9 } else { 6 },
                        },
                    )
                })
                .collect(),
        }
    }
}

#[tokio::test]
async fn prices_follow_selection_and_no_prices_prevents_provider_calls() {
    for selected in [
        ScanSelection {
            balance: true,
            ..Default::default()
        },
        ScanSelection {
            tokens: true,
            ..Default::default()
        },
        ScanSelection {
            all_tokens: true,
            ..Default::default()
        },
        ScanSelection {
            all_tokens: true,
            tokens: true,
            ..Default::default()
        },
        ScanSelection {
            nfts: true,
            ..Default::default()
        },
        ScanSelection {
            cnfts: true,
            ..Default::default()
        },
    ] {
        for no_prices in [false, true] {
            let provider = FixturePrices::default();
            let portfolio = scan_wallet(
                &client(fixture()),
                &key(1),
                &ScanOptions {
                    selection: selected,
                    no_prices,
                },
                Some(&provider),
            )
            .await;
            let calls = provider.calls.lock().unwrap();
            assert_eq!(
                calls.len(),
                usize::from(selected.needs_prices() && !no_prices)
            );
            if selected.balance && !no_prices {
                assert_eq!(calls[0], vec![WRAPPED_SOL.to_string()]);
                assert!(portfolio.native_sol.unwrap().value_usd.is_some());
            }
            if selected.all_tokens && !no_prices {
                assert_eq!(calls[0], vec![key(2).to_string()]);
                assert_eq!(portfolio.all_tokens[0].value_usd, Some(2.0));
            }
            if selected.tokens && !no_prices {
                assert_eq!(calls[0], vec![key(2).to_string()]);
                assert_eq!(portfolio.tokens[0].value_usd, Some(2.0));
            }
        }
    }
}

#[tokio::test]
async fn pricing_failure_keeps_blockchain_results() {
    let provider = FixturePrices {
        fail: true,
        ..Default::default()
    };
    let portfolio = scan_wallet(
        &client(fixture()),
        &key(1),
        &ScanOptions {
            selection: ScanSelection {
                balance: true,
                tokens: true,
                ..Default::default()
            },
            ..Default::default()
        },
        Some(&provider),
    )
    .await;
    assert!(portfolio.has_usable_results());
    assert_eq!(portfolio.tokens[0].balance.as_deref(), Some("1"));
    assert!(portfolio.tokens[0].value_usd.is_none());
    assert!(matches!(
        portfolio.scanners["prices"],
        ScanStatus::Failed(_)
    ));
    assert!(
        text_output(&portfolio, &Default::default())
            .contains("Prices: unavailable (provider request failed)")
    );
}

#[tokio::test]
async fn compact_output_sorts_values_formats_tiny_prices_and_combines_nft_types() {
    let mut portfolio = scan_wallet(
        &client(fixture()),
        &key(1),
        &options(ScanSelection::ALL),
        None::<&()>,
    )
    .await;
    portfolio.tokens.clear();
    for (mint, symbol, value) in [
        (key(21), "Unpriced", None),
        (key(22), "Tiny", Some(0.00000002)),
        (key(23), "Valuable", Some(4.0)),
    ] {
        let mut account = token(1_000_000);
        account.mint = mint.to_string();
        let mut assets = aggregate_tokens(&[account], &[], &[]);
        let mut asset = assets.pop().unwrap();
        asset.metadata.symbol = Some(symbol.into());
        asset.value_usd = value;
        asset.price = value.map(|usd| Price {
            usd,
            source: "fixture".into(),
            block_id: None,
            decimals: 6,
        });
        portfolio.tokens.push(asset);
    }
    let nft_meta = TokenMetadata {
        name: Some("Classic sample".into()),
        uri: Some("https://example.invalid/nft".into()),
        collection: Some(CollectionInfo {
            address: key(33).to_string(),
            verified: true,
        }),
        ..Default::default()
    };
    portfolio.classic_nfts = vec![
        NftAsset {
            mint: key(31).to_string(),
            token_accounts: vec![],
            metadata: nft_meta.clone(),
            programmable: false,
            edition: false,
            evidence: "fixture".into(),
        },
        NftAsset {
            mint: key(32).to_string(),
            token_accounts: vec![],
            metadata: TokenMetadata {
                name: Some("Programmable sample".into()),
                ..nft_meta
            },
            programmable: true,
            edition: false,
            evidence: "fixture".into(),
        },
    ];
    portfolio.core_assets = vec![CoreAsset {
        address: key(34).to_string(),
        owner: key(1).to_string(),
        name: "Core sample".into(),
        uri: "https://example.invalid/core".into(),
        lamports: 3,
        data_len: 90,
        update_authority: "None".into(),
        collection: None,
        plugins_status: ScanStatus::Complete,
    }];
    let text = text_output(
        &portfolio,
        &OutputOptions {
            show_price: true,
            ..Default::default()
        },
    );
    assert!(text.find("Valuable").unwrap() < text.find("Tiny").unwrap());
    assert!(text.find("Tiny").unwrap() < text.find("Unpriced").unwrap());
    assert!(text.contains("$2.000e-8"));
    assert!(!text.contains("$0.00"));
    assert!(text.contains("NFTs (3)"));
    for name in ["Classic sample", "Programmable sample", "Core sample"] {
        assert_eq!(text.matches(name).count(), 1);
    }
    assert!(!text.contains(&key(31).to_string()));
    let json = portfolio_json(&portfolio, &Default::default());
    assert_eq!(json["nfts"]["items"][0]["asset_id"], key(31).to_string());
    assert_eq!(json["nfts"]["items"][0]["collection"]["verified"], true);
    assert_eq!(json["tokens"]["items"][0]["mint"], key(23).to_string());
}

#[test]
fn cnft_only_cli_returns_error_and_clean_machine_readable_status_without_rpc() {
    let wallet = key(1).to_string();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_dock_flints"))
        .args([
            "-p",
            &wallet,
            "--cnfts",
            "--rpc-url",
            "http://127.0.0.1:1",
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stderr.is_empty());
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json.as_object().unwrap().len(), 2);
    assert_eq!(json["cnfts"]["code"], "historical_index_required");
    assert_eq!(json["cnfts"]["status"], "unsupported");
    assert!(json["cnfts"]["items"].is_null());
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_dock_flints"))
        .args(["-p", &wallet, "--cnfts", "--rpc-url", "http://127.0.0.1:1"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    let text = String::from_utf8(output.stdout).unwrap();
    assert_eq!(
        text.matches("cNFTs: unavailable (requires historical index)")
            .count(),
        1
    );
    assert!(!text.contains("(0)"));
}

#[tokio::test]
async fn nft_only_verifies_candidates_and_combined_scans_do_not_duplicate_accounts() {
    for (standard, expected) in [
        (TokenStandard::NonFungible, AssetKind::NonFungible),
        (
            TokenStandard::NonFungibleEdition,
            AssetKind::NonFungibleEdition,
        ),
        (
            TokenStandard::ProgrammableNonFungible,
            AssetKind::ProgrammableNonFungible,
        ),
        (
            TokenStandard::ProgrammableNonFungibleEdition,
            AssetKind::ProgrammableNonFungibleEdition,
        ),
    ] {
        for selected in [
            ScanSelection {
                all_tokens: true,
                ..Default::default()
            },
            ScanSelection {
                all_tokens: true,
                tokens: true,
                nfts: true,
                ..Default::default()
            },
            ScanSelection {
                nfts: true,
                ..Default::default()
            },
            ScanSelection {
                tokens: true,
                nfts: true,
                ..Default::default()
            },
            ScanSelection {
                tokens: true,
                ..Default::default()
            },
        ] {
            let mut fixture = fixture();
            fixture.token_amount = Some(1);
            fixture.accounts.insert(
                key(2).to_string(),
                ui(raw_mint(0, 1), TokenProgram::Legacy.id(), 1),
            );
            let mut nft_metadata = metadata(Some(standard));
            nft_metadata.collection = Some(mpl_token_metadata::types::Collection {
                verified: true,
                key: rpc::to_metaplex(key(44)),
            });
            fixture.accounts.insert(
                Metadata::find_pda(&rpc::to_metaplex(key(2))).0.to_string(),
                ui(
                    nft_metadata.try_to_vec().unwrap(),
                    rpc::to_rpc(mpl_token_metadata::ID),
                    1,
                ),
            );
            let calls = fixture.calls.clone();
            let provider = FixturePrices::default();
            let portfolio = scan_wallet(
                &client(fixture),
                &key(1),
                &ScanOptions {
                    selection: selected,
                    ..Default::default()
                },
                Some(&provider),
            )
            .await;
            assert_eq!(portfolio.classic_nfts.len(), usize::from(selected.nfts));
            assert!(
                portfolio.tokens.is_empty(),
                "NFT must not appear as a fungible asset"
            );
            assert!(
                provider.calls.lock().unwrap().is_empty(),
                "NFTs must not be priced as fungible tokens"
            );
            if selected.all_tokens {
                assert_eq!(portfolio.all_tokens.len(), 1);
                assert_eq!(portfolio.all_tokens[0].kind, expected);
                assert!(portfolio.all_tokens[0].price.is_none());
                let json = portfolio_json(
                    &portfolio,
                    &OutputOptions {
                        details: true,
                        ..Default::default()
                    },
                );
                assert_eq!(
                    json["all_tokens"]["items"][0]["accounts"][0],
                    key(3).to_string()
                );
                assert_eq!(
                    json["all_tokens"]["discovered_accounts"][0]["raw_amount"],
                    "1"
                );
                let text = text_output(
                    &portfolio,
                    &OutputOptions {
                        details: true,
                        ..Default::default()
                    },
                );
                assert!(text.contains("ALL TOKEN ASSETS (1)"));
                for field in [
                    "Mint",
                    "Raw",
                    "Decimals",
                    "Program",
                    "Accounts",
                    "Lamports",
                    "Metadata URI",
                ] {
                    assert!(text.contains(field));
                }
            }
            if selected.nfts {
                assert_eq!(
                    portfolio.classic_nfts[0].programmable,
                    expected.is_programmable()
                );
                assert_eq!(
                    portfolio.classic_nfts[0]
                        .metadata
                        .collection
                        .as_ref()
                        .unwrap()
                        .address,
                    key(44).to_string()
                );
            }
            let calls = calls.lock().unwrap();
            assert_eq!(
                calls
                    .iter()
                    .filter(|(request, _)| *request == RpcRequest::GetTokenAccountsByOwner)
                    .count(),
                2
            );
            assert_eq!(
                calls
                    .iter()
                    .filter(|(request, _)| *request == RpcRequest::GetMultipleAccounts)
                    .count(),
                2
            );
        }
    }
}

#[test]
fn classifier_uses_mint_semantics_not_ui_one_or_metadata_labels() {
    use dock_flints::core::classification::{classify, classify_token_accounts};
    for decimals in [6, 9] {
        let mut account = token(10_u64.pow(decimals.into()));
        account.decimals = Some(decimals);
        let mint = mint_info(decimals, account.raw_amount);
        let classified = classify_token_accounts(
            std::slice::from_ref(&account),
            std::slice::from_ref(&mint),
            &[],
        );
        assert_eq!(classified[0].balance.as_deref(), Some("1"));
        assert_eq!(classify(&account, Some(&mint), None), AssetKind::Fungible);
    }
    let mut account = token(1);
    account.decimals = Some(0);
    let mut mint = mint_info(0, 1);
    mint.metadata.name = Some("Looks like an NFT".into());
    mint.extension_types = vec!["TokenMetadata".into(), "NonTransferable".into()];
    assert_eq!(classify(&account, Some(&mint), None), AssetKind::Unknown);
    let record = MetadataRecord {
        mint: key(2).to_string(),
        metadata: TokenMetadata {
            token_standard: Some("FungibleAsset".into()),
            ..Default::default()
        },
        nft: None,
    };
    assert_eq!(
        classify(&account, Some(&mint), Some(&record)),
        AssetKind::FungibleAsset
    );
    let mut second = account.clone();
    second.address = key(4).to_string();
    let unknown = classify_token_accounts(&[account, second], &[mint], &[]);
    assert_eq!(
        dock_flints::core::amount::aggregate_tokens(&unknown).len(),
        2,
        "Unknown accounts must not be aggregated as fungibles"
    );
}

#[tokio::test]
async fn mixed_inventory_is_partitioned_without_loss_and_prices_only_fungibles() {
    let mut fixture = fixture();
    for (mint, address, decimals, amount, standard) in [
        (2, 7, Some(6), 2_000_000, None), // second fungible account
        (8, 9, Some(0), 1, Some(TokenStandard::NonFungible)),
        (10, 11, Some(0), 1, None), // ambiguous zero-decimal mint
        (12, 13, None, 42, None),   // missing mint must survive
    ] {
        fixture.extra_tokens.push(json!(RpcKeyedAccount {
            pubkey: key(address).to_string(),
            account: ui(
                raw_token(key(mint), key(1), amount),
                TokenProgram::Legacy.id(),
                2_039_280
            ),
        }));
        if let Some(decimals) = decimals {
            fixture.accounts.insert(
                key(mint).to_string(),
                ui(raw_mint(decimals, amount), TokenProgram::Legacy.id(), 1),
            );
        }
        if let Some(standard) = standard {
            let mut data = metadata(Some(standard));
            data.mint = rpc::to_metaplex(key(mint));
            fixture.accounts.insert(
                Metadata::find_pda(&data.mint).0.to_string(),
                ui(
                    data.try_to_vec().unwrap(),
                    rpc::to_rpc(mpl_token_metadata::ID),
                    1,
                ),
            );
        }
    }
    let calls = fixture.calls.clone();
    let provider = FixturePrices::default();
    let portfolio = scan_wallet(
        &client(fixture),
        &key(1),
        &ScanOptions {
            selection: ScanSelection {
                tokens: true,
                all_tokens: true,
                nfts: true,
                ..Default::default()
            },
            ..Default::default()
        },
        Some(&provider),
    )
    .await;
    assert_eq!(portfolio.all_tokens.len(), 5);
    assert_eq!(portfolio.tokens.len(), 3);
    assert_eq!(portfolio.classic_nfts.len(), 1);
    let fungible = portfolio
        .tokens
        .iter()
        .find(|token| token.kind.is_fungible())
        .unwrap();
    assert_eq!(fungible.balance.as_deref(), Some("3"));
    assert_eq!(fungible.accounts.len(), 2);
    assert_eq!(fungible.value_usd, Some(6.0));
    assert_eq!(
        portfolio
            .tokens
            .iter()
            .filter(|token| token.kind == AssetKind::Unknown)
            .count(),
        2
    );
    assert!(portfolio.tokens.iter().all(|token| !token.kind.is_nft()));
    assert!(
        portfolio
            .all_tokens
            .iter()
            .filter(|token| !token.kind.is_fungible())
            .all(|token| token.price.is_none())
    );
    assert_eq!(
        *provider.calls.lock().unwrap(),
        vec![vec![key(2).to_string()]]
    );
    let calls = calls.lock().unwrap();
    assert_eq!(
        calls
            .iter()
            .filter(|(request, _)| *request == RpcRequest::GetTokenAccountsByOwner)
            .count(),
        2
    );
    assert_eq!(
        calls
            .iter()
            .filter(|(request, _)| *request == RpcRequest::GetMultipleAccounts)
            .count(),
        2
    );
    let text = text_output(&portfolio, &Default::default());
    assert!(text.contains("Unknown"));
    assert!(text.contains("ALL TOKEN ASSETS (5 found; incomplete)"));
    let json = portfolio_json(&portfolio, &Default::default());
    assert_eq!(json["tokens"]["items"].as_array().unwrap().len(), 3);
    assert_eq!(json["nfts"]["items"].as_array().unwrap().len(), 1);
    assert_eq!(json["all_tokens"]["items"].as_array().unwrap().len(), 5);
}

#[test]
fn empty_verified_nft_accounts_stay_out_of_fungible_categories() {
    use dock_flints::core::classification::classify_token_accounts;
    let mut account = token(0);
    account.decimals = Some(0);
    let record = MetadataRecord {
        mint: account.mint.clone(),
        metadata: TokenMetadata {
            token_standard: Some("NonFungible".into()),
            ..Default::default()
        },
        nft: None,
    };
    let raw = classify_token_accounts(&[account], &[mint_info(0, 1)], &[record]);
    assert_eq!(raw[0].kind, AssetKind::NonFungible);
    assert!(dock_flints::core::amount::aggregate_tokens(&raw).is_empty());
    assert_eq!(raw.len(), 1);
}

#[tokio::test]
async fn legacy_edition_evidence_drives_all_categories_and_unknown_fallback() {
    use mpl_token_metadata::accounts::Edition;
    for edition in [None, Some(false), Some(true)] {
        let mut fixture = fixture();
        fixture.token_amount = Some(1);
        fixture.accounts.insert(
            key(2).to_string(),
            ui(raw_mint(0, 1), TokenProgram::Legacy.id(), 1),
        );
        fixture.accounts.insert(
            Metadata::find_pda(&rpc::to_metaplex(key(2))).0.to_string(),
            ui(
                metadata(None).try_to_vec().unwrap(),
                rpc::to_rpc(mpl_token_metadata::ID),
                1,
            ),
        );
        if let Some(printed) = edition {
            let data = if printed {
                Edition {
                    key: Key::EditionV1,
                    parent: rpc::to_metaplex(key(88)),
                    edition: 1,
                }
                .try_to_vec()
                .unwrap()
            } else {
                MasterEdition {
                    key: Key::MasterEditionV2,
                    supply: 0,
                    max_supply: Some(0),
                }
                .try_to_vec()
                .unwrap()
            };
            fixture.accounts.insert(
                MasterEdition::find_pda(&rpc::to_metaplex(key(2)))
                    .0
                    .to_string(),
                ui(data, rpc::to_rpc(mpl_token_metadata::ID), 1),
            );
        }
        let calls = fixture.calls.clone();
        let portfolio = scan_wallet(
            &client(fixture),
            &key(1),
            &options(ScanSelection {
                tokens: true,
                all_tokens: true,
                nfts: true,
                ..Default::default()
            }),
            None::<&()>,
        )
        .await;
        assert_eq!(portfolio.all_tokens.len(), 1);
        assert_eq!(
            portfolio.all_tokens[0].kind,
            match edition {
                None => AssetKind::Unknown,
                Some(false) => AssetKind::NonFungible,
                Some(true) => AssetKind::NonFungibleEdition,
            }
        );
        assert_eq!(portfolio.classic_nfts.len(), usize::from(edition.is_some()));
        assert_eq!(portfolio.tokens.len(), usize::from(edition.is_none()));
        assert_eq!(
            calls
                .lock()
                .unwrap()
                .iter()
                .filter(|(request, _)| *request == RpcRequest::GetMultipleAccounts)
                .count(),
            3
        );
    }
}

#[test]
fn zero_decimal_multiunit_tokens_are_fungible_without_labels_but_unique_mints_need_evidence() {
    use dock_flints::core::classification::classify;
    let mut account = token(2);
    account.decimals = Some(0);
    let mut mint = mint_info(0, 100);
    assert_eq!(
        classify(&account, Some(&mint), None),
        AssetKind::FungibleAsset
    );
    let record = MetadataRecord {
        mint: mint.mint.clone(),
        metadata: TokenMetadata {
            token_standard: Some("NonFungible".into()),
            ..Default::default()
        },
        nft: None,
    };
    assert_eq!(
        classify(&account, Some(&mint), Some(&record)),
        AssetKind::Unknown
    );
    account.raw_amount = 1;
    mint.supply = 1;
    assert_eq!(classify(&account, Some(&mint), None), AssetKind::Unknown);
    account.raw_amount = 0;
    mint.supply = 0;
    assert_eq!(classify(&account, Some(&mint), None), AssetKind::Unknown);
}
