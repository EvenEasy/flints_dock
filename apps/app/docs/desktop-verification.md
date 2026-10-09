# Desktop verification — 2026-10-09

Implemented the stored-plan/job cleanup boundary, typed progress and confirmed
report UI, Rust category evidence, read-only DAS, shared Jupiter pacing/cache and
prefiltered raster sources. Core remains independent of React/Tauri; existing CLI
semantics and APIs are retained. Bottom-navigation component, labels, order and
geometry are unchanged. No mainnet wallet mutations were performed.

## Executed checks

| Check | Result |
| --- | --- |
| `cargo fmt --all --check` | Passed |
| `cargo check --workspace` | Passed |
| `cargo test --workspace` | 87 passed |
| `cargo clippy --workspace --all-targets --all-features` | Passed without warnings after fixing the reported unnecessary wrapper |
| `npm ci` | Passed; 239 packages, audit reported zero vulnerabilities at installation |
| `npm run check` | Passed: TypeScript, ESLint, 49 unit tests, Vite production build |
| `npm run format:check` | Passed |
| `npm run test:e2e` | Final complete invocation: 80 passed, 0 failed |
| `npm run desktop:build` | Passed; release executable with embedded `dist` |
| `npm run desktop:build -- --debug` | Passed; additional packaged-assets native IPC inspection |
| Release desktop launch | Passed; actual window screenshot below |

Initial reruns found a missing Playwright Chromium binary, an asset-load race in
assertions, a shared-title container mismatch and measurements of hidden
ornaments. These were diagnosed and corrected before the final complete run.
No failed run is reported as a pass.

Browser tests cover welcome/main/cleanup/scan/success in phone/reference modes at
360×800, 390×844, 430×932 and desktop 1440×900, plus existing narrow/short cases.
They check local fonts, shared bounds/baselines, long names/exact amounts, empty
inventory, unavailable valuation, failures/retries, keyboard focus, include/keep,
local list scrolling and separate action/navigation slots. Cleanup tests include
none, burn approval, actual mocked progress, exact large signed amounts and
partial/incomplete reports. Measurements are in [measurements](measurements/).

## Actual native runtime versus mocks

Host: Arch Linux, kernel 6.18.55, KDE/KWin Wayland 6.7.5, GTK 3.24.52,
WebKitGTK 2.54.1. Native packaged-assets inspection used `tauri://localhost`,
not a Vite URL, at 1100×1000 CSS px, DPR 1 and visual-viewport scale 1.
The release binary was separately launched and captured through Spectacle.
`bundle.active` remains false: this build produces the executable, not an installer.

The actual native React → Tauri IPC → core → local mock RPC flow was exercised:

1. Connect the public zero-seed fixture once; Rust retains the signer.
2. Analyze two empty SPL accounts of one mint, rendered as one selectable asset.
3. Uncheck it: zero planned operations, disabled CTA and no submissions.
4. Select again, approve the actual plan and observe channel-delivered confirmation
   waiting plus skipped swap/burn stages. The fixture delays RPC replies; UI does
   not invent progress.
5. Confirm two signed close transactions and display **+0.00406856 SOL** net, versus
   0.00407856 SOL gross account rent. The difference is two 5,000-lamport fees.
   The journal has two confirmed signatures and exact per-transaction deltas.
6. Inventory refresh runs after completion. Separate actual native IPC requests
   rejected read-only execution, none, empty selected and a protected mint.
   Reset fixture state confirmed zero submissions for those policy checks.

[Native IPC policy results](measurements/native-ipc-policy.json) contain no signer
material. [Reproduction fixture](../tests/native/README.md) never forwards RPC.
These are native IPC/renderer checks with mocked chain responses, **not real
Solana transaction confirmation** or evidence that mainnet Jupiter routing works.

Rust additionally tests Tauri's actual dispatcher/capability boundary using
MockRuntime, including unauthorized windows/origins. It covers duplicate execute,
expiry/session/network/selection mismatch, per-account mint expansion, exact
accounting, no API-error-to-burn fallback, NFT guards, fresh quotes, simulation,
partial execution, unresolved signatures without resend, durable restart and
DAS pagination/duplicates/partial failure. No shell execution is exposed.

## Confirmed raster causes and minimal fixes

Original PNG files remain unchanged. They have no ICC/gamma profile; alpha and
transparent edges were inspected. At identical 1100×760 CSS viewport / DPR 1,
WebKitGTK rendered a 1254px PNG downscaled to 300px with bright/grainy details,
including an ordinary image without component effects. The plain/component crop
mean difference was approximately 0.0007 RGB units, separating the defect from
component filters. Chromium rendered the same bytes without this undersampling.
Pixel-identical lossless WebP and compositing transforms did not improve it.
A prefiltered 300px copy rendered smoothly in the same native instance.

[Original / prefiltered native A/B](screenshots/desktop/native-prefilter-ab.png).
This supports a WebKitGTK scaling-path diagnosis; it does not establish a GPU
vendor-specific driver fault. No global GPU flags, pixelated/crisp rendering or
new artistic generation was used.

78 LANCZOS PNG derivatives preserve artwork/aspect/alpha. Shared `RasterArt`
chooses a source from actual paint size × DPR. Frame source density uses border
width / 26% slice; nine-slice corners, contain/cover and original art are retained.
The station scene loads locally in production. Small transparent category art,
mascot, border frames and opaque scene were inspected separately.

A second, browser-reproducible composition defect allocated a short grid row to a
much taller main hero. Explicit `minmax(0, 1fr)` rows and compact decoration rules
prevent it from extending beneath the title. Shared compaction now depends on
the full phone screen rather than the content area reduced by navigation. Text
roles and navigation are unchanged.

[Before](screenshots/desktop/native-current-main.png) ·
[After at the same native viewport](screenshots/desktop/native-fixed-main.png).

Chromium A/B was checked at DPR 1 and 2 (1100×760) and fractional DPR 1.25.
Native DPR 2 was available in an isolated `GDK_BACKEND=x11 GDK_SCALE=2` process,
960×526 CSS px due to host display constraints, visual-viewport scale 1. Sources,
alpha and slices loaded correctly; Chromium was also captured at that exact
viewport/DPR. Engine font metrics can change available decorative space; this
comparison does not claim pixel-identical layouts. Local GDK settings were test
process settings, not production renderer workarounds.

[Native DPR 2](screenshots/desktop/native-dpr2-welcome.png) ·
[Chromium at the same viewport/DPR](screenshots/desktop/chromium-dpr2-native-viewport.png).
Native fractional scaling was not verified. Playwright WebKit could not launch
on this Arch host because its Ubuntu fallback needed unavailable libicu74,
libxml2 and libflite libraries. It is not reported as a native WebKitGTK test.

## Screenshots and operational limits

- [Release welcome window](screenshots/desktop/release-welcome.png)
- [Native main](screenshots/desktop/native-packaged-main.png)
- [Native cleanup plan](screenshots/desktop/native-packaged-cleanup.png)
- [Native none selection](screenshots/desktop/native-packaged-none.png)
- [Native actual waiting stage](screenshots/desktop/native-packaged-processing.png)
- [Native confirmed fixture report](screenshots/desktop/native-packaged-result.png)
- [Phone/reference browser screens](visual-review.md)

Backend setup and exact category rules are in [README](../README.md).
Jupiter is mainnet only. No live mainnet swap/burn/close or live indexed DAS
account was validated. The optional Jupiter `audit.isSus` field is not guaranteed
by its published schema; missing data remains unknown. Routing checks are
sequential and provider-limited, so large wallets take time. Missing DAS does not
become zero NFTs. Original reference artwork layers are still missing; the
existing generated reconstructions are retained, with their exact missing-layer
list in [art inventory](../assets/art/README.md).

Plans/full reports are in memory; only nonsensitive signatures and transaction
deltas survive restart. Reconciliation blocks further execution for unresolved
signatures and does not resend, but an old UI report/session is not restored.
Linux journal durability was tested; Windows/macOS native behavior was not.
Missing transaction metadata remains incomplete accounting, never a preview
estimate substituted for a real result.
