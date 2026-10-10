//! Binary SDK fixtures exercise real scanner/planner/adapters; RPC submission is mocked, never an on-chain claim.
use async_trait::async_trait;
use base64::{Engine, engine::general_purpose::STANDARD};
use borsh::BorshSerialize;
use dock_flints_core::{
    app::{
        categories::ScopedSwap,
        cleanup::{plan::add_nfts_observed, *},
    },
    core::{nft_cleanup::*, *},
    infra::{
        jupiter::Jupiter,
        solana::{self, nft_cleanup::SolanaCleanupExecutor, scan::das::DasClient},
    },
};
use serde_json::{Value, json};
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_rpc_client::{
    nonblocking::rpc_client::RpcClient,
    rpc_sender::{RpcSender, RpcTransportStats},
};
use solana_rpc_client_api::request::RpcRequest;
use solana_signer::Signer;
use solana_transaction::versioned::VersionedTransaction;
use std::{
    collections::BTreeMap,
    io::{Read, Write},
    net::TcpListener,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
fn key(n: u8) -> Pubkey {
    Pubkey::new_from_array([n; 32])
}
fn ui(data: Vec<u8>, program: Pubkey) -> Value {
    json!({"lamports":5_000_000,"owner":program.to_string(),"data":[STANDARD.encode(data),"base64"],"executable":false,"rentEpoch":0})
}
fn token(mint: Pubkey, owner: Pubkey, frozen: bool) -> Vec<u8> {
    let mut d = vec![0; 165];
    d[..32].copy_from_slice(mint.as_ref());
    d[32..64].copy_from_slice(owner.as_ref());
    d[64..72].copy_from_slice(&1u64.to_le_bytes());
    d[108] = if frozen { 2 } else { 1 };
    d
}
fn mint(authority: Pubkey) -> Vec<u8> {
    let mut d = vec![0; 82];
    d[..4].copy_from_slice(&1u32.to_le_bytes());
    d[4..36].copy_from_slice(authority.as_ref());
    d[36..44].copy_from_slice(&1u64.to_le_bytes());
    d[45] = 1;
    d
}
#[derive(Default)]
struct Chain {
    accounts: BTreeMap<String, Value>,
    tokens: Vec<String>,
    cores: Vec<String>,
    parent: Option<(String, String)>,
    signatures: BTreeMap<String, String>,
    burnt: Arc<AtomicBool>,
}
#[derive(Clone)]
struct Sender {
    chain: Arc<Mutex<Chain>>,
    owner: Pubkey,
    target: NftTarget,
}
#[async_trait]
impl RpcSender for Sender {
    async fn send(
        &self,
        request: RpcRequest,
        params: Value,
    ) -> solana_rpc_client_api::client_error::Result<Value> {
        let mut state = self.chain.lock().unwrap();
        Ok(match request {
            RpcRequest::GetGenesisHash => json!("EtWTRABZaYq6iMfeYKouRu166VU2xqa1wcaWoxPkrZBG"),
            RpcRequest::GetBalance => {
                json!({"context":{"slot":1},"value":if params[0]==self.owner.to_string(){1_000_000_000u64}else{0}})
            }
            RpcRequest::GetAccountInfo => {
                json!({"context":{"slot":1},"value":state.accounts.get(params[0].as_str().unwrap())})
            }
            RpcRequest::GetMultipleAccounts => {
                json!({"context":{"slot":1},"value":params[0].as_array().unwrap().iter().map(|k|state.accounts.get(k.as_str().unwrap())).collect::<Vec<_>>()})
            }
            RpcRequest::GetTokenAccountsByOwner => {
                json!({"context":{"slot":1},"value":if params[0]==self.owner.to_string() && params[1]["programId"]==TokenProgram::Legacy.id().to_string(){state.tokens.iter().filter_map(|k|state.accounts.get(k).map(|v|json!({"pubkey":k,"account":v}))).collect::<Vec<_>>()}else{vec![]}})
            }
            RpcRequest::GetProgramAccounts => {
                if params[0] == mpl_core::ID.to_string() {
                    let bytes = &params[1]["filters"][1]["memcmp"]["bytes"];
                    let own = *bytes == self.owner.to_string()
                        || *bytes == json!(self.owner.to_bytes().to_vec());
                    json!(if own {
                        state
                            .cores
                            .iter()
                            .filter_map(|k| {
                                state
                                    .accounts
                                    .get(k)
                                    .map(|v| json!({"pubkey":k,"account":v}))
                            })
                            .collect::<Vec<_>>()
                    } else {
                        vec![]
                    })
                } else {
                    json!(
                        state
                            .parent
                            .as_ref()
                            .map(|(m, _)| vec![json!({"pubkey":m,"account":state.accounts[m]})])
                            .unwrap_or_default()
                    )
                }
            }
            RpcRequest::GetTokenLargestAccounts => {
                json!({"context":{"slot":1},"value":[{"address":state.parent.as_ref().unwrap().1,"amount":"1","decimals":0,"uiAmount":1.0,"uiAmountString":"1"}]})
            }
            RpcRequest::GetLatestBlockhash => {
                json!({"context":{"slot":1},"value":{"blockhash":Pubkey::default().to_string(),"lastValidBlockHeight":1000}})
            }
            RpcRequest::GetBlockHeight => json!(10),
            RpcRequest::GetVersion => json!({"solana-core":"3.0.0","feature-set":0}),
            RpcRequest::SimulateTransaction => {
                assert_eq!(params[1]["sigVerify"], true);
                json!({"context":{"slot":1},"value":{"err":null,"logs":[],"unitsConsumed":100_000}})
            }
            RpcRequest::SendTransaction => {
                assert_eq!(params[1]["skipPreflight"], false);
                let tx: VersionedTransaction =
                    bincode::deserialize(&STANDARD.decode(params[0].as_str().unwrap()).unwrap())
                        .unwrap();
                let keys = tx.message.static_account_keys();
                let ops = tx.message.instructions();
                assert_eq!(
                    ops.len(),
                    2,
                    "Only compute budget and standard-specific burn; no generic SPL burn/close"
                );
                let ix = &ops[1];
                assert_eq!(
                    keys[usize::from(ix.program_id_index)].to_string(),
                    match self.target.standard {
                        NftStandard::TokenMetadata(_) => mpl_token_metadata::ID.to_string(),
                        NftStandard::Core => mpl_core::ID.to_string(),
                        NftStandard::Compressed => mpl_bubblegum::ID.to_string(),
                    }
                );
                assert!(!ix.accounts.is_empty());
                let id = if matches!(self.target.standard, NftStandard::TokenMetadata(_)) {
                    keys[usize::from(ix.accounts[5])].to_string()
                } else {
                    self.target.id.clone()
                };
                if self.target.token_account.as_ref() == Some(&id)
                    && let Some((parent, _)) = state.parent.clone()
                {
                    let parent_key: Pubkey = parent.parse().unwrap();
                    let edition = solana::to_rpc(
                        mpl_token_metadata::accounts::MasterEdition::find_pda(
                            &solana::to_metaplex(parent_key),
                        )
                        .0,
                    )
                    .to_string();
                    let record = state.accounts.get_mut(&edition).unwrap();
                    let mut bytes = STANDARD
                        .decode(record["data"][0].as_str().unwrap())
                        .unwrap();
                    let supply = u64::from_le_bytes(bytes[1..9].try_into().unwrap());
                    if supply > 0 {
                        bytes[1..9].copy_from_slice(&(supply - 1).to_le_bytes());
                    }
                    record["data"][0] = STANDARD.encode(bytes).into();
                }
                state.accounts.remove(&id);
                state.burnt.store(true, Ordering::Relaxed);
                let signature = tx.signatures[0].to_string();
                state.signatures.insert(signature.clone(), id);
                json!(signature)
            }
            RpcRequest::GetSignatureStatuses => {
                json!({"context":{"slot":1},"value":params[0].as_array().unwrap().iter().map(|sig|state.signatures.contains_key(sig.as_str().unwrap()).then(||json!({"slot":1,"confirmations":null,"err":null,"status":{"Ok":null},"confirmationStatus":"confirmed"}))).collect::<Vec<_>>()})
            }
            RpcRequest::GetTransaction => {
                let source = &state.signatures[params[0].as_str().unwrap()];
                let reclaim = if matches!(self.target.standard, NftStandard::Compressed) {
                    0
                } else {
                    5_000_000
                };
                json!({"transaction":{"message":{"accountKeys":[self.owner.to_string(),source]}},"meta":{"err":null,"preBalances":[1_000_000_000u64,reclaim],"postBalances":[1_000_000_000u64+reclaim-5000,0]}})
            }
            _ => panic!("Unexpected NFT fixture RPC {request:?}"),
        })
    }
    fn url(&self) -> String {
        "http://fixture.invalid".into()
    }
    fn get_transport_stats(&self) -> RpcTransportStats {
        Default::default()
    }
}
struct DasServer {
    url: String,
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl Drop for DasServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        self.thread.take().unwrap().join().unwrap();
    }
}
fn das_server(asset: Value, proof: Value, burnt: Arc<AtomicBool>) -> DasServer {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let stop = Arc::new(AtomicBool::new(false));
    let stopped = stop.clone();
    let thread = std::thread::spawn(move || {
        while !stopped.load(Ordering::Relaxed) {
            let (mut stream, _) = match listener.accept() {
                Ok(v) => v,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(2));
                    continue;
                }
                Err(e) => panic!("{e}"),
            };
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut bytes = vec![];
            let mut buffer = [0; 8192];
            let request: Value = loop {
                let n = stream.read(&mut buffer).unwrap();
                assert!(n > 0);
                bytes.extend_from_slice(&buffer[..n]);
                if let Some(end) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&bytes[..end]);
                    let length: usize = headers
                        .lines()
                        .find_map(|l| {
                            l.split_once(':')
                                .filter(|(k, _)| k.eq_ignore_ascii_case("content-length"))
                                .map(|(_, v)| v.trim().parse().unwrap())
                        })
                        .unwrap();
                    if bytes.len() >= end + 4 + length {
                        break serde_json::from_slice(&bytes[end + 4..end + 4 + length]).unwrap();
                    }
                }
            };
            let result = match request["method"].as_str().unwrap() {
                "getGenesisHash" => json!("EtWTRABZaYq6iMfeYKouRu166VU2xqa1wcaWoxPkrZBG"),
                "getAssetsByOwner" => json!({"total":0,"items":[]}),
                "getAsset" => {
                    let mut a = asset.clone();
                    a["burnt"] = burnt.load(Ordering::Relaxed).into();
                    a
                }
                "getAssetProof" => proof.clone(),
                m => panic!("Unexpected DAS {m}"),
            };
            let body = json!({"jsonrpc":"2.0","id":request["id"],"result":result}).to_string();
            write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).unwrap();
        }
    });
    DasServer {
        url,
        stop,
        thread: Some(thread),
    }
}
fn fixture(standard: NftStandard, owner: Pubkey) -> (Sender, Value, Value) {
    let mut state = Chain::default();
    let mut asset = Value::Null;
    let mut proof = Value::Null;
    let target = match standard {
        NftStandard::TokenMetadata(kind) => {
            use mpl_token_metadata::{
                accounts::{Edition, MasterEdition, Metadata, TokenRecord},
                types::{Key, TokenStandard, TokenState},
            };
            let mintkey = key(2);
            let source = key(3);
            let metadata = Metadata::find_pda(&solana::to_metaplex(mintkey)).0;
            let edition = MasterEdition::find_pda(&solana::to_metaplex(mintkey)).0;
            let edition_kind = matches!(
                kind,
                AssetKind::NonFungibleEdition | AssetKind::ProgrammableNonFungibleEdition
            );
            let data = Metadata {
                key: Key::MetadataV1,
                update_authority: solana::to_metaplex(owner),
                mint: solana::to_metaplex(mintkey),
                name: "Binary NFT fixture".into(),
                symbol: "FIX".into(),
                uri: "https://fixture.invalid".into(),
                seller_fee_basis_points: 0,
                creators: None,
                primary_sale_happened: false,
                is_mutable: true,
                edition_nonce: None,
                token_standard: Some(match kind {
                    AssetKind::NonFungible => TokenStandard::NonFungible,
                    AssetKind::NonFungibleEdition => TokenStandard::NonFungibleEdition,
                    AssetKind::ProgrammableNonFungible => TokenStandard::ProgrammableNonFungible,
                    _ => TokenStandard::ProgrammableNonFungibleEdition,
                }),
                collection: None,
                uses: None,
                collection_details: None,
                programmable_config: None,
            };
            state.accounts.insert(
                metadata.to_string(),
                ui(
                    data.try_to_vec().unwrap(),
                    solana::to_rpc(mpl_token_metadata::ID),
                ),
            );
            state.accounts.insert(
                mintkey.to_string(),
                ui(mint(solana::to_rpc(edition)), TokenProgram::Legacy.id()),
            );
            state.accounts.insert(
                source.to_string(),
                ui(
                    token(mintkey, owner, kind.is_programmable()),
                    TokenProgram::Legacy.id(),
                ),
            );
            state.tokens.push(source.to_string());
            let master = MasterEdition {
                key: Key::MasterEditionV2,
                supply: 0,
                max_supply: Some(0),
            };
            if edition_kind {
                let parent = key(4);
                let parent_edition = MasterEdition::find_pda(&solana::to_metaplex(parent)).0;
                let parent_token = key(5);
                state.accounts.insert(
                    edition.to_string(),
                    ui(
                        Edition {
                            key: Key::EditionV1,
                            parent: parent_edition,
                            edition: 1,
                        }
                        .try_to_vec()
                        .unwrap(),
                        solana::to_rpc(mpl_token_metadata::ID),
                    ),
                );
                state.accounts.insert(
                    parent_edition.to_string(),
                    ui(
                        master.try_to_vec().unwrap(),
                        solana::to_rpc(mpl_token_metadata::ID),
                    ),
                );
                state.accounts.insert(
                    parent.to_string(),
                    ui(
                        mint(solana::to_rpc(parent_edition)),
                        TokenProgram::Legacy.id(),
                    ),
                );
                state.accounts.insert(
                    parent_token.to_string(),
                    ui(token(parent, owner, false), TokenProgram::Legacy.id()),
                );
                state.parent = Some((parent.to_string(), parent_token.to_string()));
                let marker = Pubkey::find_program_address(
                    &[
                        b"metadata",
                        mpl_token_metadata::ID.as_ref(),
                        parent.as_ref(),
                        b"edition",
                        b"0",
                    ],
                    &solana::to_rpc(mpl_token_metadata::ID),
                )
                .0;
                let mut bytes = vec![Key::EditionMarker as u8];
                bytes.extend([0; 31]);
                bytes[1] = 0x40;
                state.accounts.insert(
                    marker.to_string(),
                    ui(bytes, solana::to_rpc(mpl_token_metadata::ID)),
                );
            } else {
                state.accounts.insert(
                    edition.to_string(),
                    ui(
                        master.try_to_vec().unwrap(),
                        solana::to_rpc(mpl_token_metadata::ID),
                    ),
                );
            }
            if kind.is_programmable() {
                let record = TokenRecord::find_pda(
                    &solana::to_metaplex(mintkey),
                    &solana::to_metaplex(source),
                )
                .0;
                state.accounts.insert(
                    record.to_string(),
                    ui(
                        TokenRecord {
                            key: Key::TokenRecord,
                            bump: 0,
                            state: TokenState::Unlocked,
                            rule_set_revision: None,
                            delegate: None,
                            delegate_role: None,
                            locked_transfer: None,
                        }
                        .try_to_vec()
                        .unwrap(),
                        solana::to_rpc(mpl_token_metadata::ID),
                    ),
                );
            }
            NftTarget {
                id: mintkey.to_string(),
                owner: owner.to_string(),
                standard,
                token_account: Some(source.to_string()),
                mint: Some(mintkey.to_string()),
            }
        }
        NftStandard::Core => {
            let id = key(6);
            let mut raw = vec![mpl_core::types::Key::AssetV1 as u8];
            raw.extend(owner.to_bytes());
            raw.push(0);
            raw.extend("Core fixture".try_to_vec().unwrap());
            raw.extend("https://fixture.invalid".try_to_vec().unwrap());
            raw.push(0);
            state.accounts.insert(
                id.to_string(),
                ui(raw, Pubkey::new_from_array(mpl_core::ID.to_bytes())),
            );
            state.cores.push(id.to_string());
            NftTarget {
                id: id.to_string(),
                owner: owner.to_string(),
                standard,
                token_account: None,
                mint: None,
            }
        }
        NftStandard::Compressed => unreachable!("use compressed_fixture"),
    };
    let sender = Sender {
        chain: Arc::new(Mutex::new(state)),
        owner,
        target,
    };
    (
        sender,
        std::mem::take(&mut asset),
        std::mem::take(&mut proof),
    )
}
fn compressed_fixture(v2: bool, owner: Pubkey) -> (Sender, Value, Value) {
    let tree = key(7);
    let config =
        mpl_bubblegum::accounts::TreeConfig::find_pda(&tree.to_string().parse().unwrap()).0;
    let id = Pubkey::find_program_address(
        &[b"asset", tree.as_ref(), &0u64.to_le_bytes()],
        &Pubkey::new_from_array(mpl_bubblegum::ID.to_bytes()),
    )
    .0;
    let mut state = Chain::default();
    let mut cfg = vec![122, 245, 175, 248, 171, 34, 0, 207];
    cfg.extend(owner.to_bytes());
    cfg.extend(owner.to_bytes());
    cfg.extend(8u64.to_le_bytes());
    cfg.extend(1u64.to_le_bytes());
    cfg.extend([0, 0, u8::from(v2)]);
    cfg.resize(96, 0);
    state.accounts.insert(
        config.to_string(),
        ui(cfg, Pubkey::new_from_array(mpl_bubblegum::ID.to_bytes())),
    );
    let path = 32 * 4 + 8;
    let mut raw = vec![0; 56 + 24 + 8 * path + path + 32 * 6];
    raw[0] = 1;
    raw[2..6].copy_from_slice(&8u32.to_le_bytes());
    raw[6..10].copy_from_slice(&3u32.to_le_bytes());
    state.accounts.insert(
        tree.to_string(),
        ui(
            raw,
            if v2 {
                "mcmt6YrQEMKw8Mw43FmpRLmf7BqRnFMKmAcbxE3xkAW"
            } else {
                "cmtDvXumGCrqC1Age74AVPhSRVXJMd8PJS91L8KbNCK"
            }
            .parse()
            .unwrap(),
        ),
    );
    let collection_hash = mpl_bubblegum::hash::hash_collection_option(None).unwrap();
    let leaf = if v2 {
        mpl_bubblegum::types::LeafSchema::V2 {
            id: id.to_string().parse().unwrap(),
            owner: owner.to_string().parse().unwrap(),
            delegate: owner.to_string().parse().unwrap(),
            nonce: 0,
            data_hash: [8; 32],
            creator_hash: [9; 32],
            collection_hash,
            asset_data_hash: [10; 32],
            flags: 0,
        }
    } else {
        mpl_bubblegum::types::LeafSchema::V1 {
            id: id.to_string().parse().unwrap(),
            owner: owner.to_string().parse().unwrap(),
            delegate: owner.to_string().parse().unwrap(),
            nonce: 0,
            data_hash: [8; 32],
            creator_hash: [9; 32],
        }
    };
    let leaf = leaf.hash();
    let nodes = [[11; 32], [12; 32], [13; 32]];
    let mut root = leaf;
    for node in &nodes {
        root = solana_keccak_hasher::hashv(&[&root, node]).to_bytes();
    }
    let strhash = |h| Pubkey::new_from_array(h).to_string();
    let asset = json!({"id":id.to_string(),"ownership":{"owner":owner.to_string(),"delegated":false},"burnt":false,"compression":{"compressed":true,"tree":tree.to_string(),"leaf_id":0,"data_hash":strhash([8;32]),"creator_hash":strhash([9;32]),"collection_hash":strhash(collection_hash),"asset_data_hash":strhash([10;32]),"flags":0},"grouping":[]});
    let proof = json!({"tree_id":tree.to_string(),"root":strhash(root),"leaf":strhash(leaf),"node_index":8,"proof":nodes.map(strhash)});
    let target = NftTarget {
        id: id.to_string(),
        owner: owner.to_string(),
        standard: NftStandard::Compressed,
        token_account: None,
        mint: None,
    };
    (
        Sender {
            chain: Arc::new(Mutex::new(state)),
            owner,
            target,
        },
        asset,
        proof,
    )
}
#[tokio::test]
async fn every_supported_nft_standard_reaches_its_adapter_and_confirmed_report_without_generic_close()
 {
    for variant in 0..7 {
        let signer = Keypair::new();
        let owner = signer.pubkey();
        let standard = match variant {
            0 => NftStandard::TokenMetadata(AssetKind::NonFungible),
            1 => NftStandard::TokenMetadata(AssetKind::NonFungibleEdition),
            2 => NftStandard::TokenMetadata(AssetKind::ProgrammableNonFungible),
            3 => NftStandard::TokenMetadata(AssetKind::ProgrammableNonFungibleEdition),
            4 => NftStandard::Core,
            _ => NftStandard::Compressed,
        };
        let (sender, asset, proof) = if variant >= 5 {
            compressed_fixture(variant == 6, owner)
        } else {
            fixture(standard.clone(), owner)
        };
        let chain = sender.chain.clone();
        let target = sender.target.clone();
        let das_server = das_server(asset, proof, chain.lock().unwrap().burnt.clone());
        let das = DasClient::new(das_server.url.clone()).unwrap();
        let rpc = RpcClient::new_sender(sender, Default::default());
        let executor = SolanaCleanupExecutor {
            rpc: &rpc,
            das: Some(&das),
        };
        let options = CleanupOptions {
            quote_interval: Duration::ZERO,
            ..Default::default()
        };
        let provider = ScopedSwap::<Jupiter> {
            provider: None,
            mainnet: false,
        };
        let snapshot = dock_flints_core::app::scan_wallet::scan_wallet::<()>(
            &rpc,
            &owner,
            &ScanOptions {
                selection: ScanSelection {
                    all_tokens: true,
                    ..ScanSelection::ALL
                },
                no_prices: true,
            },
            None,
        )
        .await;
        let compressed = categories::CompressedReport {
            items: if standard == NftStandard::Compressed {
                vec![categories::CompressedAsset {
                    id: target.id.clone(),
                    owner: owner.to_string(),
                    name: "cNFT fixture".into(),
                }]
            } else {
                vec![]
            },
            status: ScanStatus::Complete,
        };
        let inventory = inventory::normalize(&snapshot, &compressed);
        assert!(
            inventory
                .iter()
                .any(|a| a.id == target.id || a.mint.as_ref() == Some(&target.id)),
            "variant {variant}: {inventory:?}"
        );
        if variant < 4 {
            assert_eq!(inventory[0].name, "Binary NFT fixture");
        }
        let mut plan = plan::build_plan(
            &owner,
            plan::assets_from_portfolio(&snapshot),
            ScanStatus::Complete,
            vec![],
            &provider,
            &options,
        )
        .await;
        add_nfts_observed(&mut plan, &snapshot, &compressed, &executor, &options, &()).await;
        assert_eq!(plan.nft_entries.len(), 1, "variant {variant}");
        assert!(
            plan.nft_entries[0].prepared.is_some(),
            "variant {variant}: {:?}",
            plan.nft_entries[0]
        );
        assert_eq!(plan.executable_count(), 1);
        assert!(plan.entries.is_empty());
        let report = execute::execute_plan(&plan, &provider, &executor, &signer, &options)
            .await
            .unwrap();
        assert_eq!(report.failed, 0, "variant {variant}: {report:?}");
        assert_eq!(report.completed, 1);
        assert_eq!(report.closed, usize::from(variant < 4));
        assert!(report.accounting_complete);
        assert_eq!(
            report.known_net_wallet_lamports,
            if variant >= 5 { -5000 } else { 4_995_000 }
        );
        assert_eq!(chain.lock().unwrap().signatures.len(), 1);
    }
}
#[tokio::test]
async fn mismatched_proof_and_core_asset_signer_coverage_block_before_submission() {
    let signer = Keypair::new();
    let (sender, asset, mut proof) = compressed_fixture(true, signer.pubkey());
    proof["node_index"] = 9.into();
    let chain = sender.chain.clone();
    let server = das_server(asset, proof, chain.lock().unwrap().burnt.clone());
    let das = DasClient::new(server.url.clone()).unwrap();
    let target = sender.target.clone();
    let rpc = RpcClient::new_sender(sender, Default::default());
    assert!(
        SolanaCleanupExecutor {
            rpc: &rpc,
            das: Some(&das)
        }
        .prepare_nft(&target, &signer.pubkey())
        .await
        .is_err()
    );
    assert!(chain.lock().unwrap().signatures.is_empty());
    let (sender, _, _) = fixture(NftStandard::Core, signer.pubkey());
    let target = sender.target.clone();
    let rpc = RpcClient::new_sender(sender, Default::default());
    let error = SolanaCleanupExecutor {
        rpc: &rpc,
        das: None,
    }
    .prepare_nft(&target, &signer.pubkey())
    .await
    .unwrap_err();
    assert!(error.to_string().contains("Asset Signer"));
}

#[tokio::test]
async fn selected_prints_precede_their_master_and_protected_prints_block_only_the_master() {
    let signer = Keypair::new();
    let owner = signer.pubkey();
    let (sender, _, _) = fixture(
        NftStandard::TokenMetadata(AssetKind::NonFungibleEdition),
        owner,
    );
    {
        use mpl_token_metadata::{
            accounts::{MasterEdition, Metadata},
            types::{Key, TokenStandard},
        };
        let mut state = sender.chain.lock().unwrap();
        let parent = key(4);
        let token = key(5);
        state.tokens.push(token.to_string());
        let metadata = Metadata {
            key: Key::MetadataV1,
            update_authority: solana::to_metaplex(owner),
            mint: solana::to_metaplex(parent),
            name: "Master fixture".into(),
            symbol: "FIX".into(),
            uri: "https://fixture.invalid".into(),
            seller_fee_basis_points: 0,
            creators: None,
            primary_sale_happened: false,
            is_mutable: true,
            edition_nonce: None,
            token_standard: Some(TokenStandard::NonFungible),
            collection: None,
            uses: None,
            collection_details: None,
            programmable_config: None,
        };
        state.accounts.insert(
            Metadata::find_pda(&solana::to_metaplex(parent))
                .0
                .to_string(),
            ui(
                metadata.try_to_vec().unwrap(),
                solana::to_rpc(mpl_token_metadata::ID),
            ),
        );
        state.accounts.insert(
            MasterEdition::find_pda(&solana::to_metaplex(parent))
                .0
                .to_string(),
            ui(
                MasterEdition {
                    key: Key::MasterEditionV2,
                    supply: 1,
                    max_supply: Some(1),
                }
                .try_to_vec()
                .unwrap(),
                solana::to_rpc(mpl_token_metadata::ID),
            ),
        );
    }
    let chain = sender.chain.clone();
    let rpc = RpcClient::new_sender(sender, Default::default());
    let executor = SolanaCleanupExecutor {
        rpc: &rpc,
        das: None,
    };
    let provider = ScopedSwap::<Jupiter> {
        provider: None,
        mainnet: false,
    };
    let compressed = categories::CompressedReport {
        items: vec![],
        status: ScanStatus::Complete,
    };
    let options = CleanupOptions {
        quote_interval: Duration::ZERO,
        ..Default::default()
    };
    let mut protected = options.clone();
    protected.selection.ignored_mints.insert(key(2).to_string());
    let blocked = plan_wallet_complete(&rpc, &owner, &provider, &executor, &protected, &compressed)
        .await
        .unwrap();
    assert_eq!(blocked.executable_count(), 0);
    assert!(
        blocked
            .nft_entries
            .iter()
            .any(|e| e.reason.contains("unselected, unowned or blocked"))
    );
    assert!(chain.lock().unwrap().signatures.is_empty());
    let plan = plan_wallet_complete(&rpc, &owner, &provider, &executor, &options, &compressed)
        .await
        .unwrap();
    assert_eq!(plan.nft_entries[0].target.id, key(2).to_string());
    assert_eq!(plan.nft_entries[1].target.id, key(4).to_string());
    assert_eq!(plan.executable_count(), 2);
    let report = execute::execute_plan(&plan, &provider, &executor, &signer, &options)
        .await
        .unwrap();
    assert_eq!(report.failed, 0, "{report:?}");
    assert_eq!(report.completed, 2);
    assert_eq!(report.closed, 2);
    assert_eq!(chain.lock().unwrap().signatures.len(), 2);
}
