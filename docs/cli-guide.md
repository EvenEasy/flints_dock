# CLI commands and examples

Commands run from the repository root. Cargo defaults to package `dock_flints`; the release binary is `target/release/dock_flints`.

```bash
cargo run --locked -- --help
cargo run --locked -- scan --help
cargo run --locked -- cleanup --help
cargo run --locked -- quote --help
cargo run --locked -- swap --help
```

## Wallet identity and RPC

Each command accepts exactly one of `--pubkey`/`-p`, `--keypair` or `--seed`.

| Input                                  | Purpose                                                                          |
| -------------------------------------- | -------------------------------------------------------------------------------- |
| `--pubkey ADDRESS`                     | Read-only identity for scan, quote and cleanup preview                           |
| `--keypair /absolute/path/wallet.json` | Local Solana JSON keypair; signing is used only by execution commands            |
| `--seed BASE64`                        | Exactly 32 raw Ed25519 bytes in base64; neither a mnemonic nor a 64-byte keypair |

Prefer public addresses for read-only examples. Scan can also derive identity from a keypair without signing. Secret command arguments can appear in shell history/process listings; a file is preferable for signer workflows.

CLI uses `--rpc-url`/`-r`, default `https://api.mainnet.solana.com`. `DOCK_FLINTS_RPC_URL` configures desktop, not CLI. `--timeout-seconds` defaults to `30`, accepts `1…300`, and limits an individual RPC request; retries can extend total duration.

```bash
DEMO_WALLET=9FCR2PU1jZgCHyjWxzk2BNQHJxszAK24vBFiWmUyRNpv
DEVNET_RPC=https://api.devnet.solana.com
```

## Read-only scanning

```bash
# Native SOL
cargo run --locked -- scan --pubkey "$DEMO_WALLET" \
  --rpc-url "$DEVNET_RPC" --balance --no-prices

# Fungible/Unknown assets with backing accounts and exact raw amounts
cargo run --locked -- scan --pubkey "$DEMO_WALLET" \
  --rpc-url "$DEVNET_RPC" --tokens --no-prices --details --show-mint

# Raw token accounts, including empty and NFT backing accounts
cargo run --locked -- scan --pubkey "$DEMO_WALLET" \
  --rpc-url "$DEVNET_RPC" --all-tokens --no-prices --details --include-empty

# Classic, programmable and uncompressed MPL Core NFTs
cargo run --locked -- scan --pubkey "$DEMO_WALLET" \
  --rpc-url "$DEVNET_RPC" --nfts --no-prices --details --show-mint

# Save a complete selected inventory view as JSON
cargo run --locked -- scan --pubkey "$DEMO_WALLET" \
  --rpc-url "$DEVNET_RPC" --all --all-tokens --no-prices \
  --details --include-empty --format json > /tmp/flints-devnet-scan.json
```

JSON stdout is separate from `--verbose` diagnostics on stderr. This local output is a new scan, separate from saved [verification evidence](current-verification.md).

| Flag                           | Selection / effect                                                               |
| ------------------------------ | -------------------------------------------------------------------------------- |
| `--balance`                    | Native SOL                                                                       |
| `--tokens`                     | Fungible SPL/Token-2022 and Unknown; verified NFTs excluded                      |
| `--all-tokens`                 | One row per raw token account; overlaps semantic views                           |
| `--nfts`                       | Classic, programmable and uncompressed Core NFTs                                 |
| `--cnfts`                      | RPC-only compressed capability result; standalone enumeration is not implemented |
| `--all`                        | SOL + tokens + nfts + cnfts; add `--all-tokens` explicitly for raw accounts      |
| `--no-prices`                  | Disable pricing requests                                                         |
| `--include-empty`              | Include zero-balance asset rows                                                  |
| `--details`                    | Raw amounts, decimals, accounts, metadata and account summary                    |
| `--show-mint` / `--show-price` | Full IDs / unit prices in tables                                                 |
| `--format table\|json`         | Table default or structured JSON                                                 |
| `--verbose` / `-v`             | Scanner diagnostics on stderr                                                    |

With no selection flags, scan uses `--all`. Flags combine. The compatible shorthand `cargo run -- -p ADDRESS --tokens` is retained.

Confirmed fungibles aggregate by mint/program; Unknown assets remain visible. Name, decimals `0` or balance `1` alone do not prove NFT status. NFT/Unknown assets do not receive fungible valuations. Default output hides zero-balance rows, but `--details` JSON retains full `discovered_accounts` even without `--include-empty`.

## JSON and status

CLI scan returns `wallet` and selected objects `sol`, `tokens`, `all_tokens`, `nfts`, `cnfts`, each with `status` and optional `reason`. This differs from Tauri `WalletAnalysis`, which includes the UI categories `scam`, `nft`, `dust`, `dead_token`.

Example SOL-only shape, with illustrative values:

```json
{
  "wallet": "PUBLIC_ADDRESS",
  "sol": {
    "status": "complete",
    "lamports": 1000000000,
    "balance": "1",
    "usd_value": null,
    "price": null,
    "pricing": { "status": "skipped", "reason": "Disabled by --no-prices" }
  }
}
```

Raw token amounts and aggregates are decimal strings. Other CLI `u64` fields need an integer-aware JSON parser. Prices/USD are approximate numbers or `null`; missing quotes do not become zero.

Scan exit `0` means at least one selected SOL/token/NFT category produced a complete or partial usable result. It does not guarantee complete coverage. Exit `2` means no usable result; JSON reasons remain available. Invalid input uses the CLI error path.

Standalone `scan --cnfts` currently returns `historical_index_required`, `items: null` and exit `2`. It does not call DAS or read `DOCK_FLINTS_DAS_URL`. Desktop analysis and cleanup planning support DAS; an empty standalone cNFT result must not be inferred.

## Pricing scope

CLI scan calls Jupiter pricing only when valuation is needed, pricing is enabled, RPC genesis verifies mainnet and `JUPITER_API_KEY` is nonempty. `.env` is not loaded automatically. Devnet holdings do not receive mainnet prices.

```bash
# MAINNET_WALLET is a public mainnet address; API key is already in the environment
cargo run --locked -- scan --pubkey "$MAINNET_WALLET" \
  --rpc-url https://api.mainnet.solana.com --tokens --show-price --format json
```

SOL-only valuation requests SOL; NFT-only scan does not construct a price provider. Pricing failure preserves blockchain inventory. See [desktop configuration](../apps/app/README.md) and [swaps](swaps.md) for provider differences.

## Read-only cleanup preview

Use only preview on the documentation devnet wallet:

```bash
cargo run --locked -- cleanup --pubkey "$DEMO_WALLET" \
  --rpc-url "$DEVNET_RPC" --dry-run --format json \
  > /tmp/flints-devnet-cleanup-plan.json
```

Without `--execute`, cleanup is also preview. JSON includes `mode: "dry_run"`, `transactions_submitted: 0` and the plan. Planning can read RPC/providers but does not sign or submit.

| Selection            | Scope                                                            |
| -------------------- | ---------------------------------------------------------------- |
| `--account ADDRESS`  | Selected token accounts; repeatable                              |
| `--ignore-mint MINT` | Keep every backing account of the mint, including empty accounts |
| `--asset ID`         | Standalone NFT IDs; classic NFT ID is the mint                   |
| `--ignore-asset ID`  | Protect Core/compressed IDs                                      |

Fresh backend evidence determines eligibility. Unsupported network is not NoRoute. Structural routing unavailability may produce a separately explained burn/close plan; execution still requires approval. See [cleanup](cleanup.md).

## Quote, swap and execution

These are separate signer-workflow examples, not commands executed on the documentation wallet. Jupiter quote/swap are mainnet-oriented.

```bash
# Read-only quote: exact mint and raw units, no holding or Price V3 required
cargo run --locked -- quote --pubkey "$MAINNET_WALLET" \
  --mint EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v \
  --raw-amount 1000000 --format json
```

For mainnet USDC, `1000000` raw units equals 1 USDC. `quote` has no `--rpc-url`; Jupiter builds the route. Standalone `swap` does not validate RPC genesis before its provider request; choose matching mainnet RPC yourself.

```bash
cargo run --locked -- swap --keypair /absolute/path/to/wallet.json \
  --rpc-url https://api.mainnet.solana.com \
  --mint EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v \
  --raw-amount 1000000 --slippage-bps 50 --max-price-impact-bps 100

cargo run --locked -- cleanup --keypair /absolute/path/to/wallet.json \
  --rpc-url https://api.mainnet.solana.com \
  --ignore-mint EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v --execute
```

Swap shows a preview, requests `y`/`yes`, then obtains a fresh route. Cleanup requests `cleanup`; approval includes irreversible burns. `--yes` is explicit noninteractive approval; cleanup also requires `--execute`. SOL is needed for fees. Defaults: 50 bps slippage, 100 bps max price impact, 1,000,000 lamports priority cap, 90 s confirmation timeout.

Reconcile uncertain sends using the returned signature before repeating. The persistent signature journal is a desktop feature; CLI does not use that journal. See [cleanup](cleanup.md) and [single-token swap guarantees](swaps.md).

## Checks

```bash
cargo fmt --all --check
cargo check --workspace --locked
cargo test --workspace --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
```

Offline fixtures do not establish live provider coverage. The [current report](current-verification.md) separates live devnet, UI/IPC checks and test failures.
