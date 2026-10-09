# Visual verification

This report describes the current desktop-cleanup implementation. For native
runtime diagnosis, executed checks and limits see
[desktop verification](desktop-verification.md).

All five design screens were rendered and visually inspected in Chromium in phone
and reference modes. Sample values/progress in these screens are explicitly
**design preview**, never live cleanup results. Screenshots are evidence, not
application backgrounds; texts remain DOM elements.

| Screen | Phone 390×844 | Reference mode at 390×844 viewport |
| --- | --- | --- |
| Welcome | [Screenshot](screenshots/phone-welcome.png) | [Screenshot](screenshots/reference-welcome.png) |
| Main | [Screenshot](screenshots/phone-main.png) | [Screenshot](screenshots/reference-main.png) |
| Cleanup | [Screenshot](screenshots/phone-cleanup.png) | [Screenshot](screenshots/reference-cleanup.png) |
| Scan | [Screenshot](screenshots/phone-scanning.png) | [Screenshot](screenshots/reference-scanning.png) |
| Success | [Screenshot](screenshots/phone-success.png) | [Screenshot](screenshots/reference-success.png) |

The final complete Playwright run passed 80 cases. Phone/reference comparisons
cover 360×800, 390×844, 430×932 and desktop 1440×900. Existing 320–1280px widths and
short 360×640 / desktop 1280×720 are also covered. DOM geometry checks compare
brand slots/baselines, equal hero variants, title/sword anchors and bottom
navigation. Hidden compact ornaments have no painted bounds. The short 1100×760
case explicitly verifies that the hero does not extend under the title.

Asset assertions wait for ResizeObserver-selected derivatives to load after font
settling. Local Cyrillic font loading is held/released in a separate test to check
slot stability. Exact sums and full asset names remain available through titles
and normal wrapped text. Lists scroll independently; primary actions/navigation
do not cover content. Category counts distinguish complete, partial and unknown;
missing valuation never labels a token dead.

[Measurements](measurements/) store the normalized DOM comparisons. Native
screenshots and actual mock-chain IPC results are linked from the desktop report.
Browser IPC mocks are not evidence of on-chain execution. Production UI retains
the phone aspect ratio; the XML reference canvas is only an opt-in preview.

Reproduce:

```sh
npm run test:e2e
```

If Chromium is not installed for the pinned Playwright version, run
`npx playwright install chromium` first. Playwright WebKit and native WebKitGTK
are separate runtimes; only checks actually executed are reported.

[Art inventory and missing originals](../assets/art/README.md) records original
sources, existing generated reconstructions and technical PNG derivatives.
