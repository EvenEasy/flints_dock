# Flint’s Dock desktop

React + TypeScript + Vite renders the phone-sized UI. Tauri owns wallet sessions,
provider configuration, stored cleanup plans, jobs and the signature journal.
Discovery, classification, swap/burn/close and accounting run in `crates/core`;
the CLI continues to use the same core without Tauri or React.

## Run

Use the Node version range in `package.json` and the Rust workspace toolchain.
Linux additionally requires the Tauri GTK/WebKit development libraries.

```sh
cd apps/app
npm ci
npm run desktop
```

`npm run dev` is a browser/design preview, not a signing application. Open
`http://127.0.0.1:1420/?preview=1` for demonstration screens. Preview amounts and
progress never become a live execution result. Stop an existing dev server before
starting another on port 1420; the desktop launcher also starts Vite.

Display options work with `dev`, `preview`, `desktop` and `desktop:build`:

```sh
npm run desktop -- --width=390 --backdrop
npm run desktop -- --width=430 --no-backdrop
npm run desktop:build
```

Widths are 360–480 CSS px, default 430. The app stays centered on the desktop star
background and fills narrow viewports. Short screens reduce decoration; asset
lists scroll independently. `?layout=reference&preview=1` retains the design review
layout. Production uses the phone layout. The launcher merges
`tauri.frontend.conf.json` with the base Rust configuration; use the launcher to
embed `dist`, rather than compiling the backend-only base config directly.

## Backend configuration

Rust reads **inherited process environment variables**. It does not load `.env`
automatically. `backend.env.example` documents the settings; export them in the
shell launching the app, or configure your desktop launch environment. Never put
credentials in `VITE_*`, frontend files, URLs rendered in the UI, or git.

| Variable | Default / purpose |
| --- | --- |
| `DOCK_FLINTS_RPC_URL` | `https://api.mainnet.solana.com`; HTTP(S) Solana RPC |
| `DOCK_FLINTS_RPC_TIMEOUT_SECONDS` | 30; allowed 1–300 |
| `JUPITER_API_KEY` | Rust-only `x-api-key`; configure for mainnet Price/Tokens/Swap access; keyless availability is provider-controlled |
| `DOCK_FLINTS_DAS_URL` | Optional HTTP(S) DAS endpoint supporting `getGenesisHash`, `getAssetsByOwner`, `getAsset`, `getAssetProof`; credentials stay in Rust |
| `DOCK_FLINTS_DUST_USD` | `0.01`; finite positive USD dust threshold |
| `DOCK_FLINTS_JOURNAL_PATH` | `$XDG_DATA_HOME/dock-flints/signatures.json`, falling back to `$HOME/.local/share/dock-flints/signatures.json` |

Example, without credentials:

```sh
export DOCK_FLINTS_RPC_URL=https://api.devnet.solana.com
export DOCK_FLINTS_RPC_TIMEOUT_SECONDS=30
npm run desktop
```

The RPC genesis hash determines the actual network. Jupiter prices, risk and
routes are enabled only for verified mainnet. A devnet RPC does not make Jupiter
a devnet provider. One cleanup strategy applies on every cluster: close eligible empty
accounts, unwrap selected WSOL, swap fungibles where routes exist, and plan burn/close
when a route is explicitly absent or swap capability is structurally unsupported.
Temporary provider/auth/rate-limit failures block that asset, rather than authorize
burn. Review the displayed Swap/Burn/Close counts and irreversible burn notice:
**RECOVER SOL approves and executes that exact saved plan**. There is no policy
selector or second approval dialog. A public key can preview but cannot execute.
DAS must report the same genesis hash as the RPC, or its inventory is unavailable.
RPC errors redact endpoint URLs; avoid exposing credentials in diagnostics.

## Wallet and cleanup flow

Connect with a public key (read-only), a standard base64 encoding of **32 raw
Ed25519 seed bytes**, or an absolute path to a local Solana JSON keypair file.
The seed is cleared from the input after submission. Rust retains the signer for
the session and returns only session ID, public address, source kind and `canSign`.
Connection itself does not authorize transactions.

Analysis discovers legacy SPL and Token-2022 accounts once, then enriches that
snapshot. Missing prices leave holdings visible. Open a category tile to inspect
its real items and coverage/evidence. Standard-aware NFT burns are available; unsupported evidence/plugins/proofs remain visible. See [cleanup coverage](../../docs/cleanup.md).

In cleanup, a checked asset means **include**, unchecked means **keep**. A protected
mint keeps every backing account, including empty accounts. A single Rust inventory aggregates fungibles by mint/program and retains every
backing account. Core/cNFT selection uses their asset IDs. Rust resolves these IDs
into server-owned targets. IPC distinguishes `all`, `selected` and `none`;
empty `selected` means none even though the preserved CLI core allowlist semantics
use an empty list for all. The backend also rejects empty execution.

Selection changes invalidate old plans. `prepare_cleanup` stores a read-only plan
in Rust, bound to wallet/session/network/revision, for 120 seconds after planning.
The UI shows swap, burn, close and skip actions and estimates before fees. A public
key can prepare but cannot execute. The screen explicitly states irreversible burning before RECOVER SOL. `execute_cleanup` accepts only the
stored plan ID and approval of its actions, never frontend-authored transactions.
Default limits: 50 bps slippage, 100 bps maximum price impact and 1,000,000 lamports
maximum priority fee. Each swap re-quotes and preserves the approved minimum.

Execution is sequential. It simulates and uses preflight, verifies token identity,
amount, program, authorities and supported extensions, confirms operations, checks
zero balance before close, and returns rent to the owner. A planned fungible burn rechecks the saved route-absence or structural-capability
reason and the exact on-chain identity, amount and eligibility. NFT adapters refresh
metadata/plugins/proofs and compare the approved identity.
Failed swaps are never converted to burns. Individual failures
remain in the report. Duplicate execute returns the same job. Wallet changes and
disconnect are blocked while a job is active.

Typed channels carry actual core stages and operation counts. Recovery reads the
existing job; it never repeats execute. Closing a screen does not cancel a sent
transaction. The final SOL figure is an exact signed sum of this job’s transaction
metadata, including burn/close fees and confirmed failed-transaction fees. It is
neither a quote nor a whole-wallet before/after difference. Missing metadata makes
accounting incomplete. Partial/failed results have distinct headings. Inventory is
refreshed after execution.

Before the first send, Rust durably journals the nonsensitive signature, wallet,
network, source account, operation and expiry. It records no seed, transaction
bytes or keypair. An OS file lock protects concurrent instances. On later execution
Rust checks pending signatures and metadata before proceeding; uncertain sends
are not resubmitted. Do not delete this journal to bypass unresolved submissions.
Plans and full job reports are currently in memory: restart reconciliation retains
transaction safety and deltas, but does not restore the old React session/report.
Journal durability and native rendering have been tested on Linux; other OSes
need their native validation.

## Category rules and provider coverage

- **SCAM** means provider-flagged suspicious, only an explicit optional
  `audit.isSus: true` from exact-mint Jupiter Tokens V2 observations. False is a
  negative observation; absent record/field is unknown. Verification, authorities
  and organic score alone do not establish scam. The published V2 response schema
  does not currently guarantee `isSus`, so unknown coverage is expected.
- **DUST** means a nonzero fungible mint/program holding with aggregate available
  valuation `0 < value <= DOCK_FLINTS_DUST_USD`, in USD. Unpriced holdings are not
  dust. This does not estimate whether a swap is economical after fees.
- **DEAD TOKEN** means a nonzero fungible holding with explicit no-route evidence
  from Jupiter Swap V2 `/build` for its exact mint/raw amount at the recorded time.
  It is provider/time scoped, not a claim of permanent worthlessness. Missing
  prices, HTTP failures, liquidity errors and malformed success responses remain
  unknown. Empty accounts are not dead tokens.
- **NFT** combines verified classic/programmable/Core inventory and read-only DAS
  compressed inventory. DAS paginates with bounded retries/timeouts, checks owner
  and burned state, deduplicates IDs and excludes noncompressed records already
  discoverable on chain. A missing/partial DAS scan is not zero NFTs.

Synthetic category observations exist only in test fixtures. Production analysis does
not consume `DOCK_FLINTS_DEVNET_TEST_MANIFEST`. Devnet does not acquire invented risk,
valuation or market evidence. NFT coverage independently reports classic, Core and
compressed checks, preserving known items when DAS is unavailable.

Counts describe assets, not token accounts. Mint/program holdings aggregate all
backing accounts; category flags can overlap. Complete checks may display zero;
partial checks display available lower bounds and a reason. RPC, pricing, risk,
routing and DAS have independent statuses. Category membership never grants burn
approval. Price requests batch 50 mints, risk requests 100. A shared Jupiter limiter
paces Price/Tokens/Swap with bounded retries and provider rate-limit headers.
Prices cache for 30 seconds and risks for 60, keyed by mainnet/mint; swap builds
are always fresh. Routing is sequential, so large inventories can take time.

Official references: [Tokens lookup](https://developers.jup.ag/docs/tokens/token-information),
[Tokens response schema](https://developers.jup.ag/docs/api-reference/tokens/search),
[Swap V2 build](https://developers.jup.ag/docs/swap/build),
[shared rate limits](https://developers.jup.ag/docs/portal/rate-limits),
[DAS paging](https://www.helius.dev/docs/api-reference/das/getassetsbyowner).

## Boundaries and verification

`frontend-contract` is the typed IPC boundary. Only the local `main` window is
allowed the six identity/analysis/cleanup commands registered in `lib.rs` and
`build.rs`. There is no shell plugin, general filesystem command, remote-origin
permission or JavaScript swap/burn implementation. All exact on-chain amounts
cross IPC as decimal strings; USD valuations remain approximate numbers.

```sh
# Repository root
cargo fmt --all --check
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets --all-features

# apps/app
npm ci
npm run check
npm run format:check
npm run test:e2e
npm run desktop:build
```

The Rust IPC regression suite uses Tauri MockRuntime and capability enforcement
with local mock RPC. Browser tests mock IPC for execution. Native validation and
raster diagnosis, including limitations, are recorded in
[desktop verification](docs/desktop-verification.md). No test requires sending
transactions to a user’s mainnet wallet.

Artwork provenance and missing original layers are documented in
[art inventory](assets/art/README.md). `?diagnostics=art` provides a **dev-only**
plain-image/component comparison; it is absent from production rendering.

Devnet verification and the complete public-wallet account audit are recorded in
[devnet cleanup verification](../../docs/devnet-cleanup-verification.md).
