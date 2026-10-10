# Flint’s Dock desktop

React + TypeScript + Vite render the UI. Tauri 2 manages wallet sessions, provider configuration, cleanup plans/jobs and the signature journal. Shared Rust core implements discovery, classification and swap/burn/close.

Start with [getting started](../../docs/getting-started.md). The [gallery](../../docs/screenshots/README.md) labels screenshot data sources; [current verification](../../docs/current-verification.md) records completed checks and limitations. Developer references: [frontend](FRONTEND.md), [IPC contract](frontend-contract/README.md), [integration limits](BACKEND_GAPS.md).

## System prerequisites

Install Rust/Cargo, Node matching [package.json](package.json), and platform libraries. For Ubuntu/Debian:

```sh
sudo apt-get update
sudo apt-get install build-essential pkg-config libwebkit2gtk-4.1-dev \
  libssl-dev libxdo-dev libayatana-appindicator3-dev librsvg2-dev curl wget file
```

macOS needs Xcode Command Line Tools; Windows needs Microsoft C++ Build Tools with Desktop development with C++, WebView2 and the Rust MSVC toolchain. See [official Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) for other distributions/platforms.

Solana CLI is unnecessary for public-address scans; examples use `solana-keygen` only to derive a local public key. Commands using `export`, inline environment variables and POSIX paths assume Linux/macOS shells; adapt environment syntax on Windows.

## Launch and build

Node ranges: `^22.22.2`, `^24.15.0` or `>=26.0.0`. In `apps/app`:

```sh
npm ci

# Desktop: launcher starts both Vite and Rust
DOCK_FLINTS_RPC_URL=https://api.devnet.solana.com npm run desktop

# Browser development preview without native IPC
npm run dev
# http://127.0.0.1:1420/?preview=1#welcome

# Browser production preview
npm run build
npm run preview
# http://127.0.0.1:1421/?preview=1#main

# Desktop with embedded React dist
npm run desktop:build
DOCK_FLINTS_RPC_URL=https://api.devnet.solana.com ../../target/release/dock-flints-app
```

Stop a separate Vite server on port 1420 before running desktop. Real local-wallet connection and analysis work in the Tauri window. `?preview=1` uses sample values and progress.

`desktop:build` merges `tauri.frontend.conf.json` to embed the current frontend. The default Linux binary path above assumes no custom `CARGO_TARGET_DIR`. `bundle.active=false`: installer packages are not currently produced. `--debug` forwarded to `desktop:build` builds an embedded executable under `target/debug` instead.

## Display options

```sh
npm run desktop -- --width=390 --no-backdrop
npm run desktop -- --width=430 --backdrop
npm run dev -- --width=480 --backdrop
npm run desktop:build -- --width=390 --no-backdrop
```

Supported by `dev`, `preview`, `desktop` and `desktop:build`. Width: **360–480 CSS px**, default **430**. Star backdrop is enabled by default. URL overrides: `?width=390&backdrop=0`; design reference: `?layout=reference&preview=1`. The phone layout fits narrow viewports; asset lists scroll independently.

## Backend configuration

Rust reads **inherited environment variables**. `.env` and [backend.env.example](backend.env.example) are not loaded automatically. Export settings before launch and restart after changes. Credentials belong to Rust environment variables, never `VITE_*`, frontend source or screenshot URLs.

| Variable                          | Default / purpose                                                                                          |
| --------------------------------- | ---------------------------------------------------------------------------------------------------------- |
| `DOCK_FLINTS_RPC_URL`             | `https://api.mainnet.solana.com`; absolute HTTP(S) RPC                                                     |
| `DOCK_FLINTS_RPC_TIMEOUT_SECONDS` | `30`; allowed 1–300 seconds                                                                                |
| `JUPITER_API_KEY`                 | Optional Rust-only `x-api-key` for mainnet Price/Tokens/Swap; provider determines keyless availability     |
| `DOCK_FLINTS_DAS_URL`             | Optional matching-network DAS supporting `getGenesisHash`, `getAssetsByOwner`, `getAsset`, `getAssetProof` |
| `DOCK_FLINTS_DUST_USD`            | `0.01`; finite positive USD threshold                                                                      |
| `DOCK_FLINTS_JOURNAL_PATH`        | `$XDG_DATA_HOME/dock-flints/signatures.json`, fallback `$HOME/.local/share/dock-flints/signatures.json`    |

```sh
export DOCK_FLINTS_RPC_URL=https://api.devnet.solana.com
export DOCK_FLINTS_RPC_TIMEOUT_SECONDS=30
npm run desktop
```

For a mainnet key without placing its value in shell history:

```sh
read -rsp 'Jupiter API key: ' JUPITER_API_KEY
export JUPITER_API_KEY
npm run desktop
```

RPC genesis determines the network. Jupiter prices/risk/routes use **verified mainnet** only. Analysis retries transient genesis failures up to three times with bounded timeout/backoff; failed verification preserves independent inventory and is not treated as verified Unsupported or NoRoute. Scoped coverage can remain partial where some checks succeeded; see the current regression failure in the [report](../../docs/current-verification.md).

Without DAS, compressed coverage is Unsupported and classic/Core assets remain available. DAS verifies matching genesis. Malformed DAS URLs reject **startup**: correct/remove the setting. Malformed Jupiter credentials are isolated from RPC discovery. Provider errors retain diagnostics.

## Read-only devnet walkthrough

Public address of the local documentation wallet:

```text
9FCR2PU1jZgCHyjWxzk2BNQHJxszAK24vBFiWmUyRNpv
```

1. Launch desktop with devnet RPC.
2. Click **CONNECT WALLET**, select **Public key (read only)** and enter the address.
3. Keep **Native SOL**, **Tokens**, **All token-account assets**, **NFTs / MPL Core** and **Compressed NFT capability** enabled. Prices default to enabled; devnet records their mainnet-only reason.
4. Click **SCAN WALLET**. Open a category for items/evidence/coverage.
5. **HOLDS** opens **TOKENS**, **ALL ACCOUNTS**, **NFT / CORE**, **cNFT**. **PROFILE** shows **PUBLIC ADDRESS · READ ONLY**, balance, diagnostics, **CHANGE WALLET / RESCAN** and **DISCONNECT WALLET**.
6. **RECOVER SOL** opens planning. A public-key session cannot execute; the final execution button is disabled with **Read-only wallet**.

Keypair contents and seeds are unnecessary. Real balances can change. Missing devnet valuation/risk/route evidence produces `—`, not invented zero. Missing DAS does not prove absence of compressed NFTs.

A rejected analysis shows **ANALYSIS INTERRUPTED**, **RETRY SCAN** and **CHANGE WALLET**. Partial scanner results remain in details. Rescan clears old results and late responses cannot replace a new wallet. Dismiss discards the UI response; it does not cancel backend RPC.

## Category counters

Tiles and CategoryDialog consume **one normalized category result**. Keys: `scam`, `nft`, `dust`, `dead_token`. Counts describe unique assets, not backing-account rows; categories can overlap.

| Evidence                                                                       | Numeric field     |
| ------------------------------------------------------------------------------ | ----------------- |
| Complete with confirmed items                                                  | Unique item count |
| Complete confirmed-empty list                                                  | `0`               |
| Partial with confirmed items                                                   | Number found      |
| Empty partial / failed / unsupported / skipped without retained reliable items | `—`               |
| Missing result / scan start                                                    | `—`               |

All counters have the same numeric typography. No `Partial`, `Not checked`, `Unavailable`, `N/A` or `+` appears inside numeric fields. Tooltip, accessible description and details retain status, reason and coverage. A partial number is a lower bound.

- **SCAM:** explicit suspicious signal with source, currently `audit.isSus: true` for an exact mint in Jupiter Tokens V2. False is negative evidence; missing fields/records are unknown. Names, decimals, authorities and organic score are insufficient.
- **DUST:** nonzero fungible mint/program holding with reliable aggregate valuation `0 < value <= DOCK_FLINTS_DUST_USD`. Unpriced holdings are not dust.
- **DEAD TOKEN:** explicit Jupiter Swap V2 `/build` NoRoute for an exact mint/raw amount in recorded provider/time scope. It does not imply permanent worthlessness. Provider/parsing errors are unknown.
- **NFT:** verified standard and ownership for classic/programmable/Core and owner-verified unburned compressed assets via DAS. Unknown compressed coverage prevents an otherwise empty result from proving total zero.

RPC, prices, risk, routing and DAS retain independent statuses. Pricing/risk failures do not block NFT; routing failures do not hide reliable DUST. Price batches: 50 mints; risk: 100. Bounded Jupiter retries share a limiter. Mainnet/mint caches: prices 30 seconds, risk 60. Routing is sequential; swap execution builds fresh routes. Production analysis does not load synthetic observations.

## Cleanup with a signer

Desktop also accepts **File (keypair)** with an absolute local path or **Seed (base64)** containing exactly 32 raw Ed25519 bytes. Seed is not a mnemonic or 64-byte keypair. Rust stores the signer; React clears seed input after submission. Connection alone does not authorize transactions.

Checked assets are included; unchecked assets are kept. React submits canonical `assetIds`/`ignoredAssetIds`; backend also supports mint selection. Ignoring a mint protects all backing accounts, including empty ones. IPC distinguishes `all`, `selected`, `none`; empty selected means none.

`prepare_cleanup` creates a read-only immutable plan bound to wallet/session/network/revision, valid for 120 seconds after planning. The UI displays Swap/Burn/Close/Skip, reasons and estimates **before fees**. Selection changes invalidate the plan. Public key can prepare, not execute.

The final **RECOVER SOL** approves and executes the displayed saved plan. Required burns carry an irreversible-action notice. There is no separate second approval dialog or policy selector. Rust accepts a plan ID and action approval, not frontend-authored transactions. Defaults: 50 bps slippage, 100 bps max price impact, 1,000,000 lamports maximum priority fee.

Execution is sequential: fresh quotes, identity/amount/authority checks, simulation, preflight, confirmation and zero-balance reread before close. Unsupported plugins/extensions/proofs remain skip/review reasons. Failed swaps and temporary provider failures do not authorize fallback burns. See [cleanup coverage](../../docs/cleanup.md).

Recovery reads an existing job without repeating execution. Active jobs block wallet change/disconnect. Final SOL is the signed sum of per-transaction wallet deltas, including fees; missing metadata marks accounting incomplete. Partial/failed jobs have their own reports. Inventory refreshes after execution.

Before send, Rust durably stores nonsensitive signature/owner/network/source/operation/expiry. An OS file lock coordinates instances. Uncertain sends are not repeated. The journal contains no keypair, seed or transaction bytes. Full plans/reports remain in memory: restart preserves reconciliation safety but does not restore the old React session. This persistent journal is a desktop feature, not CLI-wide behavior.

## Checks and evidence

```sh
# apps/app
npm run check
npm run format:check
npx playwright install chromium
npm run test:e2e
npm run desktop:build

# Repository root
cargo fmt --all --check
cargo check --workspace --locked
cargo test --workspace --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
```

Browser tests use mocked/recorded IPC. Registered Tauri IPC tests use MockRuntime with capability enforcement. The [native fixture](tests/native/README.md) uses isolated loopback RPC. Live devnet capture is documented separately.

- [Current verification](../../docs/current-verification.md)
- [Screenshots and provenance](../../docs/screenshots/README.md)
- [Historical desktop verification](docs/desktop-verification.md)
- [Historical devnet audit](../../docs/devnet-cleanup-verification.md)
- [Historical unified/local-validator verification](../../docs/unified-cleanup-verification.md)
- [Artwork provenance](assets/art/README.md)

`?diagnostics=art` is development-only. Linux native evidence does not establish macOS/Windows behavior. See [troubleshooting](../../docs/troubleshooting.md) for common issues.

## Licensing

Application source code is [Apache-2.0](../../LICENSE), with attribution in [NOTICE](../../NOTICE). [Listed artwork](../../ASSETS_LICENSE.md) has separate terms, and [branding](../../BRANDING.md) must not suggest an official third-party product. Third-party resources retain their own licenses. This mixed package uses [LICENSING.md](LICENSING.md) as its metadata license statement.
