# Licensing audit

This audit records the repository evidence and the licensing changes requested
by the maintainers. It does not infer ownership from a GitHub account or assign
third-party work to the project.

## Code and attribution

No repository-level code LICENSE or NOTICE existed before this change. Existing
font notices were present and remain unchanged. The workspace and npm package
had no package license declaration.

Git history records commits by `EvenEasy`; the remote is
`https://github.com/EvenEasy/flints_dock`. Commit authorship and repository
administration do not establish sole ownership of all contributions.

The maintainer explicitly provided these names for 2026 attribution:

- **Gleb (TG: @Scullgrey)** — Founder.
- **Nazar (TG: @notGucuL)** — Engineer.

[NOTICE](../NOTICE) uses those names for their respective project contributions.
No legal entity was invented. Third-party notices remain with their materials.

[LICENSE](../LICENSE) is the unmodified official
[Apache License 2.0 text](https://www.apache.org/licenses/LICENSE-2.0.txt), including
its original appendix. The code license allows commercial use, modification and
redistribution under its conditions. The separate [branding policy](../BRANDING.md)
does not prohibit competing forks or add restrictions to Apache-2.0.

## Artwork evidence

[Art README](../apps/app/assets/art/README.md) and
[generation.json](../apps/app/assets/art/generation.json) describe ten active
PNG sources as image-generation reconstructions of a supplied reference.
`art/sizes/` contains 78 resized derivatives. They are not documented as extracted
original production layers. Original lossless mascot/scene/panel layers are
explicitly recorded as missing.

[Legacy manifest](../apps/app/assets/manifest.json) links the older emblem,
illustrations, thumbnails and frames to cropped reference-board regions.
[Asset README](../apps/app/assets/README.txt) explains that extraction.
Inspection confirms the raptor/pirate emblem, project wordmark in the legacy
illustration and the active generated mascot. Some images also depict Solana’s
mark, which is not assigned to Flint’s Dock.

[ASSETS_LICENSE.md](../ASSETS_LICENSE.md) lists concrete paths for the identified
project branding/art and resized derivatives. These are excluded from the code
license; their respective holders reserve rights to the extent they hold them.
The statement preserves third-party permissions and does not claim that every
reference component belongs to the project. Screenshots reproduce mixed
materials and do not grant extraction/reuse rights for the embedded art.

## Preserved third-party terms

| Material                        | Evidence retained                                                                                                                    |
| ------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| Roboto Condensed                | Copyright 2011 The Roboto Project Authors; SIL OFL 1.1 in `apps/app/assets/fonts/roboto-condensed/OFL.txt`                           |
| Nimbus Sans Narrow Regular/Bold | URW notices and AGPL-3 with Font exception in `apps/app/assets/fonts/LICENSE.txt`; that file also retains upstream packaging notices |
| Rust/npm libraries              | Individual upstream license declarations/notices; no dependency is relicensed by this change                                         |
| Token/Solana depictions         | Third-party identity remains separate; this repository supplies no verified upstream reuse authorization                             |

Neither font notice was edited or replaced. Generic functional icons, fonts and
third-party token logos are not automatically restricted by the project’s
reserved-art list.

## Unconfirmed ownership and provenance

The supplied holder names establish requested project attribution, but the
repository does not establish:

- The creator and permission chain for the original supplied reference boards
  and `dev/image(20261008-075307).png`.
- Rights in every cropped legacy illustration, NFT sample, token image, generic
  functional icon, frame or reconstructed SVG.
- Upstream permissions for embedded third-party marks and sample token logos.
- A human author or rights transfer for every generated reconstruction, or a
  legal conclusion about its copyrightability.
- The origin/license of the plain placeholder `apps/app/src-tauri/icons/icon.png`.
- Any registered trademark status for Flint’s Dock. No registration is asserted.

These gaps are recorded rather than replaced with invented copyright claims.
Original font attribution remains confirmed by the bundled notices.

## Package metadata

| Package                                            | Declaration                                            | Scope                                                             |
| -------------------------------------------------- | ------------------------------------------------------ | ----------------------------------------------------------------- |
| Workspace / `dock-flints-core` / `dock_flints` CLI | `Apache-2.0` inherited by the two source-code packages | Project code; dependency licenses remain separate                 |
| `dock-flints-app` Tauri package                    | `license-file = "../LICENSING.md"`                     | Mixed desktop package statement                                   |
| Private `flints-dock-app` npm package              | `SEE LICENSE IN LICENSING.md`                          | Same mixed-package statement; root lockfile metadata synchronized |

[Desktop LICENSING.md](../apps/app/LICENSING.md) links the code license, notice,
branding rules, asset inventory and this audit. The mixed desktop package is not
labelled wholly Apache-2.0. No author metadata was guessed or filled from Git.

Checks use metadata parsing, local-link validation and comparison with the
official license download. No publication or push is part of this change.

## Completed verification

- Official Apache text matches `LICENSE` byte-for-byte: 11,358 bytes.
  SHA-256: `cfc7749b96f63bd31c3c42b5c471bf756814053e847c10f3eb003417bc523d30`.
- All 100 reserved-art paths exist, are unique and exclude fonts, generic icon
  directories and third-party token-logo directories.
- `cargo metadata --no-deps --format-version 1 --offline --locked` parsed the
  workspace successfully; core/CLI report Apache-2.0 and desktop reports its
  mixed-package license file.
- npm manifest and root lockfile license declarations match; the referenced
  `LICENSING.md` exists.
- All local Markdown file links resolve. Documentation has no remaining
  Ukrainian/Cyrillic prose. ESLint, Prettier and diff whitespace checks pass.
- Original bundled font notices are unchanged. Runtime application source,
  functionality and design were not edited. Capture tools were adjusted only to
  observe immutable Tauri IPC, find the actual X11 title and avoid snapshot crops.
- No commit, publication or push was performed for these changes.
