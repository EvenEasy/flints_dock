# Current verification and documentation materials

Application code: `e013ea9e8b93f3ffc1a90c0890d0ce72e5686e06`, carried forward in documentation commit `1cfc3e5a51ac0261abc81b2e7da5f8b261530fb0`. The current working copy updates English documentation, licensing metadata and capture tools. Capture JSON records its own UTC timestamps and source revision; wallet balances and provider availability can change.

## Completed checks and known failure

Linux, Node `v26.11.1`, npm `12.2.0`, Rust/Cargo `1.98.0`.

| Check                                     | Result                                                                                                                                    |
| ----------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| Earlier `npm run check` in `apps/app`     | TypeScript, ESLint, 68 unit/component tests across 7 files, Vite production build passed on this application code                         |
| Earlier full browser E2E                  | **99 passed**, 12.5 minutes; Playwright `.last-run.json` reports `passed`                                                                 |
| Current documentation/capture checks      | ESLint and `npm run format:check` passed; Python helper syntax, metadata and local links validated                                        |
| `cargo fmt --all --check`                 | Passed on the application code                                                                                                            |
| Earlier `cargo test --workspace --locked` | **Not passing**: 24 tests passed, one failed, one ignored before the run stopped                                                          |
| Native embedded-frontend build            | `desktop:build -- --debug --no-backdrop --width=430` completed using existing compiled dependencies                                       |
| Live native capture                       | Public-key connection and registered `analyze_wallet` succeeded; category counters/details/fonts/tile sizes checked at 430 and 360 CSS px |
| Browser preview capture                   | 12 PNGs, six screens at 430/360 CSS px, DPR 2; numeric fields contain only digits/`—`, horizontal overflow zero                           |

The failing Rust test is `exhausted_genesis_failure_is_failed_scope_and_does_not_discard_independent_nfts` in `apps/app/src-tauri/tests/network_analysis.rs:211`. It expects category status `failed`, but receives `partial`. The workspace test command stopped there, so later test groups are not claimed as completed. No product-code fix was made as part of documentation/licensing work. The ignored `live_devnet_readonly_report` requires live devnet RPC for public scan/planning and was not invoked by the workspace run; the separate native harness performed a real public-key scan.

Browser E2E use mocked/recorded IPC to test React counts/details, missing/zero/partial results, deduplication, NFT fallback, stale scans, cleanup contracts and phone layouts. They are not live wallet scans. Registered-IPC MockRuntime tests and the real native capture below are separate evidence sources.

## Real read-only devnet evidence

Public address of the local documentation wallet:

```text
9FCR2PU1jZgCHyjWxzk2BNQHJxszAK24vBFiWmUyRNpv
```

The native app was launched with `https://api.devnet.solana.com`, no DAS endpoint, no Jupiter key and no synthetic observation manifest. The harness records only public-key connection and analysis, checks `canSign=false`, and submits **zero transactions**. It never loads the keypair file or prepares/executes cleanup.

[Authentic native audit](fixtures/current-native-devnet-readonly.json), [430 capture audit](screenshots/devnet/audit.json) and [360 capture audit](screenshots/devnet/phone-360/audit.json) contain requests, registered IPC responses, category details and measurements. The 360 set reuses the same real snapshot. Origin is `tauri://localhost`, not a browser mock.

Observed SOL balance: **18.3448806 SOL**, exact lamports `18344880600`. Inventory contains **15 token accounts**. The visible potential-rent estimate is **0.0074422 SOL**, an inventory estimate rather than an approved cleanup plan or guaranteed net recovery. Classic/Core discovery found no NFTs; compressed coverage remained unavailable.

### Actual category DTO → tile text

All four category results have `items: []` and `count: 0`. Their statuses determine whether that zero is complete evidence:

| Key          | DTO status  | Numeric field | Reason                                                        |
| ------------ | ----------- | ------------- | ------------------------------------------------------------- |
| `scam`       | unsupported | `—`           | Mainnet risk provider unavailable on this network             |
| `nft`        | partial     | `—`           | DAS endpoint not configured; compressed inventory unavailable |
| `dust`       | unsupported | `—`           | Jupiter valuation is mainnet-only                             |
| `dead_token` | unsupported | `—`           | Jupiter routing is mainnet-only                               |

Actual NFT fragment, with ancillary scope/timestamp fields omitted:

```json
{
  "items": [],
  "count": 0,
  "status": {
    "status": "partial",
    "reason": "DAS: DAS endpoint not configured; compressed NFT inventory unavailable"
  },
  "coverage": {
    "das": {
      "status": "unsupported",
      "reason": "DAS endpoint not configured; compressed NFT inventory unavailable"
    },
    "nft_classic": { "status": "complete" },
    "nft_core": { "status": "complete" }
  }
}
```

The tile correctly shows `—`: confirmed-empty classic/Core lists do not prove an empty compressed inventory. The full DTO and visible details retain reasons/coverage. No status words appear inside numeric fields. Counts and detail items use the same normalized category result.

### CLI evidence is a different contract

A fresh scan with the CLI compiled from this application code was saved in [current-cli-devnet-scan.json](fixtures/current-cli-devnet-scan.json): exit `0`, SOL complete, 15 token rows and 15 raw-account rows, zero classic/Core NFTs, standalone cNFT capability Unsupported with `items: null` / `historical_index_required`. Prices were explicitly disabled; USD values remain null.

```bash
cargo run --locked -- scan \
  --pubkey 9FCR2PU1jZgCHyjWxzk2BNQHJxszAK24vBFiWmUyRNpv \
  --rpc-url https://api.devnet.solana.com \
  --all --all-tokens --details --include-empty --no-prices --format json
```

CLI terminal JSON is not Tauri category DTO and its empty on-chain NFT list does not contradict the desktop’s incomplete overall NFT coverage.

## Screenshot provenance

[Gallery](screenshots/README.md): **35 PNGs**, comprising 12 design previews and 23 native read-only captures. Native viewports are 430×836 and 360×779, DPR 1.25. Numeric fields have equal font/line height/alignment and equal tile sizes, with zero numeric overflow. At 360 CSS px, main scrollHeight/clientHeight differs by one pixel from fractional-DPR rounding; the full viewport capture is preserved.

The native executable embeds React `dist` using a **debug build profile**. This proves actual Tauri IPC/UI integration for the read-only scenario, not release installers or all platforms. WebKitGTK snapshots use physical pixel dimensions to avoid cropping at fractional scale.

Design preview is labelled **DESIGN PREVIEW · NO TRANSACTIONS**. Values `42 / 24 / 53 / 23`, recovery estimates and success are sample data. Preview processing/success are not executed cleanup on the demo wallet. [Preview manifest](screenshots/preview/manifest.json) preserves the earlier UTC capture date, revision and browser version.

## Reproduction and remaining limits

```bash
# apps/app
npm run check
npm run format:check
npx playwright install chromium
npm run test:e2e

# Repository root; the known regression above currently fails
cargo test --workspace --locked
```

- Devnet does not use mainnet valuation/risk/routes. Missing evidence is `—`, not zero or NoRoute.
- No DAS was configured in this capture; compressed enumeration was not validated live.
- Live signing, swap, burn and close were not performed. Earlier receipts remain [historical](README.md#historical-reports-and-fixtures).
- Linux/WebKitGTK captures do not prove macOS/Windows behavior. Installer bundles remain disabled.
- The Rust status-composition regression remains unresolved; a future product-code task should reconcile expected and actual coverage semantics.

Guides: [getting started](getting-started.md), [CLI](cli-guide.md), [troubleshooting](troubleshooting.md), [desktop](../apps/app/README.md). Licensing provenance and metadata checks are recorded in the [licensing audit](licensing-audit.md).
