# Complete wallet cleanup

Core is independent of React/Tauri. CLI and desktop use the same scanner, normalized
inventory, planner, execution adapters and confirmed transaction accounting.

Для запуску та прикладів CLI дивіться [getting started](getting-started.md) і
[CLI guide](cli-guide.md). Нижче описано спільні cleanup rules; session/plans,
Channel, durable journal і автоматичний rescan належать Tauri desktop adapter.
Фактично виконані поточні перевірки: [current verification](current-verification.md).

## One scenario

Desktop: Connect → scan → normalized inventory → plan → uncheck assets to keep → RECOVER SOL
→ execute saved plan → confirmed report → rescan. No cleanup-policy selector.
The final button approves the displayed immutable plan, including its burn count
and short irreversible-action notice. Connection alone never authorizes transactions.

CLI: `cleanup` without `--execute` only prints a plan, including when a keypair
is provided. `--dry-run` makes this intent explicit. With `--execute`, review the
printed actions and type `cleanup` at the terminal prompt; `--execute --yes`
approves them without the prompt. Selection is supplied through CLI flags.

| Selected asset                                                              | Planned operation                                                           |
| --------------------------------------------------------------------------- | --------------------------------------------------------------------------- |
| Eligible empty SPL/Token-2022 account                                       | Close to owner, regardless of mint classification/pricing                   |
| WSOL native-backed source                                                   | Unwrap through CloseAccount; never burn                                     |
| Fungible with genuine provider route                                        | Fresh swap to native SOL, verify zero, close source                         |
| Fungible with confirmed provider-specific NoRoute                           | Approved BurnChecked, verify zero, close                                    |
| Fungible on structurally unsupported swap network                           | Same approved burn/close, reason `routing_unavailable`, no invented NoRoute |
| NFT                                                                         | Standard-specific adapter described below                                   |
| Temporary provider failure, unknown kind, invalid authority/state/extension | Visible skip with structured reason                                         |
| User-protected asset                                                        | No mutation; mint protection also keeps every empty backing account         |

`getGenesisHash` identifies the active network. Jupiter Price V3, Tokens V2 and Swap V2 are
mainnet-only. Devnet/local RPC cannot turn Jupiter into another-network provider.
Missing keys, HTTP 429/auth/5xx, timeout, insufficient liquidity and malformed
responses are not NoRoute. Bounded retries preserve that distinction. An approved
swap never turns into burn during execution; newly available routes also invalidate
a previously approved no-route burn. The structural burn reason is revalidated.

Zero decimals alone are not NFT evidence. Valid multi-unit zero-decimal mints can
be FungibleAsset without contradictory metadata/edition evidence. Supply=1 with no
verified standard/edition remains Unknown. Empty close does not depend on a mint's
classification after burn. Native SOL remains in the wallet for transaction fees.

## Inventory and selection

Raw account addresses own balances. Normalize/deduplicate before enrichment and
classification: aggregate by mint + token program, retain all backing addresses and
exact summed raw amounts. Overlapping `tokens`/`allTokens` views do not add balances.
Core/cNFT use owned asset IDs without fictitious mint accounts. Undecodable accounts
remain visible with reasons. Failed enrichment cannot erase discovered holdings.

The desktop boundary distinguishes ALL/SELECTED/NONE, with `assetIds` and protected
`ignoredAssetIds`/`ignoredMints`. Empty SELECTED means NONE. Core-only selection
cannot expand an empty SPL allowlist into ALL. CLI's existing empty account allowlist
semantics remain compatible; `--asset <ID>` and `--ignore-asset <ID>` support standalone
NFTs. Repeat `--account`/`--ignore-mint` for account scope/mint protection.

```sh
cargo run -- cleanup --pubkey <WALLET> --rpc-url https://api.devnet.solana.com --dry-run
cargo run -- cleanup --keypair <FILE> --rpc-url <RPC> --ignore-mint <MINT> --execute
cargo run -- cleanup --keypair <FILE> --asset <CORE_OR_COMPRESSED_ID> --execute
```

For the documentation devnet wallet, use this read-only plan example:

```sh
cargo run -- cleanup \
  --pubkey 9FCR2PU1jZgCHyjWxzk2BNQHJxszAK24vBFiWmUyRNpv \
  --rpc-url https://api.devnet.solana.com --dry-run --format json
```

This does not load `dev/demo-wallet.json` or submit transactions. The signer
examples with `--execute` above describe signing workflows for a separately
chosen wallet; they are not documentation verification commands.

The old CLI `--explicit-discard --account ...` input is still accepted as a
compatibility alias to this complete scenario. No separate devnet behavior remains.
Older library callers may explicitly retain their existing policy enum; desktop
never accepts a frontend policy or frontend-authored instructions/transactions.

## NFT adapters and limits

| Standard                    | Adapter / completion                                                              | Explicit limits                                                                                     |
| --------------------------- | --------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------- |
| Classic/master NFT          | Token Metadata BurnV1; source absence verified                                    | Legacy SPL, exact one unit; verified metadata/edition, verified collection metadata when applicable |
| Print edition               | BurnV1 plus verified parent master mint/token/edition marker                      | Legacy EditionMarker supported; EditionMarkerV2 blocks with evidence reason                         |
| pNFT / programmable edition | BurnV1 plus derived TokenRecord                                                   | Unlocked record; locked/delegated states cannot bypass program rules                                |
| MPL Core AssetV1            | Core BurnV1, ownership and inherited plugin checks; burnt marker/absence verified | Frozen/unknown/external lifecycle hooks oracles unsupported; nonempty Asset Signer blocked          |
| Bubblegum v1                | Official Burn, fresh DAS proof and RPC tree/config validation                     | Network-verified DAS; exact leaf owner/delegate/index, full Merkle proof, canopy trimming           |
| Bubblegum v2                | Official BurnV2 with collection/data hashes and flags                             | Same proof checks, Core collection rules, frozen flags blocked                                      |

Core Asset Signer must not hold SOL, token accounts, nested Core or compressed NFTs.
Network-verified DAS coverage is required to check compressed holdings before
irreversibly disabling this PDA's Execute capability. Missing/partial coverage blocks
Core burn but does not erase Core discovery/category results. All NFT prepared
instruction sets are checked against Solana's 1232-byte packet limit; proof/account
sets needing address lookup tables are explicitly blocked rather than presented as
executable. Unknown future plugins/extensions are not silently ignored.

Selected print editions run before their selected master NFT. A master with unselected,
unowned or blocked prints is blocked with a precise reason. Mutable master supply
is checked before send; it does not invalidate the plan merely because approved
prints just burned. A failed print cannot cause its dependent master to be sent.

NFTs never pass through generic fungible BurnChecked. Metaplex already closes its
source account; no second Close is submitted. Core/cNFT success does not require
an SPL close. cNFT has no token-account rent to reclaim. DAS confirmation lag can
produce a confirmed signature with incomplete state verification; reconciliation
retains the signature and never resubmits it blindly.

Official references: [Metaplex burn](https://www.metaplex.com/docs/smart-contracts/token-metadata/burn),
[Core burn](https://www.metaplex.com/docs/smart-contracts/core/burn),
[Asset Signer](https://www.metaplex.com/docs/smart-contracts/core/execute-asset-signing),
[Bubblegum burn](https://www.metaplex.com/docs/smart-contracts/bubblegum-v2/burn-cnfts).

## Execution and accounting

The **desktop saved plan** is bound to wallet/session/genesis/revision, exact accounts, mint,
program, amounts, approved actions and expiry (120 seconds after planning).
Selection changes invalidate it. Public-key sessions cannot execute. Duplicate
IPC execute returns the existing job. Wallet/session changes cannot replace an
active signer. No mutex guard crosses an await.

CLI builds its core plan within one command, binds it to the public wallet,
selection and approved actions, and revalidates on-chain identities before send.
It does not create a Tauri session, server-owned plan ID or 120-second desktop
plan expiry. Fresh swap quote age/blockheight checks still apply.

Selected WSOL sources are unwrapped before swaps. Failed, protected or unresolved
WSOL sources block dependent swaps, so Jupiter cannot implicitly consume their
balance or invalidate a later approved close.

Sequential operations use signature-verified simulation and preflight. Swap/burn
must confirm and zero-check before close; close must confirm and verify absence.
Independent assets continue after local failures. In **desktop**, a durable
nonsecret journal records signatures before send and reconciles ambiguous
signature status/metadata before further execution. The CLI preserves uncertain
signatures in its report and does not automatically resend, but has no persistent
desktop journal or restart recovery. Check an uncertain signature before starting
a new CLI execution.

Final net SOL is the signed sum of this job's confirmed transaction wallet deltas,
including burn/close fees and confirmed failed-transaction fees. Do not add gross
reclaimed lamports to swap wallet deltas. Do not add burned token valuation. No
whole-wallet before/after subtraction substitutes for per-transaction receipts.
Missing metadata means incomplete accounting; blocked/failed assets produce partial
or failed results. Desktop progress carries actual core stages/counts, including
standalone NFT burns. The desktop refreshes inventory after completion; rerun a
read-only CLI scan to inspect the resulting state.

## Configuration and evidence

Rust reads inherited environment; `.env` is not auto-loaded. See
[`apps/app/backend.env.example`](../apps/app/backend.env.example). No credentials in
VITE variables, frontend bundle, DTOs, events, logs or persistent wallet storage.
DAS must support matching genesis, full owner pagination, getAsset and getAssetProof.
`DOCK_FLINTS_DAS_URL` is used by desktop analysis/cleanup and CLI cleanup. The
standalone CLI `scan --cnfts` retains its RPC-only Unsupported capability report;
it does not invoke DAS. See [cNFT discovery scope](cnfts.md).
Production ignores `DOCK_FLINTS_DEVNET_TEST_MANIFEST`; synthetic observations remain
only in tests. SCAM/DUST/DEAD do not authorize burning or determine eligibility.

See [current verification](current-verification.md) and the
[historical unified cleanup verification](unified-cleanup-verification.md).
Browser/mocked RPC tests and actual local-validator/native desktop checks are
reported separately. Live
mainnet swap/burn/close on the user's wallet is never part of development validation.
