# Validation entry point

The [current verification report](../../docs/current-verification.md) records completed frontend/E2E checks, native screenshots, live read-only evidence and the current Rust regression failure. The [gallery](../../docs/screenshots/README.md) includes capture provenance and reproduction commands.

```sh
# apps/app
npm run check
npm run format:check
npx playwright install chromium
npm run test:e2e

# Repository root
cargo fmt --all --check
cargo test --workspace --locked
```

Browser tests use mocked/recorded IPC; registered Tauri IPC tests, isolated loopback native fixtures and live devnet are separate evidence sources. None implies universal live mainnet coverage.

Earlier results are archived in [validation-2026-10-07.md](docs/history/validation-2026-10-07.md), [desktop verification](docs/desktop-verification.md), [devnet account audit](../../docs/devnet-cleanup-verification.md) and [unified verification](../../docs/unified-cleanup-verification.md). Historical reports retain their original dates and observations.
