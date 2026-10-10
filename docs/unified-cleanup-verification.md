# Unified cleanup verification

> This report records cleanup evidence before the category display update in
> `e013ea9`. References below to `Not checked` describe the earlier UI. Current
> tiles display a number or `—`; see [current verification](current-verification.md)
> and the [screenshot gallery](screenshots/README.md).

Verified on 2026-10-10 against the working tree based on `fb6996c`. No transactions
were signed or sent for the user's public devnet wallet. All transaction tests used
an independently generated disposable wallet on an isolated loopback validator.

## Confirmed causes and changes

- The previous default Auto planner required `NoRoute` before fungible burn, while
  Jupiter is structurally unavailable on devnet/local networks. ExplicitDiscard
  bypassed swaps and required a separate UI policy. `Complete` now prefers a real
  route, then permits approved burn for genuine NoRoute or a typed structural
  `UnsupportedNetwork`. HTTP/auth/429/timeouts remain provider failures, never burn
  evidence. Execution rechecks the approved reason and never replaces Swap with Burn.
- Overlapping `tokens`/`allTokens` rows and first-record selection lost additional
  backing balances. Core now deduplicates account addresses, aggregates exact amounts
  by mint + program and projects one canonical selectable inventory. Enriched NFT
  labels no longer get overwritten by an empty legacy mint metadata record.
- Generic SPL eligibility excluded every NFT, and standalone Core/cNFT IDs had no
  execution path. Standard-specific Metaplex/Core/Bubblegum adapters now prepare,
  revalidate and burn these identities without routing NFTs through BurnChecked.
- Empty close and WSOL unwrap are independent of market data. Classification changes
  after a burn cannot block closing the verified zero-balance source. WSOL sources
  execute first; a failed/protected/unresolved unwrap blocks dependent swaps, preserving
  the saved source identity and preventing implicit recovery of an excluded WSOL balance. Zero-decimal
  multi-unit fungibles remain eligible; ambiguous supply-one mints remain Unknown.
- IPC selected Core/cNFT IDs could become the CLI's empty-account ALL selection;
  camelCase enum fields also needed explicit serde configuration. Selected-only/NONE
  is enforced server-side, mint protection covers every backing account, and protected
  standalone IDs remain untouched.
- The desktop no longer exposes a policy selector or an additional approval dialog.
  RECOVER SOL approves the displayed saved plan, including its irreversible burn
  notice. Real progress, signed transaction accounting and refreshed inventory remain
  separate from design preview data. Production cannot load synthetic manifest observations.

## Public devnet read-only audit

Wallet: `9FCR2PU1jZgCHyjWxzk2BNQHJxszAK24vBFiWmUyRNpv`.
RPC: `https://api.devnet.solana.com`, verified genesis
`EtWTRABZaYq6iMfeYKouRu166VU2xqa1wcaWoxPkrZBG`.

| Stage                                                    | Current live result                                                           |
| -------------------------------------------------------- | ----------------------------------------------------------------------------- |
| Legacy discovery / Token-2022 discovery                  | 0 / 0 accounts                                                                |
| Core snapshot / analysis DTO / React canonical inventory | 0 discovered token accounts / 0 selectable items                              |
| Classic NFT / Core coverage                              | Complete, 0 confirmed items in each                                           |
| Compressed coverage                                      | Unsupported: no network-matched DAS configured; total NFT coverage is Partial |
| SCAM / DUST / DEAD TOKEN                                 | Not checked: mainnet-only provider coverage unavailable on devnet             |
| Selected / actionable / skipped discovered accounts      | 0 / 0 / 0                                                                     |
| Read-only plan                                           | Swap 0, Burn 0, Close 0; canExecute false                                     |

These results were obtained through raw RPC, registered Tauri IPC with live RPC, and
then the actual packaged release WebKitGTK application. Missing DAS is not evidence
of zero compressed NFTs. A completed execution means its approved known targets
completed; the inventory coverage warning still applies. No discovered account was omitted; the present inventory
is empty for the completed on-chain checks, not a failed RPC disguised as empty.

Evidence: [genesis RPC](fixtures/unified-devnet-genesis-rpc.json),
[legacy RPC](fixtures/unified-devnet-legacy-rpc.json),
[Token-2022 RPC](fixtures/unified-devnet-token2022-rpc.json),
[native React/DTO/plan audit](fixtures/unified-native-devnet-readonly.json).

The older [2026-10-09 snapshot](fixtures/devnet-account-audit.json) is **historical**:
26 legacy accounts, 6 empty, 20 nonempty fungibles. Its regression plan accounts for
all 26 exact addresses/mints/programs/raw amounts/decimals, with **20 Burn + 26 Close**,
no policy switch and no fabricated NoRoute. This is a recorded-input regression,
not today's live inventory or transaction execution. Every per-account reason and
operation is in [the recorded plan](fixtures/unified-recorded-devnet-plan.json).

## Categories and coverage

All counts derive from deduplicated items, not token-account counts; flags can overlap.
SCAM requires an explicit provider suspicious flag and retains source/reasons/time.
The [current Tokens guide](https://developers.jup.ag/docs/tokens/token-information) does not guarantee an `audit.isSus` field. The adapter treats it as conditional: absent/nonboolean fields remain Unknown;
unverified status, authorities or organic score do not independently prove scam.
DUST means a fungible mint/program holding with **0 < valued USD <= 0.01 USD** by
default (`DOCK_FLINTS_DUST_USD` changes the threshold); unpriced never means zero.
DEAD TOKEN means confirmed provider-specific NoRoute at its recorded time, not global
worthlessness. NFT requires verified standard/edition/owner evidence, independent
of Jupiter. Classic/Core results survive missing/partial DAS. Complete-empty,
Partial, Failed and Unsupported remain distinct statuses with coverage reasons.

## Actual local-validator and packaged desktop execution

Solana validator **3.1.10**, genesis
`BCE2vG87r74T8BeXPeRK21PFxNEs4LMKBKfPKSvrmrgU`, RPC `127.0.0.1:18890`.
Owned disposable public wallet:
`BkC3FX6o6bjJ8aDRF8BkhpQA8NqQneoWp2FR3VwoxMMe`.
Official Metaplex Token Metadata and MPL Core programs were cloned read-only from
devnet into this isolated ledger. Secrets stayed in `/tmp`; none are in evidence files.

The final release application at `tauri://localhost` connected using its Rust
keypair-file session, scanned **4 accounts / 4 selectable assets**, displayed
**Burn 3 / Close 2** and executed the saved plan after one final RECOVER SOL press.
The standalone Close count excludes source accounts closed internally by NFT burn
instructions. WSOL was unwrapped before other cleanup operations, never burned.

| Asset                                   | Real operation                                                     | Confirmed signed wallet delta                            |
| --------------------------------------- | ------------------------------------------------------------------ | -------------------------------------------------------- |
| Native-backed WSOL, 7,960,720 raw units | CloseAccount unwrap, account absence verified                      | +9,995,000 lamports, including source rent and minus fee |
| 42 units, zero-decimal fungible mint    | BurnChecked, zero verification, CloseAccount, absence verification | +2,029,280 lamports across two transactions              |
| Classic NFT                             | Metaplex BurnV1, source absence verified                           | +8,179,960 lamports                                      |
| Programmable NFT                        | Metaplex BurnV1 + unlocked TokenRecord, source absence verified    | +9,627,640 lamports                                      |
| Total                                   | **5 confirmed transactions, completed 4, closed 4, failed 0**      | **+29,831,880 lamports = +0.02983188 SOL**               |

The total is a sum of confirmed transaction metadata deltas including all 25,000
lamports of transaction fees, not burned token valuation or whole-wallet before/after
subtraction. Gross source-account returns were 16,117,840 lamports, including WSOL
principal and rent; Metaplex also refunded metadata/edition/token-record rent.
Plan estimates currently include generic source-account returns, not every possible
NFT protocol-account refund. Actual results use confirmed receipts. Refresh returned
zero remaining source accounts/items. The durable journal recorded all five signatures.
An immediate independent RPC getTransaction audit retained each transaction and
matched its owner's signed pre/post delta to the execution receipt, then matched
their exact sum to the final report.

[Final native inventory/plan/report/rescan](fixtures/unified-native-wsol-cleanup.json),
[immediate confirmed RPC receipts](fixtures/unified-native-wsol-rpc-receipts.json),
[signature journal](fixtures/unified-native-wsol-journal.json).

An earlier native run exercised fungible/classic/pNFT cleanup without WSOL and
returned +19,836,880 lamports across four transactions:
[inventory/plan/report/rescan](fixtures/unified-native-local-cleanup.json),
[journal](fixtures/unified-native-local-journal.json),
[later absence audit](fixtures/unified-local-post-audit.json).
That earlier ledger's bounded retention pruned raw metadata before its later audit;
its report/journal retain metadata-derived deltas observed at confirmation. The
final run above retained immediate RPC metadata independently. Additional core-only
real checks are in [fungible execution](fixtures/unified-local-fungible-cleanup.json)
and [classic/pNFT execution](fixtures/unified-local-nft-cleanup.json).

### NFT coverage and validation limits

| Standard             | Implemented path                                                                      | Executed validation                                                   |
| -------------------- | ------------------------------------------------------------------------------------- | --------------------------------------------------------------------- |
| Classic/master NFT   | Metaplex BurnV1                                                                       | Real local-validator + release React/Tauri execution                  |
| Legacy print edition | BurnV1, verified parent token/master/edition-marker bitmap; prints before master      | Binary SDK + mock RPC regression; no real print burn in this run      |
| pNFT                 | BurnV1 + TokenRecord                                                                  | Real unlocked pNFT local-validator + release desktop execution        |
| Programmable edition | Same standard-specific parent/TokenRecord path                                        | Binary SDK + mock RPC regression                                      |
| MPL Core AssetV1     | Core BurnV1, asset/collection/plugin/Asset Signer validation                          | Binary SDK + mock RPC regression; no real Core burn in this run       |
| Bubblegum v1 / v2    | Official versioned burn, fresh full proof, actual tree/config/leaf hash/canopy checks | Binary SDK + mock RPC regression; no real compressed burn in this run |

Explicit limits remain visible: EditionMarkerV2; locked/delegated pNFT records;
frozen, unknown or external lifecycle Core plugins; nonempty Asset Signer or missing
complete matching-network DAS coverage for its compressed holdings; invalid/stale
proofs, frozen Bubblegum v2 flags, unknown tree versions and proof/account sets needing
lookup-table support. A master with unselected/unowned/blocked prints cannot burn.
NFT/cNFT adapters are not advertised as universally executable for unsupported states.
No live mainnet swap or devnet Jupiter swap was executed or claimed.

## Checks

| Check                                                 | Result                                                                                                                                                                                                                 |
| ----------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| cargo fmt --all --check                               | Passed                                                                                                                                                                                                                 |
| cargo check --workspace                               | Passed                                                                                                                                                                                                                 |
| cargo test --workspace                                | 109 passed; 1 live read-only test intentionally ignored by default                                                                                                                                                     |
| cargo clippy --workspace --all-targets --all-features | Passed, no warnings                                                                                                                                                                                                    |
| Live registered-command Tauri IPC read-only test      | Passed against actual public devnet RPC                                                                                                                                                                                |
| npm ci                                                | Passed; audit reported 0 vulnerabilities                                                                                                                                                                               |
| npm run check                                         | Passed: TypeScript, ESLint, 53 unit tests, production Vite build                                                                                                                                                       |
| npm run format:check                                  | Passed                                                                                                                                                                                                                 |
| npm run test:e2e                                      | 85 passed in the full suite; 9 cleanup/devnet E2E tests rerun after final fixes, all passed; including NONE, protected accounts, standalone Core/cNFT selection/report, partial/error/preview, phone/reference layouts |
| npm run desktop:build                                 | Passed, optimized native executable with embedded production frontend                                                                                                                                                  |
| Actual packaged release launch                        | Passed on Arch Linux, WebKitGTK 2.54.1, DPR 2, CSS viewport 1100×1000                                                                                                                                                  |

Browser/E2E mocked execution is not on-chain validation. Native validation used the
real local validator and registered IPC permissions; it was not Chromium presented
as WebKitGTK. Packaged PNG/SVG assets loaded with their expected natural sizes, local
fonts were loaded, main overflow was zero and category-label overflow measurements
were zero. Existing stars/artwork/styles/navigation remained unchanged. E2E tests
also cover 360×800, 390×844, 430×932 and desktop 1440×900 compositions. Native DPR 1
and fractional desktop scaling were not tested in this run.

The original packaged WebKitGTK screenshots were stored under
`apps/app/docs/screenshots/unified/` as `native-owned-{main,plan,processing,success}.png`
and `native-readonly-{main,nft-coverage}.png`. Those historical PNG files are not
included in this checkout. The [current screenshot gallery](screenshots/README.md)
contains the separately recorded documentation captures from 2026-10-10.

## Configuration and reproduction

No policy/devnet selector or manifest setup is required. Export the desired
`DOCK_FLINTS_RPC_URL` before launching desktop; Rust verifies its genesis.
Rust reads inherited environment; `.env` is not automatically loaded. Mainnet
Jupiter should be configured with `JUPITER_API_KEY`; do not put secrets in VITE variables. The [Swap guide](https://developers.jup.ag/docs/swap) requires a key while the [rate-limit page](https://developers.jup.ag/docs/portal/rate-limits) describes a keyless tier. The client can omit the header, but availability is provider-controlled; authentication failure never means NoRoute. Configure
`DOCK_FLINTS_DAS_URL` with matching genesis and paginated getAssetsByOwner/getAsset/
getAssetProof support for compressed coverage and Core Asset Signer safety.
`DOCK_FLINTS_JOURNAL_PATH` selects the nonsecret durable signature journal.
See [backend.env.example](../apps/app/backend.env.example).

To repeat actual mutations, create a new disposable keypair and isolated validator,
clone the official programs, fund only that test wallet and create its own SPL/NFT
fixtures. `cargo run -p dock-flints-core --example nft_local -- <LOOPBACK_RPC>
<OWNED_KEYPAIR> --prepare-only` creates classic/pNFT fixtures without cleaning them.
Connect that file through the real desktop UI, review the unified burn/close plan,
press RECOVER SOL once and inspect the confirmed report/rescan. Never substitute the
user's wallet/keypair into this mutation test. Inspector used for evidence was bound
to loopback for testing only; do not expose it as a production remote inspector.

## Changed implementation areas

- Core: `core/inventory.rs`, `core/{cleanup,nft_cleanup,error}.rs`,
  `app/cleanup/{mod,plan,execute}.rs`, `app/categories.rs`, test-only observations.
- Infrastructure: `infra/solana/nft_cleanup/{mod,metadata,core,compressed}.rs`, existing
  SPL cleanup/sender/DAS reader, conditional Jupiter risk evidence adapter.
- Desktop: Tauri wallet/cleanup commands, state, lifecycle and typed DTOs;
  `frontend-contract/{types,cleanup}.ts`.
- React: canonical asset presentation, single-action cleanup orchestration/screens,
  category statuses/copy and confirmed report. No style/assets/navigation redesign.
- CLI compatibility, binary/RPC/IPC/frontend/E2E regressions, owned integration
  examples, README/configuration/architecture/cleanup documentation and public evidence.
