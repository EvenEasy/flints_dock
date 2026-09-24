# flints_station

A read-only Solana wallet portfolio CLI. Discovery uses standard Solana RPC; optional market prices use Jupiter. It never signs transactions, closes accounts, burns tokens, or calls an asset indexer.

```bash
cargo run -- --pubkey EibQ2VYpzj18qSdEBkmxWVzde7FzamTxVG9rZyY689Yj --no-prices
cargo run -- --pubkey EibQ2VYpzj18qSdEBkmxWVzde7FzamTxVG9rZyY689Yj \
  --rpc-url https://api.mainnet.solana.com --format json --no-prices
```

The default RPC endpoint follows the current [official Solana cluster documentation](https://solana.com/docs/references/clusters). Supply `--rpc-url` for another standard RPC endpoint. Public endpoints can rate-limit or reject program-wide queries; the corresponding scanner reports failure while successful scans remain available. `--timeout-seconds` defaults to 30 per request; RPC retry handling can make total runtime longer.

For prices, set `JUPITER_API_KEY` in your environment and omit `--no-prices`. [Jupiter Price V3](https://developers.jup.ag/docs/price) uses `https://api.jup.ag/price/v3` with an `x-api-key` header, in batches of up to 50 mint IDs. RPC provides balances, not market quotes. Missing credentials skip pricing; missing quotes and service failures leave values unavailable. Pricing assumes mainnet mints; use `--no-prices` with devnet, testnet or local validators. The API key is never included in output. No live authenticated pricing request was made during validation because no key was supplied.

`--verbose` sends progress to stderr and adds individual token accounts to table output. JSON stdout remains a single portfolio document. Invalid public keys and total RPC outages exit unsuccessfully; partial scans exit successfully with explicit scanner statuses.

## Architecture

- `cli` parses arguments; `main` builds the confirmed-commitment client and selects output.
- `models` contains serializable domain results and scan statuses.
- `rpc` centralizes account filters, deduplicated 100-account batches and pubkey conversions. `RpcClient::send` requests base64 token accounts through the existing client transport; a second HTTP JSON-RPC client is unnecessary.
- `scanner` contains independent SOL, token, mint metadata, Metaplex, Core, stake and nonce scanners, plus explicit cNFT capability reporting.
- `portfolio` runs independent discovery concurrently, enriches discovered mints, aggregates holdings and combines partial results.
- `pricing` provides an optional provider interface and Jupiter implementation.
- `output` handles terminal formatting. Scanners never print. On-chain labels are escaped before terminal rendering.

```text
.
├── Cargo.lock
├── Cargo.toml
├── README.md
├── docs
│   ├── research.md
│   └── validation.md
├── src
│   ├── lib.rs
│   ├── main.rs
│   ├── cli/mod.rs
│   ├── models
│   │   ├── mod.rs
│   │   └── scan.rs
│   ├── output
│   │   ├── mod.rs
│   │   └── console.rs
│   ├── portfolio
│   │   ├── mod.rs
│   │   ├── aggregate.rs
│   │   └── service.rs
│   ├── pricing
│   │   ├── mod.rs
│   │   └── jupiter.rs
│   ├── rpc/mod.rs
│   └── scanner
│       ├── mod.rs
│       ├── accounts.rs
│       ├── cnft.rs
│       ├── core.rs
│       ├── metadata.rs
│       ├── nft.rs
│       ├── sol.rs
│       ├── stake.rs
│       └── tokens.rs
└── tests/scanning.rs
```

## Coverage

“Complete” means the named, scoped query completed at the chosen RPC endpoint, not that every possible Solana asset class was enumerated. Requests are independent observations, not an atomic single-slot snapshot.

| Asset/account class | Implemented scope |
| --- | --- |
| Native SOL | Exact lamports at the requested address |
| Legacy SPL Token | All returned owner accounts, including zero balances, frozen accounts and wrapped SOL |
| Token-2022 | Owner accounts of variable size, public raw balances, mint decimals, extension details and embedded metadata; confidential balances cannot be decrypted |
| Token aggregation | By mint **and program**, retaining every decoded underlying account |
| Metaplex NFTs | NonFungible, NonFungibleEdition, programmable variants, and older mints proven by valid master edition/edition accounts |
| MPL Core | Uncompressed AssetV1 accounts with the wallet as owner; variable sizes accepted |
| Stake | Accounts where the wallet is staker or withdrawer, deduplicated; lamports, delegation, vote account, epochs and lockup retained |
| Durable nonce | Standard initialized nonce accounts with the wallet as authority |
| Unknowns | Unclassified/missing-metadata tokens and undecodable account candidates retained, with reasons and available lamports/raw data |

NFT recognition requires a held raw unit, decimals zero, mint supply one, and Metaplex token-standard or legacy edition evidence. A UI amount of `1` with decimals `6` or `9` is never NFT evidence. Metadata and edition account discriminators, program owners and mint identities are validated.

Partial or unsupported coverage is explicit:

- Bubblegum compressed NFTs cannot be fully enumerated by wallet using standard RPC. See [Metaplex's cNFT data model](https://www.metaplex.com/docs/smart-contracts/bubblegum-v2/fetch-cnfts). Tree accounts do not provide a wallet-owner map. Transaction replay would require complete relevant history and an index; no misleading short-history heuristic or DAS call is used.
- Core HashedAssetV1 records are not owner-enumerable. Core plugin permissions, inherited collection rules and collection administration rights are not interpreted as wallet ownership.
- Stake discovery covers direct authorities. Activation/cooldown is not inferred from epochs alone. `wallet_can_withdraw` means the wallet matches the withdraw authority; lockups, active stake and other conditions still apply.
- Token-2022 external metadata pointers need their target program's decoder. Embedded metadata is supported. Scaled/interest-bearing units are reported as exact base units; incompatible price semantics are excluded. Confidential balances and withheld fees are not silently added to spendable holdings.
- Off-chain metadata JSON is not fetched. Image URI is populated only when embedded on-chain. Names/symbols are labels, not authenticity guarantees.
- Arbitrary PDAs, escrow accounts, delegates, multisig-controlled assets and protocol-specific DeFi positions require additional adapters. Solana's `account.owner` is a **program ID**, not a generic wallet beneficiary field.

## Accounting and JSON

Native lamports and each account's raw amount are `u64`; aggregated raw token amounts and lamport summaries use `u128`. Token display amounts use decimal-point placement, with no floating-point accounting. Unknown mint decimals are `null`, not fabricated zeroes. Raw token amounts and aggregate integer totals serialize as decimal strings so JavaScript consumers can preserve precision. Other `u64` JSON fields should be parsed with an integer-aware parser.

USD quotes and valuations are explicitly approximate `f64` values. `known_value_usd` sums available native SOL and fungible token valuations; it is `null` when none are available. It excludes NFT/Core valuations, stake/nonce lamports, account rent and all unpriced assets. It is not a total portfolio valuation. Wrapped SOL is valued once as tokens; its account lamports are never added again.

Token account lamports are actual balances, not fixed rent estimates. A separate conditional reclaim estimate includes only empty accounts whose close authority is the wallet and whose extensions do not require further review (ImmutableOwner alone does not block closure). Other extension-bearing accounts remain “needs review”; withheld/confidential amounts can matter even with public amount zero. Nonempty wrapped SOL principal is not called rent. No rent-exemption threshold is invented, and no closing instruction is submitted.

JSON provides the complete `token_accounts`, `mints`, aggregated `tokens`, NFT/Core/stake/nonce records, `unknown_assets`, `account_summary`, per-scanner `scanners` statuses, and valuation scope. `complete`, `partial`, `failed`, `unsupported` and `skipped` are distinct. For example:

```json
{
  "compressed_nfts": {
    "status": "unsupported",
    "reason": "Compressed NFTs are not fully enumerable using standard RPC without an indexer..."
  },
  "prices": {
    "status": "skipped",
    "reason": "Disabled by --no-prices"
  }
}
```

## Verification

```bash
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features
```

Normal tests use binary fixtures and a mocked Solana RPC transport; no live network is required. They cover exact formatting, aggregation, empty-account assessment, Token-2022 extensions/metadata, pubkey conversion, NFT discriminators and editions, Core/stake decoding, pricing math, batch sizes/deduplication, partial failures, full outages and terminal/JSON output.

See [research notes](docs/research.md) for sources and locked dependency boundaries, and [validation results and example output](docs/validation.md) for the supplied mainnet wallet run.
