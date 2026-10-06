# Flint’s Dock frontend

React + TypeScript + Vite presentation layer for the existing Tauri application. All frontend work lives in `apps/app`. The supplied `README.md`, `src-tauri`, and `frontend-contract` are unchanged.

## Run

Requires Node 22.22.2+, 24.15.0+, or 26+ (Node 26 tested; Node 23/25 are unsupported by the test toolchain), npm, and the existing Rust/Tauri prerequisites for desktop mode.

```sh
cd apps/app
npm ci --ignore-scripts
npm run dev
```

Open http://127.0.0.1:1420. **Explore Design Preview** opens the reference sequence. Direct reference URL: `http://127.0.0.1:1420/?preview=1#welcome`. The navigation menu opens all ten screens. All reference values are explicitly labelled; advancing confirmation/progress submits no transactions.

For actual read-only wallet analysis:

```sh
npm run desktop
```

This command supplies `tauri.frontend.conf.json` as a CLI overlay, leaving the existing backend config intact. It starts Vite automatically; stop a separately running `npm run dev` first because port 1420 is intentionally fixed. Runtime Rust compilation may populate the existing repository target directory. Backend RPC/Jupiter configuration remains owned by the existing backend; consult the original README. No API keys belong in frontend environment variables.

Use **Connect Wallet**, enter a **public address**, choose scan categories and optional USD pricing, then **Scan Wallet**. This is address entry, not a browser wallet connection or signing request. A normal browser shows an explicit desktop-required message instead of contacting RPC directly.

The real flow is scan → snapshot → token/NFT/account views → unavailable cleanup capability summary. Backend `ScanStatus` errors and useful partial results remain visible. Unavailable (`null`) and successful empty inventories stay distinct. USD prices are not swap quotes. Token keep switches are keyed by **mint**, remain in memory, and are explicitly labelled local intent; no current planning/execution contract consumes them. Changing the wallet clears them.

## Screen inventory

| Hash | Component | Purpose |
| --- | --- | --- |
| `welcome` | `features/welcome/WelcomeScreen` | Branding, public-address entry, preview entry |
| `scanning` | `features/wallet/ScanScreen` | Indeterminate live scan or fixed reference progress |
| `summary` | `features/wallet/SummaryScreen` | Backend balances, NFT/account counters, capability gaps |
| `tokens` | `features/assets/TokensScreen` | Exact balances, per-mint keep intent, 50-row live pagination |
| `dead` | `features/assets/DeadTokensScreen` | Reference burn list; unavailable classification in live mode |
| `nfts` | `features/assets/NftsScreen` | Classic/Core inventory; explicit cNFT limitation |
| `manifest` | `features/cleanup/ManifestScreen` | Reference manifest or backend account diagnostics |
| `confirm` | `features/cleanup/ConfirmScreen` | Reference transition; disabled live execution |
| `salvage` | `features/cleanup/SalvageScreen` | Reference stages; no invented live operations |
| `success` | `features/cleanup/SuccessScreen` | Reference totals; explicit unavailable live report |

## Code ownership

- `src/app`: allowlisted hash navigation, local presentation state, shell/menu.
- `src/features`: small screen components grouped by purpose, wallet snapshot hook, reference-only data.
- `src/shared/api/wallet.ts`: desktop detection and bounded plain-text errors; calls **only** the existing `frontend-contract/wallet.ts` wrapper.
- `src/shared/ui`: semantic buttons, native dialogs, notices, titles, estimate presentation.
- `src/shared/format.ts`: lossless decimal/string formatting; no cleanup, routing, or valuation rules.
- `src/styles.css`: shared responsive styling; imports supplied palette and licensed fonts.
- `assets`: all 169 original archive files, including art, icons, frames, fonts/license, and reference boards. Unused reference boards/SVG copies are not included in the production bundle.
- `tests/e2e`: Chromium screenshots, WCAG checks, keyboard and IPC boundary tests.

The XML is a design specification, not executable application logic. Its fixed progress, transaction states, and contradictory totals are kept only as labelled reference data. The web frontend adapts its portrait composition to a maximum 430px station panel on desktop and full width on mobile. Phone status-bar ornaments are omitted; touch targets, text contrast, reduced motion, native focus containment, and scrolling replace fixed coordinates.

Examples of source inconsistencies: the NFT screen draws nine tiles but labels seven; summary NFT count is 24; manifest selection is 18 while the token screen initially selects five; quote sums and received totals disagree. These do not influence real wallet values, classifications, or execution.

## Checks

```sh
npm run check
npm run format:check
npm audit
PLAYWRIGHT_BROWSERS_PATH="$PWD/.cache/ms-playwright" npx playwright install chromium
PLAYWRIGHT_BROWSERS_PATH="$PWD/.cache/ms-playwright" npm run test:e2e
```

Tests exercise the real frontend contract through a mocked **Tauri transport**, not a mocked application hook. There is no HTTP app boundary requiring MSW. Browser screenshots are written to ignored `.cache/screenshots`; browser/download/npm caches and test results stay inside this workspace.

The browser suite covers all ten reference screens at 320/390/1280px, WCAG A/AA automated rules, no horizontal overflow, no external requests, native Escape/focus restoration, and the unchanged `analyze_wallet` payload. See `VALIDATION.md` for actual results and limits. Automated accessibility checks supplement, rather than replace, human review.

## Targets and limits

Assumptions: desktop Tauri first, responsive mobile preview, no SEO/auth requirement; project maintainer owns accessibility (WCAG 2.2 AA target). Performance targets: LCP <=2500ms, INP <=200ms, CLS <=0.1, initial JavaScript <=200KB gzip. Production JS size is measured; production/user-device Web Vitals and real chain latency are not certified by local browser checks.

Only React, React DOM, and Tauri API are runtime dependencies. Tooling is pinned and lockfile-backed. Missing backend features are listed in `BACKEND_GAPS.md`. Neither simulated reference progress nor a disabled button is a backend implementation.
