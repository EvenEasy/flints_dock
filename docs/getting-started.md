# Getting started

Run Flint’s Dock as a native desktop application, browser design preview or Rust CLI. Use desktop for real UI analysis and CLI for automated read-only inventory.

## Prerequisites

- Rust with edition 2024 support and Cargo. No `rust-toolchain.toml` currently pins a toolchain version.
- Node.js matching `apps/app/package.json`: `^22.22.2 || ^24.15.0 || >=26.0.0`, and npm.
- Native Tauri build dependencies, including GTK/WebKit on Linux. Installation commands are in the [desktop README](../apps/app/README.md#system-prerequisites).
- Access to the selected Solana RPC. Provider failures do not confirm an empty wallet.

```bash
rustc --version
cargo --version
node --version
npm --version
```

Commands start from the repository root unless another directory is specified.

## First desktop devnet scan

```bash
cd apps/app
npm ci
export DOCK_FLINTS_RPC_URL=https://api.devnet.solana.com
export DOCK_FLINTS_RPC_TIMEOUT_SECONDS=30
npm run desktop
```

The launcher starts Vite and the native Tauri window. Stop any separate dev server using port `1420` first. Rust inherits settings from the launching shell; `.env` is not loaded automatically.

1. Click **CONNECT WALLET** and choose **Public key (read only)**.
2. Enter `9FCR2PU1jZgCHyjWxzk2BNQHJxszAK24vBFiWmUyRNpv`.
3. Keep all inventory selections enabled for SOL, token accounts, NFT/Core and compressed coverage. Uncheck **Include Jupiter USD prices** to disable valuation while keeping discovery active.
4. Click **SCAN WALLET** and wait for analysis. Progress reflects backend stages.
5. Open **SCAM**, **NFT**, **DUST** or **DEAD TOKEN** for items, evidence, reasons and coverage.
6. Open **HOLDS** for token/raw-account/NFT/cNFT inventory, or **PROFILE** for the public wallet identity, balance, pricing diagnostics and rescan/disconnect controls.

The address belongs to the local `dev/demo-wallet.json`; the file is unnecessary for this workflow. If Solana CLI is installed, derive only the public address:

```bash
solana-keygen pubkey dev/demo-wallet.json
```

The documentation session uses public-key connection and read-only scanning. The [current report](current-verification.md) records observed results; a new scan can produce different balances/items. Captures appear in the [gallery](screenshots/README.md).

## Reading counters

Counts describe unique assets, not token-account rows. A mint/program holding can have several backing accounts. Repeated NFT IDs count once; categories may overlap.

| Display                                | Meaning                                                                                         |
| -------------------------------------- | ----------------------------------------------------------------------------------------------- |
| `0`                                    | Complete check with a confirmed empty list                                                      |
| Positive integer                       | Confirmed unique items in the category result                                                   |
| Positive integer with partial coverage | Items already found; the eventual total may be larger                                           |
| `—`                                    | No reliable count/items: initial, missing, failed, skipped, unsupported or empty partial result |

Status words never replace the numeric field. Reasons appear in the tile tooltip/accessible description and category dialog. Rescan clears old results and stale responses cannot populate another wallet’s counters.

Devnet does not use Jupiter mainnet price/risk/route evidence. SCAM/DUST/DEAD TOKEN may show `—` with specific reasons. A missing price is not a zero price; missing routing evidence is not NoRoute.

NFT discovery is independent of pricing/risk. Without DAS, known classic/Core NFTs remain visible, but compressed coverage is incomplete. Empty classic/Core discovery alone does not prove total `NFT = 0`.

## Cleanup preview

**RECOVER SOL** on the main screen opens cleanup. Review inventory, select assets and inspect the Rust plan containing Swap/Burn/Close/Skip actions and reasons. Checked means include; unchecked means keep.

Public-key sessions can prepare plans but cannot execute them. With a signer, the final **RECOVER SOL** approves and executes the displayed saved plan, including irreversible burns. Connecting alone does not authorize transactions. See [desktop usage](../apps/app/README.md) and [cleanup](cleanup.md) for expiry, fees and accounting.

## Browser design preview

In `apps/app`:

```bash
npm run dev
```

Open `http://127.0.0.1:1420/?preview=1`. Welcome, scanning, main, cleanup and success are available through the preview menu and hash URLs. Open processing directly at `http://127.0.0.1:1420/?preview=1#processing`.

Preview scanning is static and does not start RPC or automatically advance. Preview cleanup goes directly to sample success; processing is a separate presentation screen. Values and progress are samples, not devnet results.

```bash
npm run dev -- --width=390 --no-backdrop
```

Display options support `--width=360…480` and `--backdrop`/`--no-backdrop`; default width is `430` CSS px. See the [gallery](screenshots/README.md) for capture provenance and the [desktop README](../apps/app/README.md) for native display options.

## CLI and builds

From the repository root:

```bash
cargo run --locked -- scan --help
cargo run --locked -- scan \
  --pubkey 9FCR2PU1jZgCHyjWxzk2BNQHJxszAK24vBFiWmUyRNpv \
  --rpc-url https://api.devnet.solana.com \
  --tokens --all-tokens --no-prices --details --include-empty
cargo build --release --locked -p dock_flints
```

In `apps/app`:

```bash
npm run build
npm run desktop:build
```

CLI release binary: `target/release/dock_flints`. Use `desktop:build` to embed the current React `dist` into Tauri. Next: [CLI examples](cli-guide.md), [desktop configuration](../apps/app/README.md), [troubleshooting](troubleshooting.md).
