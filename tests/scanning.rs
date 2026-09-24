use async_trait::async_trait;
use base64::{Engine, engine::general_purpose::STANDARD};
use borsh::BorshSerialize;
use flints_station::{
    models::*,
    portfolio::{aggregate::*, service::scan_wallet},
    pricing::jupiter::parse_quote,
    rpc,
    scanner::{
        core::parse_asset,
        metadata::parse_mint,
        nft::{decode_metadata, edition_evidence, get_metadata},
        stake::{STAKE_PROGRAM, parse_position},
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
use solana_client::{
    nonblocking::rpc_client::RpcClient,
    rpc_request::RpcRequest,
    rpc_response::RpcKeyedAccount,
    rpc_sender::{RpcSender, RpcTransportStats},
};
use solana_pubkey::Pubkey;
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

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
    let assets = aggregate_tokens(&[token(u64::MAX), token(u64::MAX)], &[], &[]);
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
fn stake_layout_recognizes_each_authority_and_delegation() {
    let mut data = vec![0; 200];
    data[..4].copy_from_slice(&2_u32.to_le_bytes()); // StakeStateV2::Stake
    data[12..44].copy_from_slice(&key(1).to_bytes());
    data[44..76].copy_from_slice(&key(7).to_bytes());
    data[124..156].copy_from_slice(&key(9).to_bytes()); // validator
    data[156..164].copy_from_slice(&3_000_000_000_u64.to_le_bytes());
    data[164..172].copy_from_slice(&120_u64.to_le_bytes());
    data[172..180].copy_from_slice(&u64::MAX.to_le_bytes());
    let account = ui(data, STAKE_PROGRAM, 3_002_282_880);
    let stake = parse_position(key(6), &account, &key(1)).unwrap();
    assert!(!stake.wallet_can_withdraw);
    assert_eq!(stake.delegated_lamports, Some(3_000_000_000));
    assert_eq!(stake.validator_vote_account, Some(key(9).to_string()));
    assert!(
        parse_position(key(6), &account, &key(7))
            .unwrap()
            .wallet_can_withdraw
    );
    assert!(parse_position(key(6), &account, &key(8)).is_err());
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
        flints_station::scanner::cnft::support(),
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
}
#[async_trait]
impl RpcSender for FixtureRpc {
    async fn send(
        &self,
        request: RpcRequest,
        params: Value,
    ) -> solana_client::client_error::Result<Value> {
        self.calls.lock().unwrap().push((request, params.clone()));
        let fail = || {
            solana_client::client_error::ClientError::from(std::io::Error::other(
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
                        raw_token(key(2), key(1), 1_000_000),
                        TokenProgram::Legacy.id(),
                        2_039_280
                    )
                })];
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
    let mut portfolio = scan_wallet(&client(fixture), &key(1), true, None, false)
        .await
        .unwrap();
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
        portfolio.scanners["metaplex_metadata_and_nfts"],
        ScanStatus::Partial(_)
    ));
    assert!(
        portfolio
            .unknown_assets
            .iter()
            .any(|asset| asset.address == key(99).to_string() && asset.lamports == Some(12345))
    );
    assert!(portfolio.known_value_usd.is_none());
    portfolio.tokens[0].metadata.symbol = Some("untrusted\u{1b}[2J".into());
    let mut console = Vec::new();
    flints_station::output::console::write_portfolio(&mut console, &portfolio, false).unwrap();
    assert!(!console.contains(&0x1b));
    let console = String::from_utf8(console).unwrap();
    assert!(console.contains("Unsupported"));
    assert!(console.contains("Known USD value: unavailable"));
    let value = serde_json::to_value(&portfolio).unwrap();
    assert_eq!(
        value["scanners"]["compressed_nfts"]["status"],
        "unsupported"
    );
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
    assert!(
        scan_wallet(
            &client(FixtureRpc {
                fail_all: true,
                ..Default::default()
            }),
            &key(1),
            true,
            None,
            false
        )
        .await
        .is_err()
    );
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
