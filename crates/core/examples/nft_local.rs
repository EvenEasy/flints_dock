//! Create and clean owned classic/pNFT fixtures on loopback only; never accepts an external cluster.
mod support;
use dock_flints_core::{
    app::{
        categories::ScopedSwap,
        cleanup::{execute::execute_plan_observed, plan_wallet_complete},
    },
    core::{ScanStatus, TokenProgram, categories::CompressedReport, cleanup::*},
    infra::{solana, wallet::WalletIdentity},
};
use solana_instruction::{AccountMeta, Instruction};
use solana_keypair::Keypair;
use solana_message::{VersionedMessage, v0};
use solana_signer::Signer;
use solana_transaction::versioned::VersionedTransaction;
use std::collections::BTreeSet;
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<_> = std::env::args().collect();
    anyhow::ensure!(
        (args.len() == 3 || (args.len() == 4 && args[3] == "--prepare-only")),
        "Use LOOPBACK_RPC OWNED_KEYPAIR [--prepare-only]"
    );
    let url = reqwest::Url::parse(&args[1])?;
    anyhow::ensure!(
        url.scheme() == "http" && matches!(url.host_str(), Some("127.0.0.1") | Some("localhost")),
        "Only isolated local validator permitted"
    );
    let wallet = WalletIdentity::from_keypair(std::path::Path::new(&args[2]))?;
    let signer = wallet.signer()?;
    let rpc = solana::client(args[1].clone(), 30);
    let owner = solana::to_metaplex(wallet.address);
    let mut results = vec![];
    let observer =
        support::Journal(std::path::Path::new(&args[2]).with_file_name("nft-submissions.jsonl"));
    for programmable in [false, true] {
        use mpl_token_metadata::{
            accounts::{MasterEdition, Metadata, TokenRecord},
            instructions::{CreateV1Builder, MintV1Builder},
            types::{PrintSupply, TokenStandard},
        };
        let mint = Keypair::new();
        let mintkey = solana::to_metaplex(mint.pubkey());
        let metadata = Metadata::find_pda(&mintkey).0;
        let edition = MasterEdition::find_pda(&mintkey).0;
        let source = solana::cleanup::associated_address(
            &wallet.address,
            &mint.pubkey(),
            TokenProgram::Legacy,
        );
        let token_record = TokenRecord::find_pda(&mintkey, &solana::to_metaplex(source)).0;
        let create = CreateV1Builder::new()
            .metadata(metadata)
            .master_edition(Some(edition))
            .mint(mintkey, true)
            .authority(owner)
            .payer(owner)
            .update_authority(owner, true)
            .spl_token_program(Some(solana::to_metaplex(TokenProgram::Legacy.id())))
            .name("Disposable cleanup NFT".into())
            .uri("https://fixture.invalid/nft.json".into())
            .seller_fee_basis_points(0)
            .token_standard(if programmable {
                TokenStandard::ProgrammableNonFungible
            } else {
                TokenStandard::NonFungible
            })
            .decimals(0)
            .print_supply(PrintSupply::Zero)
            .instruction();
        let mint_ix = MintV1Builder::new()
            .token(solana::to_metaplex(source))
            .token_owner(Some(owner))
            .metadata(metadata)
            .master_edition(Some(edition))
            .token_record(programmable.then_some(token_record))
            .mint(mintkey)
            .authority(owner)
            .payer(owner)
            .amount(1)
            .instruction();
        let mut instructions: Vec<Instruction> = Vec::new();
        instructions.extend([create, mint_ix].into_iter().map(|ix| {
            Instruction {
                program_id: solana::to_rpc(ix.program_id),
                data: ix.data,
                accounts: ix
                    .accounts
                    .into_iter()
                    .map(|a| AccountMeta {
                        pubkey: solana::to_rpc(a.pubkey),
                        is_signer: a.is_signer,
                        is_writable: a.is_writable,
                    })
                    .collect(),
            }
        }));
        let (hash, expiry) = rpc
            .get_latest_blockhash_with_commitment(
                solana_commitment_config::CommitmentConfig::confirmed(),
            )
            .await?;
        let message = v0::Message::try_compile(&wallet.address, &instructions, &[], hash)?;
        let transaction =
            VersionedTransaction::try_new(VersionedMessage::V0(message), &[signer, &mint])?;
        solana::transactions::simulate(&rpc, &transaction).await?;
        let setup_signature =
            solana::transactions::send_confirm(&rpc, &transaction, expiry, &Default::default())
                .await?;
        if args.get(3).is_some() {
            results.push(serde_json::json!({"programmable":programmable,"mint":mint.pubkey().to_string(),"tokenAccount":source.to_string(),"setupSignature":setup_signature}));
            continue;
        }
        let provider = ScopedSwap::<dock_flints_core::infra::jupiter::Jupiter> {
            provider: None,
            mainnet: false,
        };
        let options = CleanupOptions {
            selection: CleanupSelection {
                selected_accounts_only: true,
                accounts: BTreeSet::from([source.to_string()]),
                asset_ids: Some(BTreeSet::from([mint.pubkey().to_string()])),
                ..Default::default()
            },
            quote_interval: std::time::Duration::ZERO,
            ..Default::default()
        };
        let executor = solana::nft_cleanup::SolanaCleanupExecutor {
            rpc: &rpc,
            das: None,
        };
        let compressed = CompressedReport {
            items: vec![],
            status: ScanStatus::Unsupported(
                "No DAS configured; this fixture is uncompressed".into(),
            ),
        };
        let plan = plan_wallet_complete(
            &rpc,
            &wallet.address,
            &provider,
            &executor,
            &options,
            &compressed,
        )
        .await?;
        anyhow::ensure!(
            plan.nft_entries.iter().any(|e| e.prepared.is_some()),
            "NFT adapter blocked: {:?}",
            plan.nft_entries
        );
        let report =
            execute_plan_observed(&plan, &provider, &executor, signer, &options, &observer).await?;
        anyhow::ensure!(
            report.completed == 1 && report.failed == 0 && report.closed == 1,
            "NFT did not complete: {report:?}"
        );
        results.push(serde_json::json!({"programmable":programmable,"mint":mint.pubkey().to_string(),"tokenAccount":source.to_string(),"setupSignature":setup_signature,"plan":plan,"report":report}));
    }
    println!(
        "{}",
        serde_json::to_string_pretty(
            &serde_json::json!({"network":rpc.get_genesis_hash().await?.to_string(),"wallet":wallet.address.to_string(),"results":results})
        )?
    );
    Ok(())
}
