# Architecture and maintenance

`dock_flints` is a Cargo workspace containing shared `dock-flints-core`, CLI and React/Tauri 2 desktop. Core/app/infra separate rules, use cases and external integrations. React handles presentation/navigation; Rust owns discovery, classification, signers, immutable plans, execution and accounting.

User guides: [getting started](getting-started.md), [desktop](../apps/app/README.md), [CLI](cli-guide.md). Actual evidence: [current verification](current-verification.md) and [screenshots](screenshots/README.md).

## Structure and dependency direction

```text
crates/core/src/
  core/                 Pure types/rules; no HTTP, RPC clients or CLI
    asset.rs            Accounts, mint, metadata, closure eligibility
    wallet.rs           Snapshot, selection and scanner statuses
    classification.rs   Fungible/NFT standards
    inventory.rs        Aggregated holdings, backing accounts and standalone IDs
    categories.rs       Category evidence, counts, status and coverage
    nft_cleanup.rs      Standard-aware NFT targets/prepared operations
    amount.rs           Exact amount aggregation
    swap.rs             Requests, quotes, limits, transaction ingredients
    cleanup.rs          Plans/results, eligibility, protected selection
    error.rs            Semantic errors; NoRoute differs from API failure
  app/
    scan_wallet.rs      WalletReader and scan orchestration
    categories.rs       Independent risk/routing/NFT coverage and classification
    pricing.rs          Optional PriceProvider valuation
    swap/               Provider/executor, preview and execute
    cleanup/            Planning and sequential execution boundaries
  infra/
    wallet.rs           Local identity and optional signer
    jupiter/            HTTP/auth/provider DTO conversion
    solana/             RPC, instructions, simulation, confirmation, accounting
      scan/             Token/mint/Metaplex/Core/DAS discovery
      nft_cleanup/      Metaplex/Core/Bubblegum preparation and burn
apps/cli/src/cli/        Argument wiring, commands, approval, table/JSON output
apps/app/src-tauri/src/
  commands/identity.rs  Public sessions and Rust-only signers
  commands/wallet.rs    Network → snapshot → enrich/classify → DTO
  commands/cleanup.rs   Saved plans, approval, jobs and progress
  cleanup.rs            Execution/journal reconciliation integration
  journal.rs            Durable nonsensitive signatures and file locking
  state.rs              Shared provider clients/configuration
  dto/                  CamelCase IPC and exact integer strings
apps/app/frontend-contract/ Typed requests, responses and invoke wrappers
apps/app/src/
  app/                  Session/navigation guards and phone shell
  features/wallet/      Connect dialog, scan lifecycle and MainScreen
  features/assets/      Inventory, categories, normalization and presenters
  features/cleanup/     Selection, immutable plans, progress and reports
  features/preview/     Labelled demonstration fixtures
  shared/               API boundary, assets, formatting and UI primitives
```

CLI/Tauri depend on APP → CORE; INFRA implements APP boundaries using CORE. APP does not import concrete adapters. Traits exist at external boundaries: reading chain, pricing, building swaps and executing operations. Static async dispatch needs no DI framework.

Core Solana keys/instructions/blockhashes are blockchain data, not network transport. Jupiter DTOs stay in their adapter and become neutral core transaction ingredients. Compilation, signing and submission live in Solana infrastructure. Two Pubkey SDK versions require byte-based conversion at adapter boundaries; verified SDK decoders remain in use.

## Refactoring history

| Earlier location                                       | Current responsibility                                   |
| ------------------------------------------------------ | -------------------------------------------------------- |
| `models/`, `classification/`, `portfolio/aggregate.rs` | `core/{asset,wallet,classification,amount}.rs`           |
| `swap/mod.rs`                                          | `core/swap.rs`, `app/swap/`, Jupiter adapter DTOs        |
| `portfolio/service.rs`                                 | `app/scan_wallet.rs`, Solana inventory, CLI wiring       |
| `cleanup/` and CLI selection                           | `app/cleanup/{plan_wallet,plan,execute}`                 |
| `rpc/`, `scanner/`, `pricing/jupiter.rs`               | `infra/solana/`, `infra/jupiter/`                        |
| Scattered output/main                                  | `cli/output/`, `cli::run` and a runtime-only main        |
| Repeated account `.find()`                             | Local BTreeMap/BTreeSet indexes                          |
| `solana-client` networking                             | HTTP RPC-only client/API, without unused TPU/QUIC/pubsub |

This map explains an earlier migration; it is not a fresh validation receipt.

## Analysis data path

```text
ConnectWalletDialog → connect_wallet → public WalletConnection
  → AnalyzeWalletRequest (walletAddress, selection, noPrices)
  → shared API → frontend contract → analyze_wallet
  → bounded genesis verification → scan_wallet_observed → WalletSnapshot
  → optional network-matched DAS → canonical inventory → categories::classify
  → WalletAnalysisDto → useWalletAnalysis → normalizeWalletCategories
  → MainScreen / CategoryDialog → presentCategory
```

The normal dialog enables balance/tokens/allTokens/nfts/cnfts and prices. On-chain inventory is shared and independent enrichment results are retained. Keys agree across core/DTO/TypeScript: `scam`, `nft`, `dust`, `dead_token`.

Core aggregates mint/program holdings and backing accounts. Frontend deduplicates category IDs/accounts and aligns count with items. NFT compatibility fallback exists only when the authoritative category is missing. MainScreen does not independently total NFT lists.

Presenter separates counter/status/reason/coverage: number or `—`, complete-empty `0`, known partial item count without words/`+`, empty incomplete `—`. Explanations stay in tooltip/accessible description/details. Sample observations are absent from real analysis.

Jupiter evidence requires verified mainnet; genesis errors are retried and do not become verified Unsupported/NoRoute. Independent on-chain assets survive. NFT does not depend on pricing/risk; DUST does not depend on routing. DAS verifies network/ownership/burned state; missing DAS preserves classic/Core but prevents an otherwise empty NFT list from proving total zero. The current status-composition regression failure is recorded in [verification](current-verification.md).

`useWalletAnalysis` clears old snapshots on rescan, verifies owner and discards stale generations. App also rejects dismissed responses and guards live hash routes. Dismiss is not RPC cancellation.

## Cleanup and recovery

React sends canonical `assetIds`/`ignoredAssetIds`; IPC retains mint compatibility. Empty selected is none; all is explicit. Ignored mints protect all backing accounts, including empty ones. Plans are bound to session/wallet/network/revision/expiry. Public key can prepare, not execute.

Registered commands: connect_wallet, disconnect_wallet, analyze_wallet, prepare_cleanup, execute_cleanup, get_cleanup_job. Lost replies are recovered by job/plan ID lookup without another economic send. Journal survives restart; full sessions/plans/reports remain in memory. Persistent journal and saved-plan expiry are desktop adapter features, not standalone CLI guarantees.

## Maintenance boundaries

- Providers: implement APP traits, convert DTOs in INFRA and replace wiring in the entry adapter.
- Cleanup policy: update core/app rules and behavioral tests; renderer/CLI do not infer burn eligibility.
- Signing: execution boundary and `infra/wallet.rs`; snapshots/plans expose public identity only.
- Output: CLI output adapters without blockchain reads.
- Arguments: shared CLI argument groups; scan display flags do not spill into swap/cleanup.
- NFT/cNFT: standard-aware Solana adapters with matched-network discovery/proofs; see [coverage](cleanup.md).

Never serialize/log secret material. Seed is exactly 32 raw Ed25519 bytes in base64. Ignore restrictions outrank selection and remain enforced at execution. Protected WSOL can prevent native-SOL swaps that might unwrap existing accounts. Burns require approved plans and revalidated reasons; API/auth/liquidity errors are not NoRoute. Reconcile uncertain signatures instead of resending; reread zero balance before close.

## Checks

```bash
cargo fmt --all --check
cargo check --workspace --locked
cargo test --workspace --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
```

Tests use binary fixtures, mock RPC/HTTP and generated test identities. Product documentation and licensing changes do not need live swap/burn/close. See [IPC semantics](../apps/app/frontend-contract/README.md) and [actual integration limits](../apps/app/BACKEND_GAPS.md).
