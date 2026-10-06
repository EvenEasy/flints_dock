# Frontend security boundaries

- The sole IPC command is the supplied read-only `analyze_wallet` wrapper. Tauri/Rust capabilities and validation enforce access; desktop detection and address shape checks are UX only.
- No seed, keypair, private key, RPC credentials, or Jupiter key is accepted, stored, logged, or bundled. No local/session storage or analytics is used. Public addresses and per-mint preferences stay in component memory.
- Backend metadata/reasons/errors render as escaped React text. No HTML injection, `eval`, arbitrary URL construction, or external metadata/image fetching exists. Error details are not serialized to the UI.
- CSP limits scripts/assets to local sources and IPC connections. Vite development adds inline React refresh scripts and localhost WebSocket only. Production preview removes those development exceptions. Existing Tauri production CSP and read-only capabilities remain unchanged.
- Vite dev/preview bind to 127.0.0.1, deny framing, disallow MIME sniffing and camera/mic/geolocation/payment, and suppress referrers. A separately deployed static web host must reproduce these headers; Vite config cannot enforce headers on another server.
- Dependencies use exact versions and `package-lock.json`. Installation was done with lifecycle scripts disabled. Run `npm audit` regularly; a clean current audit does not guarantee future vulnerability status.
- Reference preview is explicit, marked on every screen, and never calls IPC. Disabled cleanup controls are UI stubs; they do not implement backend authorization.
- All archive assets were extracted within `assets` with path/symlink/size checks. Original font license is retained. Production imports exclude large reference boards and unused SVG duplicates.

For future mutation work, keep signer material in the backend, require backend-enforced approval, revalidate mint/account/amount and ignored mints, and expose only scoped commands. Existing snapshot-based display logic must not become execution logic.
