# flints_station

A read-only Rust CLI for SOL, SPL/Token-2022 assets, classic/programmable NFTs, Core NFTs and explicit cNFT capability reporting. No signing, burning, closing accounts, DeFi adapters, stake scans or nonce scans.

## Usage

```bash
cargo run -- -p EibQ2VYpzj18qSdEBkmxWVzde7FzamTxVG9rZyY689Yj --balance
cargo run -- -p EibQ2VYpzj18qSdEBkmxWVzde7FzamTxVG9rZyY689Yj --tokens --no-prices
cargo run -- -p EibQ2VYpzj18qSdEBkmxWVzde7FzamTxVG9rZyY689Yj --nfts --cnfts
cargo run -- -p EibQ2VYpzj18qSdEBkmxWVzde7FzamTxVG9rZyY689Yj --balance --tokens
cargo run -- -p EibQ2VYpzj18qSdEBkmxWVzde7FzamTxVG9rZyY689Yj --all --no-prices
```

No category flags means `--all`. Category flags combine in any order; `--all` selects everything even when combined with other flags. Only selected categories appear in output.

| Flag | Behavior |
| --- | --- |
| `--balance` | Native SOL only |
| `--tokens` | Fungible SPL/Token-2022 and unclassified tokens; verified NFTs excluded |
| `--nfts` | Classic, programmable and uncompressed MPL Core NFTs |
| `--cnfts` | cNFT discovery capability; currently unavailable without a historical index |
| `--all` | All four categories (default) |
| `--no-prices` | Never invoke the external pricing provider |
| `--show-mint` | Full mint / asset IDs in tables |
| `--show-price` | Unit prices in addition to estimated USD values |
| `--include-empty` | Include zero-balance aggregate token rows |
| `--details` | Raw amounts, decimals, accounts, lamports and metadata |
| `--format table\|json` | Compact terminal tables (default) or structured JSON |
| `--verbose` / `-v` | Diagnostics on stderr; does not expand tables |
| `--pubkey` / `-p` | Wallet public key |
| `--rpc-url` / `-r` | Standard Solana RPC; default `https://api.mainnet.solana.com` |
| `--timeout-seconds` | Per-request timeout, default 30; RPC retries can extend total time |

Default tables have one row per asset, hide zero balances, sort available USD values descending, then list unpriced assets. Unknown prices appear as `-`; very small positive amounts use additional precision/scientific notation. There is no global portfolio valuation or technical status dump. Failed/partial selected categories get short notices; requested empty NFT/token categories get a single “none” line.

## Pricing

Set `JUPITER_API_KEY` in the shell that launches the CLI, and omit `--no-prices`. The app does not load `.env` files automatically. [Jupiter Price V3](https://developers.jup.ag/docs/price) requires the API key and supports batches of 50 mint IDs. Missing credentials produce one concise notice. Missing quotes and provider errors never invalidate blockchain results.

SOL-only requests quote SOL. Tokens-only requests quote eligible fungible holdings, without adding SOL unless wrapped SOL is actually held. NFT/cNFT-only requests never construct or call the pricing provider. Scaled UI and interest-bearing Token-2022 balances are not priced using unverified quote units. Pricing assumes mainnet; use `--no-prices` for other clusters. NFT collection floor valuations are not implemented.

## Selective RPC work

- SOL-only: `getBalance`, no token, metadata or Core queries.
- Tokens: both token-owner queries, deduplicated mint and metadata batches to label/classify assets; no Core query.
- NFTs: the same two owner queries, but only raw-unit candidates need mint lookups, and verified mint candidates need Metaplex lookups; Core runs independently.
- Tokens + NFTs: owner, mint and metadata data are shared; no duplicate owner queries.
- cNFTs: no misleading wallet-history scan. Capability detection needs no RPC request.

The existing binary decoders, pubkey-version conversions, exact amount formatting, empty-account information and conditional rent assessments are preserved. Stake and nonce modules were removed from this focused application. On-chain account data remains separate from its presentation.

## JSON and exit codes

JSON stdout contains `wallet` and **only the requested category objects**: `sol`, `tokens`, `nfts`, `cnfts`. Each category has `status` and, when applicable, a `reason`. Token/NFT objects have `items`, complete asset identifiers and exact balances. NFT items combine classic/programmable/Core records and retain collection verification and metadata URI. Unknown assets are retained without being falsely classified as NFTs.

```json
{
  "wallet": "...",
  "sol": {
    "status": "complete",
    "lamports": 1000000001,
    "balance": "1.000000001",
    "usd_value": null,
    "price": null,
    "pricing": { "status": "skipped", "reason": "Disabled by --no-prices" }
  }
}
```

`--include-empty` controls token **asset rows**, in both table and JSON output. Every raw discovered token account stays in memory. JSON `--details` exposes the complete `tokens.discovered_accounts` inventory, including empty accounts and NFT backing accounts from the shared discovery pass, plus mints and account summaries. This preserves data for future cleanup tooling without cluttering normal output. NFT-only scans retain raw accounts but intentionally do not fetch decimals/metadata for irrelevant fungible mints.

Token raw amounts and aggregate totals are decimal strings, backed by `u64`/`u128`; display balances never use floating-point arithmetic. Other `u64` JSON fields need an integer-aware parser. Prices/USD values are approximate floating-point values, with unavailable values `null`.

Exit `0` means at least one selected asset category produced complete or partial usable results. Exit `2` means every selected category failed or was unavailable. Error statuses are still printed as JSON when requested. Therefore `--cnfts` alone exits `2`, with `items: null` and code `historical_index_required`, never a successful empty array.

## cNFT limitation

Normal requested cNFT output is exactly one short notice:

```text
cNFTs: unavailable (requires historical index)
```

Complete Bubblegum V1/V2 discovery is not implemented with the current infrastructure. A self-hosted index is technically possible, but needs complete historical events (or a verifiable snapshot plus continuous events), persistent ownership state and synchronization. A recent wallet-transaction scan cannot prove an inventory. No Helius/DAS, Solscan asset API, QuickNode DAS, Shyft, SimpleHash or other external asset index is queried. The reported Solscan count was not independently enumerated.

See [cNFT investigation and infrastructure requirements](docs/cnfts.md). Uncompressed Core AssetV1 is supported; hashed Core records are not owner-enumerated. Core plugin permissions, encrypted Token-2022 balances and arbitrary external metadata pointers remain outside the decoded scope. Off-chain metadata URLs are not fetched.

## Layout and verification

```text
src/
├── cli/mod.rs
├── models/{mod.rs,options.rs,scan.rs}
├── rpc/mod.rs
├── scanner/{mod.rs,sol.rs,tokens.rs,metadata.rs,nft.rs,core.rs,cnft.rs}
├── portfolio/{mod.rs,aggregate.rs,service.rs}
├── pricing/{mod.rs,jupiter.rs}
├── output/{mod.rs,console.rs,json.rs}
├── lib.rs
└── main.rs
tests/scanning.rs
docs/{research.md,cnfts.md,validation.md}
```

```bash
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features
```

Tests use binary fixtures, a mocked Solana RPC transport and an injected price provider. Category-combination tests assert actual RPC calls and JSON keys; they do not depend on mainnet balances. See [validation](docs/validation.md) for the current mainnet run and [research](docs/research.md) for crate/API boundaries.
