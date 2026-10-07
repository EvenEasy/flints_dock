# Flint’s Dock frontend

React + TypeScript + Vite presentation layer for the Tauri application. All changes live in `apps/app`. The original `README.md` is unchanged. The authorized backend/contract extension adds local wallet identity commands, reusing the existing core loaders and scanner.

## Run

Requires Node 22.22.2+, 24.15.0+, or 26+ and existing Rust/Tauri desktop prerequisites.

```sh
cd apps/app
npm ci --ignore-scripts
npm run desktop
```

The desktop command starts Vite automatically. Stop a separately running `npm run dev` first: desktop development intentionally uses port 1420. RPC/Jupiter settings remain backend configuration; consult the original README. Never put credentials or provider keys in frontend environment variables.

For browser design preview only:

```sh
npm run dev
```

Open http://127.0.0.1:1420 and select **Explore Design Preview**, or use `/?preview=1#welcome`. A normal browser cannot connect a local wallet or call the desktop scanner and does not present seed input fields.

## Wallet connection

Select **Connect Wallet**, then exactly one source:

- **Seed (base64)**: standard base64 encoding of **32 raw Ed25519 seed bytes**, matching the existing core loader. This is not a mnemonic phrase, base64-encoded phrase, or 64-byte keypair. The password field clears after submission, including failed attempts. Rust receives the seed once and returns only the derived public address and signing capability.
- **File (keypair)**: enter the **absolute local path** to a standard Solana JSON keypair, such as `/home/you/.config/solana/id.json`. Rust reads the file directly; no file contents enter React. The current UI uses a path field, not a native file chooser.
- **Public key (read only)**: enter a public address to inspect a wallet without loading a signer.

Choose scan categories and optional USD prices, then **Scan Wallet**. Analysis requests contain only the public address, selection, and price preference. The summary distinguishes **LOCAL SIGNER CONNECTED** from **PUBLIC ADDRESS · READ ONLY**. A signer stays in Rust memory until replaced, disconnected, or the app exits; connection never authorizes a transaction. Use **Disconnect Wallet** in the navigation menu to release the backend session and clear its displayed inventory. Switching to reference preview also disconnects an active session.

Completed scans remain on the summary screen. Scan failures stay on **Scan Interrupted**, with the error, **Retry Scan**, and **Change Wallet** actions. Retrying uses the public address, not saved credentials. Partial results and per-category diagnostics remain visible; unavailable and successfully empty inventories remain distinct. Dismissing a scan discards its eventual UI result but does not cancel backend RPC work.

## Jupiter USD prices

Prices are **enabled by default** in the wallet dialog. The existing Rust Jupiter Price V3 adapter enriches the normal `analyze_wallet` snapshot; React makes no direct Jupiter requests. Uncheck **Include Jupiter USD prices** to skip pricing while retaining the same inventory.

The summary shows **1 SOL = USD price** separately from the approximate USD value of the entire native balance. Token views show **PRICE / TOKEN** separately from the USD holding value. Small unit prices use additional decimal places or scientific notation (for example `$1.23E-10`) to stay readable without falsely displaying zero. The token-price tooltip includes the provider value and source. Amount strings remain exact; all valuation is owned by the existing Rust core.

Without `JUPITER_API_KEY`, Rust attempts the official **keyless** endpoint. A configured key uses the existing sensitive `x-api-key` header and remains outside frontend requests/bundles. To supply a key without entering it into shell history:

```sh
read -rsp "Jupiter API key: " JUPITER_API_KEY
export JUPITER_API_KEY
npm run desktop
```

The existing adapter deduplicates mint addresses and requests batches of up to 50. Keyless access has a lower provider rate limit; an API key can increase available limits. See the official [Jupiter rate limits](https://developers.jup.ag/docs/portal/rate-limits). Pricing failures/partial coverage are labelled **USD PRICES** on the summary and token screens and do not remove assets or replace missing prices with zero.

Prices are a snapshot from the scan, not a streaming ticker or swap quote. Restart the desktop app after changing backend configuration and run a new scan for updated prices. Jupiter market prices describe mainnet assets; devnet test tokens generally have no quote, and SOL USD is a mainnet reference rather than a valuation of devnet funds. Missing quotes display **PRICE UNAVAILABLE** and do not classify tokens as unswappable/dead. Existing NFT and unsupported Token-2022 pricing restrictions remain in the core.

## Display options

The default is a centered portrait app screen without stars, external captions, or page numbering.

```sh
npm run desktop -- --width 393
npm run desktop -- --width 430 --backdrop
npm run dev -- --width 480 --backdrop
```

`--width` accepts **360–480 CSS px**, default **390**. `--backdrop` enables the star background; `--no-backdrop` disables it. These preferences reach the desktop's Vite subprocess. The desktop window uses the selected phone size, or a larger canvas for backdrop mode.

URL overrides also work: `/?width=393&backdrop=1`, or `/?preview=1&width=430#summary`. URL widths are clamped to the supported range. Vite build preferences are baked into production assets; override a production preview with URL parameters when needed.

The frame retains **390:844 portrait proportions**. Main content scrolls inside it and uses a combined 26px inset on each side. At tablet/fold-open viewport widths (700px+), the outer canvas grows while the phone remains bounded. Physical inches are device-dependent and are not CSS dimensions. When the browser is narrower than 360px or shorter than the selected phone, the outer page can scroll rather than shrink below the minimum or crop content.

## Code ownership

- `src/app`: navigation, public session/presentation state, phone shell and display settings.
- `src/features`: wallet connection, scan, inventory screens, cleanup placeholders, and labelled reference data.
- `frontend-contract`: typed `connect_wallet`, `disconnect_wallet`, and unchanged `analyze_wallet` transport wrappers.
- `src/shared/api/wallet.ts`: desktop-only transport entry points and bounded escaped error messages.
- `src-tauri/src/commands/identity.rs`: local connection/disconnection; public-only response and backend signer retention.
- `src-tauri/src/dto/identity.rs`: credential-safe DTO validation and existing core wallet loaders.
- `src/shared/format.ts`: lossless display formatting, without valuation or execution rules.
- `src/styles.css`: bounded phone layout, container-responsive content, and original assets/fonts.
- `scripts/launch.ts`: validated display arguments and child-process launching without shell evaluation.
- `assets`: all 169 supplied archive files, including original artwork and font license.

All ten design screens remain available in reference preview. The XML's fixed progress and inconsistent example totals are presentation data only. No preview transaction is submitted. Live swap/burn/close controls remain explicitly unavailable until their backend contracts exist. USD prices are not swap quotes. Per-mint keep preferences stay local; future planning/execution must enforce them in Rust.

## Checks

```sh
npm run check
npm run format:check
PLAYWRIGHT_BROWSERS_PATH="$PWD/.cache/ms-playwright" npm run test:e2e
CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_TARGET_DIR="$PWD/.cache/rust-check" cargo check -p dock-flints-app --offline --locked -j 2
CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_TARGET_DIR="$PWD/.cache/rust-check" cargo test -p dock-flints-app --offline --locked -j 2
```

Browser tests need Playwright Chromium installed. App-local ignored `.cache` holds screenshots, browsers, and validation/build artifacts. See `VALIDATION.md` for actual results and limits, `SECURITY.md` for boundaries, and `BACKEND_GAPS.md` for unimplemented backend capabilities.

Only React, React DOM, and Tauri API are runtime dependencies; no dependencies were added for these fixes. Accessibility target: WCAG 2.2 AA. Performance targets: LCP ≤2500ms, INP ≤200ms, CLS ≤0.1, initial JavaScript ≤200KB gzip. Local automation does not certify user-device Web Vitals, chain latency, or every assistive technology.
