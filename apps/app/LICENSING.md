# Desktop package licensing

This package contains both Apache-2.0 source code and graphical resources with
separate terms. Its package-level metadata therefore points to this file
instead of describing the entire mixed package as Apache-2.0.

- Original application source code, scripts, configuration and documentation
  text are licensed under [Apache License 2.0](../../LICENSE). This permits use,
  modification and redistribution, including commercial use, subject to the
  license’s notice and redistribution requirements.
- Attribution for Gleb and Nazar is recorded in [NOTICE](../../NOTICE).
- [BRANDING.md](../../BRANDING.md) covers the Flint’s Dock name, logos and mascot.
- The exact artwork inventory and its separate terms are in
  [ASSETS_LICENSE.md](../../ASSETS_LICENSE.md). Those graphical files are
  excluded from the code license; this is not a blanket restriction on fonts,
  third-party icons or dependencies.
- Third-party resources retain their own licenses and copyright notices.
  Unverified resource provenance is listed in the
  [licensing audit](../../docs/licensing-audit.md); no rights are invented for it.

The Rust core and CLI packages contain application source code and inherit the
workspace’s Apache-2.0 identifier. The Tauri package uses this mixed-package
statement as its `license-file`; the private npm package uses
`SEE LICENSE IN LICENSING.md`. These metadata declarations do not relicense
third-party dependencies or the artwork.
