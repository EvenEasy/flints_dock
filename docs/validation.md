# Validation

Validated on 2026-09-30 after the shared classification refactor.

```text
cargo fmt                               PASS
cargo check                             PASS
cargo test                              PASS — 28 offline integration tests
cargo clippy --all-targets --all-features PASS — no warnings
```

Regression coverage includes UI balance exactly 1 for fungible 6/9-decimal mints; all four NFT TokenStandard variants; legacy MasterEdition and Edition proof; missing or contradictory metadata; empty NFT accounts; multiple fungible accounts aggregated with backing addresses retained; Unknown accounts kept separately; raw account visibility; NFT exclusion from fungible output/pricing; and combined classic/programmable/Core presentation. All 31 nonempty category combinations check actual RPC calls and JSON category selection. A mixed inventory verifies no category loss and a single deduplicated pricing request. Existing decoder, precision, batching, partial-failure and cNFT capability tests remain covered.

## Live mainnet comparison

Wallet: `EibQ2VYpzj18qSdEBkmxWVzde7FzamTxVG9rZyY689Yj`.

Ran each category independently against `https://api.mainnet.solana.com`:

```bash
cargo run -- -p EibQ2VYpzj18qSdEBkmxWVzde7FzamTxVG9rZyY689Yj --all-tokens --no-prices --format json --details --include-empty --timeout-seconds 20
cargo run -- -p EibQ2VYpzj18qSdEBkmxWVzde7FzamTxVG9rZyY689Yj --tokens --no-prices --format json --details --include-empty --timeout-seconds 20
cargo run -- -p EibQ2VYpzj18qSdEBkmxWVzde7FzamTxVG9rZyY689Yj --nfts --no-prices --format json --details --include-empty --timeout-seconds 20
```

All three scans exited 0 with complete category status.

| Observation | Result |
| --- | ---: |
| Legacy / Token-2022 accounts | 110 / 30 |
| Total raw accounts | 140 |
| Empty / nonempty token accounts | 20 / 120 |
| All-token rows with `--include-empty` | 140 |
| Fungible rows with `--include-empty` | 140 |
| Unknown classifications | 0 |
| Verified classic / programmable NFTs | 0 |
| Core AssetV1 assets | 0 |

The broad view preserves every discovered token account: compared each row's backing account address against the complete raw inventory, with exactly one row per account. Fungible and raw views match by mint, raw amount and classification for this wallet, which currently has one account per mint. All mints have positive decimals (5, 6, 8 or 9). Normal output hides the 20 empty accounts. No verified NFT appears in the fungible category.

These are point-in-time results. The earlier documented scan found 139 accounts; the current scan finds 140. Zero live classic/Core results do not provide positive NFT coverage; binary fixtures exercise NFT standards and legacy editions. NFTs shown by another wallet UI may include compressed assets; this run does not enumerate those or validate that UI's inventory.

cNFTs remain unavailable without indexed Bubblegum history. The CLI reports `items: null` / `historical_index_required`, and a cNFT-only invocation exits 2 without making unrelated RPC calls. It never reports a successful empty cNFT list.

Live pricing was intentionally disabled. Offline provider tests verify deduplication, selected-category pricing, missing/error handling and exclusion of NFT/Unknown assets. No authenticated Jupiter result is claimed.
