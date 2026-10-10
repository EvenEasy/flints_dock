# Frontend development

React renders a phone-sized interface hosted by Tauri. Rust owns wallet identity/signers, inventory, category classification, saved cleanup plans and execution. Start with [desktop launch instructions](README.md), [IPC contract](frontend-contract/README.md) and [current verification](../../docs/current-verification.md).

## Structure

```text
src/app/                  App, navigation/session guards, display preferences
src/features/wallet/      ConnectWalletDialog, scan progress, useWalletAnalysis, MainScreen
src/features/assets/      Inventory/CategoryDialog, asset and category presenters
src/features/cleanup/     Selection, stored-plan preparation, job execution/recovery
src/features/preview/     Explicitly labelled design-preview fixtures
src/shared/api/           Typed invoke boundary; no independent blockchain logic
src/shared/               Assets, formatting and reusable UI
frontend-contract/        Canonical TypeScript requests and DTOs
src-tauri/                Registered IPC, sessions, plans/jobs, journal and DTO conversion
```

Only the shared API boundary invokes Tauri. UI components consume typed data; they do not call RPC/Jupiter directly or build transactions. `src/shared/assets.ts` allowlists local media. Wallet metadata cannot choose arbitrary webview image URLs.

## Analysis lifecycle

`ConnectWalletDialog` collects identity and scan selection. Public key creates a read-only session; file/seed creates a Rust-held signer. Connection alone does not approve execution. The normal scan enables SOL, token/raw-account inventory, classic/Core NFT discovery, compressed capability and optional pricing.

`useWalletAnalysis` requests `analyze_wallet` and receives backend progress through a Tauri Channel. Each request has a generation. A new scan/reset clears prior data/progress; late generations and mismatched returned owners are rejected. App also guards dismissed responses and live hash navigation. Dismiss does not cancel backend RPC.

The accepted snapshot remains available across main/inventory/cleanup navigation. Wallet change, disconnect and rescan reset the relevant state. Live success/processing cannot be created by changing a hash without an actual result/job.

## Category presentation

`normalizeWalletCategories` runs once for the accepted analysis. It deduplicates IDs, merges backing accounts and aligns count with items. If the authoritative NFT category is absent, compatibility fallback normalizes classic/Core/compressed discovery in this same location, including ownership and coverage. MainScreen and CategoryDialog use that same result.

`presentCategory` separates numeric text, status, reason and coverage. All four counters use the same numeric typography and contain only a number or `—`. Complete-empty is `0`; known partial items show the number found; empty partial/missing/unsupported/failed/skipped without reliable items show `—`. Coverage explanations belong in tooltip/accessible description and details.

Pricing/risk errors do not hide known NFTs. Routing errors do not invalidate reliable dust valuation. Unknown observations do not become zero, suspicious evidence or NoRoute. Demo values are confined to explicit preview mode.

## Cleanup and recovery

Selection uses canonical asset IDs. Checked means include; unchecked means keep. Rust creates an immutable session/network/revision-bound plan; the UI displays actions/reasons/estimates before approval. Public-key sessions can prepare but cannot execute.

The final **RECOVER SOL** approves the shown plan. Rust executes sequentially with fresh checks, simulation and confirmation. `useCleanup` consumes progress/jobs through the typed boundary and recovers uncertain responses with read-only job lookup. Active jobs block wallet change/disconnect. Reports preserve partial failures and incomplete accounting; inventory refreshes after execution.

The durable signature journal belongs to Rust desktop. Full UI session/plans/reports remain in memory; restart does not restore them.

## Preview and responsive layout

`?preview=1` explicitly enables sample screens with **DESIGN PREVIEW · NO TRANSACTIONS**. Welcome/scanning/main/cleanup/success are accessible through the preview menu; `#processing` is a separate presentation. Scanning is static; preview cleanup goes directly to sample success.

Phone width defaults to 430 CSS px, allowed range 360–480. Star backdrop defaults to enabled. Asset lists own their scrolling. Display options are documented in [README](README.md#display-options); capture sizes/provenance are in the [gallery](../../docs/screenshots/README.md).

## Development checks

```sh
npm ci
npm run check
npm run format:check
npx playwright install chromium
npm run test:e2e
```

Browser tests use mocked/recorded IPC. Native fixture and live devnet captures are separate evidence, documented in the [current report](../../docs/current-verification.md). Product limits and configuration failures are tracked in [BACKEND_GAPS.md](BACKEND_GAPS.md).
