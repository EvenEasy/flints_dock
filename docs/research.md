# Implementation research

> Historical implementation research. Current setup, desktop capabilities and
> verification are indexed in [the documentation guide](README.md).
> Desktop analysis now supports a configured network-scoped DAS provider; the
> RPC-only index limitation below still applies to standalone CLI `scan`.

Reviewed 2026-09-24 against the repository's installed sources and lockfile. Existing working owner queries and Core filters were reused. No Solana/Metaplex upgrade was required. The unused `solana-sdk` umbrella dependency was removed; this prunes unused lockfile packages. Direct serde/JSON/HTTP/account dependencies already existed transitively. Test-only base64, Borsh 0.10 and async-trait dependencies also use existing compatible versions.

## Crate boundaries

| Integration | Locked version | Relevant API |
| --- | --- | --- |
| RPC client / account decoder | 4.3.0 | `RpcClient::send`, `get_program_ui_accounts_with_config`, `get_multiple_accounts`, local SPL decoders |
| RPC pubkey | 4.4.0 | `Pubkey::new_from_array` |
| Token Metadata | 5.1.1 | `Metadata`, `MasterEdition`, `Edition`, `TokenStandard`; pubkey **2.4.0** |
| MPL Core | 0.12.1 | `BaseAssetV1`, `Key`; its solana-program 3.0.0 uses pubkey **3.0.0** |
| SPL Token interface | 3.0.0 | Program ID |
| Token-2022 interface | 3.1.1 | Program ID, `StateWithExtensions`, TLV extension types |
| RPC account | 4.7.0 | Raw account model |

The Token Metadata/RPC conversions are named helpers; Core's program ID is converted directly by byte array. A similarly named `Pubkey` from another version is never passed through implicitly.

## RPC and layouts

- [getTokenAccountsByOwner](https://solana.com/docs/rpc/http/gettokenaccountsbyowner): both program IDs come from the installed SPL crates. The convenience method forces parsed JSON, so the common RPC transport requests base64 and the application decodes locally. This avoids losing accounts when RPC cannot resolve mint metadata.
- [getMultipleAccounts](https://solana.com/docs/rpc/http/getmultipleaccounts): deduplicated batches of at most 100 for mints, metadata PDAs and legacy edition PDAs. Batch errors are distinct from absent accounts.
- [getProgramAccounts](https://solana.com/docs/rpc/http/getprogramaccounts): ordinary memcmp filters for Core, with local type/authority validation. No indexing-service RPC methods are used.
- [getBalance](https://solana.com/docs/rpc/http/getbalance) and [official endpoints](https://solana.com/docs/references/clusters): integer lamports; the current documented mainnet endpoint is `https://api.mainnet.solana.com`.
- [Core fetching](https://www.metaplex.com/docs/smart-contracts/core/fetch): Core supports GPA discovery. Installed `BaseAssetV1` confirms discriminator at byte 0 and owner at byte 1; no fixed data size is assumed. Base fields are decoded even with a plugin tail, with permissions explicitly unassessed.
- [Metaplex token standards](https://www.metaplex.com/docs/smart-contracts/token-metadata/token-standard): classification covers classic/edition/programmable standards. Installed generated decoders deserialize a `Key` without verifying its expected variant, so the scanner checks discriminators explicitly. Legacy V1 master editions also require their additional printing-mint fields.
- [Token-2022 metadata](https://solana.com/docs/tokens/extensions/metadata): the pointer may target another program; embedded TokenMetadata provides name/symbol/URI and additional fields. External target formats are not guessed. Installed account-decoder output is retained for other extensions.

## Compressed assets

See [the current V1/V2 investigation](cnfts.md) for official event/source references, historical replay feasibility, required self-hosted infrastructure and the CLI's unavailable status. No external asset index is used. Recent wallet transactions are never presented as a complete inventory.

## Pricing

[Jupiter's current Price documentation](https://developers.jup.ag/docs/price) and [API reference](https://jupiter.mintlify.app/api-reference/price) confirm Price V3, required `x-api-key`, `ids` batching (50), `usdPrice`, `decimals` and `blockId`. Omitted/unreliable quotes are kept unavailable. No deprecated public/lite endpoint or hardcoded SOL price is used. Market quotes remain separate from on-chain ownership.
