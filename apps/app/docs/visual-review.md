# Visual verification

All five screens were opened and visually inspected in Chromium, in phone and
reference modes. These screenshots were captured from rendered DOM, not used as
application backgrounds. Phone images are 390×844; reference images use the XML's
1024×1536 canvas (853×1280 for main). All sample amounts and stage progress in
these images are explicit design preview data, never a real cleanup result.

| Screen  | Phone                                        | Reference canvas                                 |
| ------- | -------------------------------------------- | ------------------------------------------------ |
| Welcome | [Screenshot](screenshots/phone-welcome.png)  | [Screenshot](screenshots/reference-welcome.png)  |
| Main    | [Screenshot](screenshots/phone-main.png)     | [Screenshot](screenshots/reference-main.png)     |
| Cleanup | [Screenshot](screenshots/phone-cleanup.png)  | [Screenshot](screenshots/reference-cleanup.png)  |
| Scan    | [Screenshot](screenshots/phone-scanning.png) | [Screenshot](screenshots/reference-scanning.png) |
| Success | [Screenshot](screenshots/phone-success.png)  | [Screenshot](screenshots/reference-success.png)  |

[Centered desktop, 1440×900](screenshots/desktop-main.png).

Additional states use mocked Tauri IPC fixtures, not a connected real wallet:
[long names/amounts](screenshots/live-long-values.png),
[empty inventory/unavailable estimate](screenshots/live-empty.png),
and [analysis failure](screenshots/live-analysis-error.png).

## Checks performed

- `npm run check`: TypeScript, ESLint, 44 unit tests, and production build passed.
- `npm run format:check`: passed.
- All 75 Playwright cases passed in bounded groups (40 layout/accessibility and
  35 component/behavior cases). After the final station-gutter/API-copy changes,
  the 35 behavior cases and 8 affected main layout/accessibility cases passed again.
  Final proportional-reference/nav-bound checks also passed in the 35-case group.
- The single long suite invocation was terminated with SIGTERM before completion;
  the grouped runs cover every case and completed successfully.
- Phone/reference at 360×800, 390×844, 430×932 and desktop 1440×900; existing
  320–1280px width checks and short 360×640 / desktop 1280×720 checks also passed.
- No whole-page scrolling at tested phone sizes. Long cleanup lists scroll locally;
  action and navigation remain separate. Reference remains an opt-in inspection
  canvas. Narrow canvases retain their XML aspect ratio; larger canvases scroll
  when their height exceeds the viewport. Scaled nav buttons remain inside the frame.
- WCAG axe checks, keyboard focus, dialog focus restoration, native checkbox
  include/keep behavior, navigation, unavailable estimates and error/retry states.
- Exact maximum-u64 lamport amount, full long names/symbols and mint identity;
  no numeric coercion or precision loss, no decorative station/value overlap.
- Font loading was deliberately held pending, then released. Brand/emblem/sector
  and navigation slots remained stable before/after local Cyrillic font load.
- Assets remain local; no remote wallet-metadata images or external runtime font
  requests. No Rust, frontend contract or transaction API was changed.

Normalized DOM measurements for all requested viewport/mode pairs are saved in
[measurements](measurements/). They compare the brand's bounds/insets/baselines,
centered emblem/text slots, square hero stage/art, fixed sword anchor relative to
the title, and navigation bounds/icon sizes/type. Normalizing by `--ui-unit`
accounts for the intentionally different XML reference canvas widths.

Reproduce the grouped runs:

```sh
PLAYWRIGHT_BROWSERS_PATH=.cache/ms-playwright npm run test:e2e -- --grep 'Preview_'
PLAYWRIGHT_BROWSERS_PATH=.cache/ms-playwright npm run test:e2e -- --grep-invert 'Preview_'
```

Native Tauri execution, a real signer and live Solana/Jupiter responses were not
verified in this browser environment. IPC tests preserve the existing contract;
they are not evidence of a successful on-chain cleanup. Real cleanup stays
disabled until the desktop API exposes plans, quotes and execution results.

See [art inventory and missing originals](../assets/art/README.md). Generated
reconstructions are more detailed but do not claim exact original production art.
