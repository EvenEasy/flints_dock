# Flint’s Dock UI

The React UI implements the five specifications in `dev/desing_codes`:
welcome/connection, wallet analysis, station, cleanup selection, and completion.
The previous ten-screen presentation and its shared frame/title/recovery components
have been removed. Rust and `frontend-contract` are unchanged.

## Run and inspect

```sh
npm run dev
npm run dev -- --width 393
npm run dev -- --no-backdrop
npm run desktop
npm run check
PLAYWRIGHT_BROWSERS_PATH=.cache/ms-playwright npm run test:e2e
```

Desktop uses a centered portrait screen and decorative space backdrop. Width
preferences are bounded to 360–480 CSS px. Viewports up to 600px use the available
width, including 320px screens. The desktop canvas uses a modern phone's 390:844
aspect ratio and fits the available viewport height. Production UI adapts to the
screen height: decorative heroes and panel spacing shrink before text or touch
targets. On standard phone sizes, the page stays in view and only long asset lists
scroll; summary, action, and navigation remain visible. Extremely short windows or
large text zoom retain a scrolling fallback rather than clipping controls.

Browser mode cannot connect a signer or analyze a wallet. Its connection dialog
explains how to start Tauri and provides entry into the design preview.

Preview URLs:

- `/?preview=1#welcome`
- `/?preview=1#scanning`
- `/?preview=1#main`
- `/?preview=1#cleanup`
- `/?preview=1#success`

The **DESIGN PREVIEW · NO TRANSACTIONS** control opens all five destinations.
It is outside the design canvas. No preview action invokes IPC, RPC, or a signer.
For the original XML proportions, add `&layout=reference` to a preview URL. This
explicit inspection mode renders 1024×1536, or 853×1280 for the main screen,
when the viewport has room. Narrower reference canvases scale uniformly at the
same aspect ratio.
Production reflow is the normal app mode; reference mode is never applied to a
live wallet. The XML completion example is reachable only in preview.

## Code ownership

- `src/app/App.tsx`: session, allowlisted screen routing, snapshot lifetime,
  and excluded mint state. Dismissed/superseded scans cannot reopen late results.
- `src/app/AppShell.tsx`, `BottomNavigation.tsx`: portrait viewport and the XML's
  five navigation sections: station, hangar, holds, missions, profile.
- `src/shared/ui/Design.tsx`: the shared raster scene, brand, hero, nine-slice surface frames,
  and the title pedestal. Frames contain no baked text.
- `src/features/wallet`: unchanged identity/analysis transport plus station and
  truthful scan presentation.
- `src/features/assets`: actual inventory inspection, exact amount presentation,
  and reusable include/keep rows. Metadata is escaped text; remote artwork is
  never loaded. Inventory inspection paginates by 50 rows.
- `src/features/cleanup`: frontend-only selection and static result presentation.
- `src/styles.css`: reset, viewport/backdrop, dialogs and live diagnostic states.
- `src/styles/typography.css`: brand, sector, display, screen, section, body,
  caption, navigation and numeric roles, including explicit numeric size variants.
- `src/styles/components.css`: shared component internals, nine-slice skins,
  safe exact-value wrapping, and common short-content adaptations.
- `src/styles/phone.css`: production page composition and local list scrolling.
- `src/styles/reference.css`: opt-in XML canvas layout and one boundary scale.
- `src/shared/assets.ts`: centralized semantic media keys and allowlisted SVGs.
- `assets/art`: detailed generated PNG reconstructions, provenance and inventory.
- `assets/design`: retained functional XML icons and preview thumbnails.
- `assets/fonts/roboto-condensed`: locally bundled Cyrillic variable font and OFL
  license from the official Google Fonts repository. No runtime font requests.

Shared components have explicit contracts: `Brand` accepts `dock` or `station`
without changing its geometry; `MascotHero` accepts `large`, `compact` or `corner`
and owns a square art stage; `TitlePedestal` anchors swords to the outer panel;
`MechanicalPanel` accepts a tone and content inset; `BottomNavigation` accepts an
explicit active section. A composite hero includes its ring and badge only once.

Typography uses an inherited, registered length `--ui-unit`. Production defaults
to 1px; reference computes it once against the `dock` container (width / 650).
Nested hero/content containers affect art and layout but cannot reinterpret that
text scale. Layout files do not override individual role sizes or component slots.
Reference canvases have different XML widths, so geometry comparisons normalize
by this unit. Small reference canvases are inspection thumbnails; production
phone mode keeps readable role sizes.

`ExactAmount` retains every supplied digit and separates the non-wrapping unit.
Long numeric strings wrap within their own slot. Names and quantities wrap rather
than overlapping valuation; at phone widths valuation moves under identity.
DEAD TOKEN has a two-line label slot shared with the other categories. Brand and
sector use stable one-line slots sized for FLINT’S STATION. Full mint identity is
available in the row title, checkbox accessible label and inventory inspection.
No exact balance or SOL amount is truncated or converted to a JavaScript number.

## Real data and preview boundaries

Wallet sources remain a public key, raw 32-byte base64 seed, or keypair file path.
Rust reads keypair files directly. A seed is cleared after its one-time connection
request and never stored in parent state or browser storage. Connecting does not
authorize a transaction. Profile provides disconnect/rescan and actual SOL balance,
Jupiter unit price, and backend-computed USD value. Holds exposes tokens, all account
assets, Metaplex NFTs, MPL Core, cNFT capability, and scanner diagnostics.

`checked = !ignoredMints.has(mint)`: a checked asset is included; unchecking keeps
that mint. Selection persists when moving between station and cleanup. Asset names
and symbols never determine identity. Available token inventories are merged by
mint for presentation without inventing prices, liquidity, or risk classifications.

The desktop API currently returns a completed analysis snapshot, not step events.
The live scan shows pending work without stage numbers or percentage progress.
It opens the station only when an actual snapshot arrives, including partial
results. Empty, unavailable, skipped, and failed results remain distinguishable.
The five-step stage-three view belongs to preview and never advances on a timer.

## Backend and media limitations

The unchanged desktop contract does **not** expose:

- a cleanup plan with route-backed SOL estimates and eligibility assessments;
- token-to-SOL swap quotes or transaction execution;
- cleanup execution results or incremental scan/cleanup progress;
- mission data or a functioning swap/hangar flow.

Real cleanup therefore retains local selection, displays unvalued SOL slots, and
disables execution. No token is called dead merely because its Jupiter USD price
is missing. The main screen displays actual token-account counts and potential
rent assessments; SCAM/DUST/DEAD TOKEN counts remain unavailable until supplied by
the backend. The cNFT owner's historical index requirement remains visible.
No NFT cleanup or account closure is implemented by this frontend.

Only the explicit preview uses XML sample counts, SOL amounts, dead badges, and
completion. Its `≈ 0.428 SOL` estimate remains the literal reference value while
selection count changes; it is not recomputed from displayed example prices.
An empty preview selection has no estimate and disables the preview action.

The supplied raster reference was found at `dev/image(20261008-075307).png`.
Unsuitable older opaque/green/warm artwork was inspected and excluded from the
active bundle. Detailed text-free PNGs were reconstructed using the built-in
image generation tool; they are not original lossless source layers or a claim of
pixel-perfect reproduction. See [the artwork inventory](assets/art/README.md)
for resolutions, alpha checks, provenance and the exact remaining source gaps.

## Verification artifacts

`npm run check` runs TypeScript, ESLint, unit tests, and the production build.
Playwright exercises all five pages at 320, 360, 390, 393, 430, 480, 700, and 1280px,
reference canvas sizes/text, WCAG checks, focus restoration, preview navigation,
include/keep semantics, price display, partial data, and a 150-asset scroll case.

Screenshots are written to `.cache/redesign-screenshots/`. Browser integration tests
mock only the Tauri IPC boundary; they do not constitute a native desktop or live
Solana RPC test. Native wallet analysis must additionally be checked in a Tauri
window using the configured local backend.

The artwork regression suite also exercises all five pages in both phone and
reference mode at 360×800, 390×844, 430×932 and 1440×900. It compares normalized
brand bounds, emblem/copy slots and text baselines, hero aspect ratios, title/saber
anchors, navigation height/icon sizes/type, local Cyrillic font loading, a maximum
u64 lamport value, long names/quantities and empty/unvalued inventory. Measurements
and screenshots are written to `.cache/art-review/`.

Selected screenshots and verification scope are retained in [the visual review](docs/visual-review.md).
