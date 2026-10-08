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
width and readable production reflow, including 320px screens. Scrolling content
and the bottom navigation occupy separate layout rows; navigation cannot obscure
an action. An asset list can also scroll independently.

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
explicit inspection mode renders 1024×1536, or 853×1280 for the main screen.
Production reflow is the normal app mode; reference mode is never applied to a
live wallet. The XML completion example is reachable only in preview.

## Code ownership

- `src/app/App.tsx`: session, allowlisted screen routing, snapshot lifetime,
  and excluded mint state. Dismissed/superseded scans cannot reopen late results.
- `src/app/AppShell.tsx`, `BottomNavigation.tsx`: portrait viewport and the XML's
  five navigation sections: station, hangar, holds, missions, profile.
- `src/shared/ui/Design.tsx`: local scene layers, brand, hero, surface frames,
  and the title pedestal. Frames contain no baked text.
- `src/features/wallet`: unchanged identity/analysis transport plus station and
  truthful scan presentation.
- `src/features/assets`: actual inventory inspection, exact amount presentation,
  and reusable include/keep rows. Metadata is escaped text; remote artwork is
  never loaded. Inventory inspection paginates by 50 rows.
- `src/features/cleanup`: frontend-only selection and static result presentation.
- `src/styles.css`: shared visuals, dialogs, and readable responsive flow.
- `src/styles/reference.css`: page-specific XML reference geometry.
- `assets/design`: inert SVG resources exported from the new XML catalogs.
- `assets/fonts/roboto-condensed`: locally bundled Cyrillic variable font and OFL
  license from the official Google Fonts repository. No runtime font requests.

All CSS frames and UI labels are independent of decorative art. To replace an
art approximation, replace its local SVG resource while preserving the existing
key and layout slot; do not add a screenshot with UI text baked into it.

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

No matching new reference PNGs or original complex layered art were found in the
repository. The supplied XML SVG approximations are used for the portrait, city,
ships, station, coins, and deck. They preserve composition but do not reproduce
photorealistic textures. The original font is also unspecified; Roboto Condensed
is the XML-recommended substitute, with optical size adjustments for long labels.

## Verification artifacts

`npm run check` runs TypeScript, ESLint, unit tests, and the production build.
Playwright exercises all five pages at 320, 360, 390, 393, 430, 480, 700, and 1280px,
reference canvas sizes/text, WCAG checks, focus restoration, preview navigation,
include/keep semantics, price display, partial data, and a 150-asset scroll case.

Screenshots are written to `.cache/redesign-screenshots/`. Browser integration tests
mock only the Tauri IPC boundary; they do not constitute a native desktop or live
Solana RPC test. Native wallet analysis must additionally be checked in a Tauri
window using the configured local backend.
