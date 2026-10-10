# cNFT discovery investigation

Historical investigation, reviewed 2026-09-24. **At that time the CLI could not reliably enumerate a wallet's compressed NFTs.** The existing infrastructure is an on-demand standard-RPC client with no historical archive, snapshot or persistent owner index. Adding a recent-wallet-history loop would not solve that gap.

The reported approximately 53 items on Solscan are not evidence that `getTokenAccountsByOwner` or a Merkle account exposes those assets. We did not query Solscan or another external asset index, and do not claim to have independently verified that count.

## Current implementation and CLI scope

Matching-network DAS provides paginated owner discovery for Tauri `analyze_wallet` and CLI/desktop cleanup. Bubblegum v1/v2 burn adapters validate fresh proofs against RPC tree/config data. Rust reads `DOCK_FLINTS_DAS_URL`; the endpoint must support `getGenesisHash`, `getAssetsByOwner`, `getAsset`, `getAssetProof`. Discovery verifies ownership, unburned/compressed state and deduplicates IDs. A late failure preserves confirmed items with partial coverage.

**Standalone CLI `scan --cnfts` still uses the RPC-only capability adapter** in `infra/solana/scan/cnft.rs`. It neither calls DAS nor uses that environment setting. JSON reports `status: "unsupported"`, `code: "historical_index_required"`, `items: null`; cNFT-only scan exits `2`. Mixed scans preserve independent results. This is not a confirmed-empty inventory.

Desktop without DAS retains classic/Core NFTs and marks compressed coverage Unsupported. Known unique NFTs show a number; empty classic/Core with unknown compressed coverage shows `—`, not total NFT=0. Invalid DAS URLs reject startup; verified genesis mismatch is Unsupported and failed verification is Failed.

See [cleanup coverage](cleanup.md), [current verification](current-verification.md) and [historical unified verification](unified-cleanup-verification.md). Research below describes RPC-only limitations as of **2026-09-24**; its official-source links belong to that dated investigation.

## Bubblegum V1 and V2

[V1 overview](https://www.metaplex.com/docs/smart-contracts/bubblegum) and [V2 overview](https://www.metaplex.com/docs/smart-contracts/bubblegum-v2) describe NFTs committed as hashed tree leaves. V2 adds Core collections, flags and additional hashes; V1/V2 trees and leaf formats are not interchangeable. A correct decoder must distinguish versions and handle their different instruction semantics.

The official [leaf schema source reviewed in that investigation](https://raw.githubusercontent.com/metaplex-foundation/mpl-bubblegum/main/programs/bubblegum/program/src/state/leaf_schema.rs) contains owner/delegate/nonce and data/creator hashes in V1, with collection/asset-data hashes and flags added in V2. `LeafSchemaEvent` carries a versioned schema and leaf hash. These preimages are event data; searching raw tree-account bytes for the wallet cannot enumerate them.

The official [transfer implementation](https://raw.githubusercontent.com/metaplex-foundation/mpl-bubblegum/main/programs/bubblegum/program/src/processor/transfer.rs) handles both legacy SPL compression and MPL compression paths. Transfers replace leaves; ownership therefore cannot be inferred from mint events alone. The [metadata structures](https://raw.githubusercontent.com/metaplex-foundation/mpl-bubblegum/main/programs/bubblegum/program/src/state/metaplex_adapter.rs) also differ: V1 carries collection verification alongside its key; V2 uses Core collection semantics.

## What standard RPC provides

| Data/API                                       | Useful for                                                                 | Why it is insufficient alone                                            |
| ---------------------------------------------- | -------------------------------------------------------------------------- | ----------------------------------------------------------------------- |
| Tree account via `getAccountInfo`              | Current commitments, canopy and bounded change history                     | Hashes cannot be reversed into all owner/name records                   |
| `getProgramAccounts`                           | Discover program-owned tree/config accounts                                | No generic wallet-owner field exists for individual cNFTs               |
| `getSignaturesForAddress(wallet)`              | Transactions referencing the wallet in account keys                        | It is not an event-data ownership query or complete asset-state history |
| `getTransaction` / `getBlock`                  | Instructions, inner instructions, metadata and event payloads, if retained | Requires comprehensive discovery, history coverage, decoding and replay |
| `getFirstAvailableBlock` / `minimumLedgerSlot` | Endpoint retention boundaries                                              | A boundary does not prove every required transaction/event is available |

Official RPC references: [signatures](https://solana.com/docs/rpc/http/getsignaturesforaddress), [transactions](https://solana.com/docs/rpc/http/gettransaction), [first available block](https://solana.com/docs/rpc/http/getfirstavailableblock), [local ledger retention](https://solana.com/docs/rpc/http/minimumledgerslot).

A known tree's complete signature history is a possible replay input; it still requires knowing **all** relevant trees, retaining every state transition, resolving versioned transactions and inner instructions, and proving coverage. An arbitrary wallet query does not supply that tree inventory.

[Metaplex's event/indexing documentation](https://www.metaplex.com/docs/smart-contracts/bubblegum-v2/stored-nft-data) explains that compression and Bubblegum emit events through no-op instruction data to avoid textual log truncation. Reading only `logMessages` is therefore insufficient. Its reference deployment uses validator notifications, queues, an ingester and a database.

[Concurrent Merkle trees](https://www.metaplex.com/docs/smart-contracts/bubblegum-v2/concurrent-merkle-trees) keep a bounded changelog to accommodate concurrent proofs. The changelog and canopy are not a full historical database of leaf preimages. A membership proof verifies a known leaf; it cannot discover unknown owners or prove that an owner inventory is complete.

## Feasible self-hosted approach

This is technically possible without an external NFT indexing service, but requires infrastructure beyond the supplied RPC client:

1. Establish complete coverage from deployment/tree creation through a finalized checkpoint, using retained raw transactions/blocks, or begin from a verifiable complete snapshot.
2. Decode successful Bubblegum V1/V2 instructions and authenticated no-op/compression events, resolving versioned account keys and CPI context. Ignore failed transactions; reject unknown versions or gaps instead of declaring completion.
3. Replay state changes in canonical slot/transaction/instruction order, checking tree sequence continuity. Handle mint, transfer, delegate, metadata/collection changes, burn, redeem/decompress/cancel flows and version-specific behavior.
4. Persist asset ID, tree/index, current owner/delegate, metadata preimages, live/burned/compressed state and coverage checkpoints. Index the current owner. Resolve forks or ingest finalized data; make replay resumable and idempotent.
5. Continuously ingest new events using an owned validator/Geyser feed or a fully retained, pollable RPC block stream. Label results with their indexed-through slot and lag. Maintain Merkle state/root verification for integrity; proofs are needed if later tooling operates on leaves. The CLI considered in this historical investigation was read-only; current cleanup has separate mutation adapters.
6. Before claiming a complete inventory, reconcile roots/sequence numbers and prove that every relevant tree and event interval was covered. A configured subset of trees yields only a partial inventory.

A minimal persistence engine could use SQLite; it does not remove the archive, decoder and coverage requirements. Full-chain replay through a rate-limited public endpoint is not a small, fast per-wallet CLI feature. This repository has no provided archive or trusted complete snapshot, and does not add a half-working local indexer or import unverifiable external inventories.

## Historical RPC-only implementation (2026-09-24)

At that time, `scanner::cnft::get_compressed_nfts` returned an explicit unsupported capability status, not an empty asset vector. It made no unrelated RPC calls. Mixed scans retained successful categories and printed one concise cNFT notice. cNFT-only scans exited `2`; JSON retained `status: "unsupported"`, `code: "historical_index_required"` and `items: null`. The current standalone CLI scan preserves this behavior under `infra/solana/scan/cnft.rs`; the desktop/cleanup DAS path is described above.

That investigation added no Bubblegum crate solely for isolated event decoding:
this would not supply the missing inventory. The current workspace does include
`mpl-bubblegum` for standard-aware burn/proof validation. Its version and other
Solana/Metaplex versions are defined in the current `Cargo.toml`/`Cargo.lock`.
