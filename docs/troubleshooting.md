# Troubleshooting

Start with the selected runtime mode and RPC. [Getting started](getting-started.md) provides the devnet workflow; the [desktop README](../apps/app/README.md) lists prerequisites and configuration.

## Wallet connection does not work in the browser

`npm run dev` starts a browser frontend. Real wallet commands require Tauri IPC: use `npm run desktop` in `apps/app`. For sample screens, open `http://127.0.0.1:1420/?preview=1#welcome`.

Preview scanning is static; use the preview menu or hash URL to choose another screen. It does not await RPC and is not a stalled live scan.

## Port 1420 is occupied

Desktop starts its own Vite server. Stop the separate dev server in its launching terminal and retry `npm run desktop`. Production browser preview uses port `1421`.

## Native build cannot find GTK/WebKit or a linker

Install the [system prerequisites](../apps/app/README.md#system-prerequisites). On Linux, check development packages:

```bash
pkg-config --modversion gtk+-3.0 webkit2gtk-4.1
rustc --version
node --version
```

Rust needs edition 2024 support. Node must satisfy [package.json](../apps/app/package.json). `npm ci` uses locked dependency versions.

For disk/memory exhaustion, check available resources and reduce compiler jobs:

```bash
# apps/app
CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0 npm run desktop:build
```

## Desktop contains an old frontend

Use `npm run desktop:build`. The launcher merges `tauri.frontend.conf.json` to embed the current React `dist`; the base Rust config alone is not the current frontend build command.

The default Linux executable is `target/release/dock-flints-app` relative to the repository root. A custom `CARGO_TARGET_DIR` changes that location. `bundle.active=false` currently disables installer packages.

## Editing .env does not change network/providers

Rust does not load `.env` automatically. Export settings before launch and restart:

```bash
# apps/app
export DOCK_FLINTS_RPC_URL=https://api.devnet.solana.com
export DOCK_FLINTS_RPC_TIMEOUT_SECONDS=30
npm run desktop
```

CLI takes `--rpc-url`; `DOCK_FLINTS_RPC_URL` configures desktop. Keep credentials in backend environment variables; `VITE_*` values are public frontend configuration.

## A category shows —

Open its dialog for status, reason and coverage. A dash means no reliable count is available: unsupported network, disabled check, missing provider, auth/rate-limit failure or empty partial results. A complete confirmed-empty list shows `0`.

Devnet does not use Jupiter mainnet pricing/risk/routes. Without matching devnet evidence, SCAM/DUST/DEAD TOKEN dashes are expected. Do not switch the demo wallet’s RPC merely to produce nonzero categories.

NFT discovery is independent. Without DAS, known classic/Core assets remain visible but compressed coverage is incomplete. Empty classic/Core results alone do not prove total NFT=0.

## RPC, authentication or rate limits

Check endpoint availability and network. Analysis has bounded retries for transient genesis/provider errors. Failed network verification is not a verified unsupported network or NoRoute.

After provider recovery, use **CHANGE WALLET / RESCAN**. A rejected whole analysis shows **ANALYSIS INTERRUPTED** with **RETRY SCAN**. Independently discovered assets and partial results appear in details. See [integration limits](../apps/app/BACKEND_GAPS.md).

## DAS configuration fails

`DOCK_FLINTS_DAS_URL` must be an absolute HTTP(S) URL serving the same network. Desktop checks genesis: mismatch is Unsupported; unavailable verification has a failure reason. A malformed URL can reject startup. Correct it or remove the optional variable, then restart.

Standalone CLI `scan --cnfts` does not enumerate DAS inventory. Use desktop analysis for that coverage; see [cNFT guidance](cnfts.md).

## Cleanup execution is disabled

A public-key session is **READ ONLY**: scanning/planning work, execution does not. Use this mode for the documentation wallet. Signer connection is described in the [desktop README](../apps/app/README.md#cleanup-with-a-signer).

Saved plans are session/network-bound and expire. Selection changes require a new plan. Active cleanup blocks wallet change; recovery reads the existing job. The signature journal preserves reconciliation data, but restart does not restore the whole UI session.

## E2E cannot find Chromium

In `apps/app`:

```bash
npx playwright install chromium
npm run test:e2e
```

If installation used `PLAYWRIGHT_BROWSERS_PATH`, use the same value when running tests. Browser tests use mocked/recorded IPC; [native/live verification](current-verification.md) is documented separately.
