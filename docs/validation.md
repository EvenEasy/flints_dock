# Validation

Validated on 2026-09-24.

All required checks passed:

```text
cargo fmt --check                         PASS
cargo check                              PASS
cargo test                               PASS — 17 offline tests
cargo clippy --all-targets --all-features PASS — no warnings
```

Tests exercise exact integer formatting (including u128/255-decimal boundaries), mint/program aggregation, zero balances and conservative closure assessment, Token-2022 variable lengths and embedded metadata, pubkey conversions, Metaplex discriminators and legacy/programmable editions, contradictory NFT evidence, Core variable-sized assets, stake authority offsets, pricing parsing/math, scanner states, partial-failure orchestration, total outages, RPC batching/deduplication and clean JSON/terminal output.

## Mainnet CLI run

```bash
cargo run -- --pubkey EibQ2VYpzj18qSdEBkmxWVzde7FzamTxVG9rZyY689Yj \
  --rpc-url https://api.mainnet.solana.com \
  --no-prices --format json --verbose --timeout-seconds 20
```

The executable completed successfully against the official mainnet endpoint. The first restricted-network attempt could not reach RPC; rerunning with network access succeeded. Standard JSON-RPC only was used.

Observed values (a point-in-time result, not hardcoded expectations):

| Item | Result |
| --- | ---: |
| Native SOL | 856,157,308 lamports = 0.856157308 SOL |
| Legacy token accounts | 109 |
| Token-2022 accounts | 30 |
| Total token accounts / mint-program groups | 139 / 139 |
| Empty public token balances | 20 |
| Actual token-account lamports | 285,689,513 = 0.285689513 SOL |
| Potentially reclaimable lamports | 14,414,160 = 0.014414160 SOL across 7 accounts |
| Accounts needing closure-extension review | 13 |
| Metaplex metadata records | 107 |
| Embedded Token-2022 metadata records | 30 |
| Tokens lacking recognized on-chain metadata | 2 |
| Verified classic/programmable NFTs | 0 |
| Uncompressed Core AssetV1 assets | 0 |
| Stake authority accounts | 0 |
| Durable nonce authority accounts | 0 |

SOL, both token programs, mint enrichment, Metaplex metadata/NFT checks, Core AssetV1, stake discovery and nonce discovery reported **complete**. Compressed NFTs and hashed Core assets reported **unsupported**. Generic other-account coverage reported **partial**. Prices reported **skipped** because `--no-prices` was set. No USD total was fabricated.

This validates the prior many-token-account behavior and both programs. The zero NFT/Core/stake results do not establish live positive-case coverage; positive-case decoding is exercised by offline fixtures. No authenticated Jupiter price request was possible without a provided API key; quote parsing, missing-price behavior and valuation arithmetic are tested offline.

## Example output excerpt

Human-readable representation of the observed scan, with individual token rows omitted:

```text
Wallet: EibQ2VYpzj18qSdEBkmxWVzde7FzamTxVG9rZyY689Yj

SOL
  Balance: 0.856157308 SOL (856157308 lamports)
  USD: unavailable

NFT: 0 discovered (0 programmable)
MPL CORE: 0 discovered AssetV1
STAKE: 0 authority-associated accounts
NONCE ACCOUNTS: 0 discovered

TOKEN ACCOUNTS
  Total decoded: 139
  Empty public balances: 20
  Lamports stored: 285689513 (0.285689513 SOL)
  Potentially reclaimable from empty accounts: 14414160 lamports
  Closure needs extension review: 13

SCAN STATUS
  compressed_nfts: Unsupported: not fully enumerable using standard RPC without an indexer
  legacy_tokens: Complete
  token_2022: Complete
  prices: Skipped: Disabled by --no-prices

PORTFOLIO
  Known USD value: unavailable
```

Actual output also includes every aggregate token, unknown assets, all scanner statuses, valuation scope and limitations. JSON includes every underlying token account, raw amounts, metadata and extension details.
