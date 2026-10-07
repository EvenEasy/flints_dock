# App validation — 2026-10-07

Environment: Node 26.10.0, npm 12.2.0. Source changes and generated check artifacts are confined to `apps/app`. The original README, underlying core, dependency manifests for Rust, and workspace Cargo.lock remain unchanged. Backend/contract edits are limited to the authorized local identity extension.

- Strict TypeScript, ESLint, Prettier, and Vite production build passed.
- Vitest / Testing Library: **41 passing tests**.
- Chromium / Playwright: **76 passing cases**.
- All ten reference screens tested at **360, 390, 393, 430, 480, 700, and 1280 CSS px**. Checks verify bounded phone width, 390:844 proportions, horizontal/vertical centering, no footer/page counter, no horizontal overflow at supported widths, no external requests, and no page errors.
- Automated WCAG A/AA checks passed for the reference matrix, partial live inventory, and seed/keypair-path/public-key dialog modes, including validation errors. Keyboard checks cover Escape focus restoration and skip-link routing.
- Real dev launcher checks verified `--width 430 --backdrop` reaches Vite's frontend environment. Widths 359, 481, and nonnumeric input were rejected before server startup. Owned test servers were stopped.
- All ten built production screens loaded under the production CSP; local non-lazy images decoded, geometry matched the selected 393px width, and there were no console/page errors. Headers include nosniff and exclude development inline-script allowances.
- Production JavaScript: **134,583 bytes gzip** (Node zlib measurement), below the 200,000-byte target. Measurements are in ignored `.cache/production-validation.json`; screenshots are in `.cache/screenshots`.
- `cargo fmt -p dock-flints-app`, `cargo check -p dock-flints-app --offline --locked`, and `cargo test -p dock-flints-app --offline --locked` passed with an app-local target directory. **10 Rust integration tests passed**.
- Rust tests exercise existing core seed/keypair loaders, public-only connection responses, credential-safe errors, strict request fields, local-main-window capabilities, denied remote/other-window connect/disconnect, signer/read-only capability distinction, and the existing scanner through a local RPC fixture.
- No runtime or development dependencies were added. All 169 supplied media files are retained. No fresh network dependency audit was performed in this pass; previous audit results are point-in-time results only.

Frontend tests cover one-time seed transfer and cleared fields on success/failure, keypair path transport, public-only analysis requests, explicit disconnection, transport errors/retry, stable post-scan hash navigation, partial/unavailable/empty results, escaped metadata, exact large balances, per-mint keep intent, pagination, older Tauri bridges, late-result dismissal, and reference preview without analysis/connection IPC.

## Limits

The Rust application compiled and its IPC commands ran with Tauri MockRuntime. A graphical WebKit/Tauri window and live mainnet/devnet scan were not exercised. Browser integration used transport doubles; scanner tests used a local RPC fixture. No real wallet secrets or on-chain transactions were submitted.

Signer loading is implemented; swap/burn/close/quotes/cleanup planning/progress/reports remain unavailable until the desktop contracts in `BACKEND_GAPS.md` are implemented. Core capabilities must still be exposed through explicitly authorized commands before the UI can execute them.

Chromium automation does not certify all WebKit versions, assistive technologies, or full WCAG compliance. Local checks do not certify real-user LCP/INP/CLS or chain latency. Viewports smaller than the 360px minimum or shorter than the fixed phone proportion may scroll the outer canvas, as documented in `FRONTEND.md`.
