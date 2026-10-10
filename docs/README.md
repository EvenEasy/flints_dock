# Flint’s Dock documentation

Start with [getting started](getting-started.md). The current product consists of a React/Tauri desktop application and a Rust CLI sharing the same core. Historical reports document specific earlier checks, not the present wallet balance or universal integration coverage.

## Launch and usage

| Document                                        | Contents                                                               |
| ----------------------------------------------- | ---------------------------------------------------------------------- |
| [Getting started](getting-started.md)           | Prerequisites, desktop/browser/CLI, first devnet scan                  |
| [CLI guide](cli-guide.md)                       | Commands, scan selection, JSON, cleanup preview, quote/swap            |
| [Desktop README](../apps/app/README.md)         | Launch/build, backend environment, connection, plans and reports       |
| [Troubleshooting](troubleshooting.md)           | Runtime modes, build dependencies, RPC, coverage and common errors     |
| [Screenshot gallery](screenshots/README.md)     | Screens, scenarios, viewport and data provenance                       |
| [Current verification](current-verification.md) | Checks and read-only devnet evidence for this documentation revision   |
| [Cleanup](cleanup.md)                           | Selection, swap/burn/close, NFT adapters, accounting and journal       |
| [Swaps](swaps.md)                               | Jupiter quote/swap, exact raw units, confirmation and execution limits |
| [Licensing audit](licensing-audit.md)           | Code license, branding inventory, metadata and unresolved provenance   |

## Development

| Document                                                     | Contents                                                      |
| ------------------------------------------------------------ | ------------------------------------------------------------- |
| [Architecture](architecture.md)                              | Core/app/infra, CLI/Tauri boundaries and change locations     |
| [Frontend](../apps/app/FRONTEND.md)                          | React structure, state/navigation, API boundary and UI checks |
| [Frontend contract](../apps/app/frontend-contract/README.md) | Typed IPC commands and DTOs                                   |
| [Integration limits](../apps/app/BACKEND_GAPS.md)            | Providers, standards and unsupported scenarios                |
| [Design](../apps/app/UI_DESIGN.md)                           | Visual reference and UI assets                                |
| [Visual review](../apps/app/docs/visual-review.md)           | Earlier viewport/geometry checks                              |
| [cNFT research](cnfts.md)                                    | RPC-only ownership limits and current DAS requirements        |
| [Research](research.md)                                      | Historical technical decisions and SDK/API boundaries         |

## Historical reports and fixtures

- [Validation](validation.md): initial scanner/workspace migration and dated mainnet observations.
- [Devnet cleanup verification](devnet-cleanup-verification.md): earlier devnet/account checks.
- [Unified cleanup verification](unified-cleanup-verification.md): canonical inventory, NFT cleanup and earlier native validation.
- [Native desktop verification](../apps/app/docs/desktop-verification.md): earlier packaged desktop results.
- [Fixtures](fixtures/): recorded inputs/outputs for particular regressions. Synthetic observations are test data, not live market evidence.

Obtain current wallet values through a new scan. Saved screenshots and JSON describe their capture time only.

## Licensing

Code and documentation text use [Apache-2.0](../LICENSE); attribution is in [NOTICE](../NOTICE). The [branding policy](../BRANDING.md) and [graphical asset terms](../ASSETS_LICENSE.md) apply separately. Third-party licenses and notices are preserved.
