use base64::{Engine, engine::general_purpose::STANDARD};
use clap::{CommandFactory, Parser};
use dock_flints::{
    cli::{Cli, Command},
    infra::wallet::WalletIdentity,
};
use solana_keypair::{Keypair, write_keypair_file};
use solana_signer::Signer;

#[test]
fn identity_is_required_and_exclusive_in_every_command() {
    Cli::command().debug_assert();
    let wallet = Keypair::new().pubkey().to_string();
    let seed = STANDARD.encode([7; 32]);
    for command in [
        vec![],
        vec!["scan"],
        vec!["cleanup"],
        vec!["quote", "--mint", &wallet, "--raw-amount", "1"],
        vec!["swap", "--mint", &wallet, "--raw-amount", "1"],
    ] {
        let mut base = vec!["dock_flints"];
        base.extend(command.iter().copied());
        assert!(
            Cli::try_parse_from(&base).is_err(),
            "missing identity: {base:?}"
        );
        for identity in [
            vec!["--pubkey", &wallet],
            vec!["--keypair", "wallet.json"],
            vec!["--seed", &seed],
        ] {
            let mut args = base.clone();
            args.extend(identity);
            let read_only_swap = command.first() == Some(&"swap") && args.contains(&"--pubkey");
            assert_eq!(Cli::try_parse_from(args).is_ok(), !read_only_swap);
        }
        for pair in [
            vec!["--pubkey", &wallet, "--keypair", "wallet.json"],
            vec!["--pubkey", &wallet, "--seed", &seed],
            vec!["--keypair", "wallet.json", "--seed", &seed],
        ] {
            let mut args = base.clone();
            args.extend(pair);
            assert!(Cli::try_parse_from(args).is_err());
        }
    }
}
#[test]
fn seed_and_keypair_derive_the_same_address_and_readonly_cannot_sign() {
    let signer = Keypair::new_from_array([7; 32]);
    let encoded = STANDARD.encode([7; 32]);
    let from_seed = WalletIdentity::from_seed(&encoded).unwrap();
    assert_eq!(from_seed.address, signer.pubkey());
    assert_eq!(from_seed.signer().unwrap().pubkey(), signer.pubkey());
    let file = std::env::temp_dir().join(format!("dock-identity-test-{}.json", std::process::id()));
    write_keypair_file(&signer, &file).unwrap();
    let from_file = WalletIdentity::from_keypair(&file).unwrap();
    std::fs::remove_file(file).unwrap();
    assert_eq!(from_file.address, from_seed.address);
    assert!(WalletIdentity::read_only(signer.pubkey()).signer().is_err());
    for malformed in [
        "secret-invalid-base64".to_owned(),
        STANDARD.encode([1; 31]),
        STANDARD.encode([1; 33]),
        STANDARD.encode(signer.to_bytes()),
    ] {
        let error = match WalletIdentity::from_seed(&malformed) {
            Ok(_) => panic!("invalid seed accepted"),
            Err(error) => error.to_string(),
        };
        assert!(!error.contains(&malformed));
    }
}
#[test]
fn cleanup_preview_accepts_signer_but_execution_needs_explicit_mode() {
    let seed = STANDARD.encode([7; 32]);
    let preview = Cli::try_parse_from(["dock_flints", "cleanup", "--seed", &seed]).unwrap();
    assert!(matches!(preview.command, Some(Command::Cleanup(args)) if !args.execute));
    assert!(
        Cli::try_parse_from([
            "dock_flints",
            "cleanup",
            "--seed",
            &seed,
            "--execute",
            "--yes"
        ])
        .is_ok()
    );
    assert!(Cli::try_parse_from(["dock_flints", "cleanup", "--seed", &seed, "--yes"]).is_err());
    assert!(
        Cli::try_parse_from([
            "dock_flints",
            "cleanup",
            "--seed",
            &seed,
            "--execute",
            "--dry-run"
        ])
        .is_err()
    );
}
