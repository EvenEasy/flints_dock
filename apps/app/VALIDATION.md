# Frontend validation — 2026-10-06

Environment: Node 26.10.0, npm 12.2.0. All work and test artifacts remain inside apps/app.

- Strict TypeScript, ESLint, Prettier, and production build passed.
- Vitest / Testing Library: 26 passing tests.
- Chromium / Playwright: 34 browser cases pass, including the four affected keyboard/IPC/accessibility cases rerun after the final focus fix.
- All ten reference screens tested at 320, 390, and 1280px: no horizontal overflow, external requests, page errors, or detected automated WCAG A/AA violations. Partial live inventory also checked at 320px.
- All ten production screens rendered under strict CSP; local non-lazy images decoded; no failed asset responses. Production headers include nosniff and exclude development inline-script allowances.
- Production JavaScript: 133,230 bytes gzip, below the 200,000-byte target.
- npm audit: zero reported vulnerabilities at the time of testing.
- All 25 original README/backend/contract file hashes are unchanged. Git scope verification found no changes outside apps/app. All 169 archive assets are retained.

Tests cover the unchanged analyze_wallet payload, public-address validation, contract errors/retry, partial/unavailable/empty data, escaped metadata, exact large balances, per-mint keep intent, 50-row pagination, older Tauri hosts, late response dismissal, preview without IPC, native Escape/focus restoration, and skip-link navigation.

Screenshots were reviewed against assets/reference/source-board.png. Generated browser captures and production measurements are in ignored .cache/screenshots and .cache/production-validation.json.

## Limits

The Rust/Tauri executable was not built or launched during this frontend-only task. Integration was tested through transport doubles at the existing IPC boundary. No actual RPC or on-chain transaction was submitted. Swap/burn/close/quotes/progress/reports remain explicit stubs pending the contracts listed in BACKEND_GAPS.md.

Chromium automation does not certify WebKit/Tauri rendering, every assistive technology, or full WCAG compliance. Local checks do not certify real-user LCP/INP/CLS or network latency. Performance targets are documented in FRONTEND.md. Static hosting must reproduce the security headers; audit results are a point-in-time check.
