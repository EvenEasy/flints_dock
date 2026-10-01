# Wallet cleanup

Cleanup works per token account, identified by **account address and mint**, rather than aggregated token balances or names. It reuses the existing two-program scanner, mint/Metaplex classifier, closure assessment, Jupiter client and Solana RPC client.

## Preview and execution

Preview is the default and does not read a keypair, sign or submit transactions:

```bash
cargo run -- cleanup -p <WALLET> --dry-run
cargo run -- cleanup -p <WALLET> --format json
```

Execute the displayed plan with the wallet's local Solana JSON keypair:

```bash
cargo run -- cleanup -p <WALLET> --execute --keypair /path/to/wallet.json
```

The CLI shows the complete action list and asks you to type `cleanup`, explicitly including irreversible burns. For unattended execution, `--execute --keypair ... --yes` explicitly approves that displayed plan. `--dry-run` conflicts with `--execute`; passing `--keypair` or `--yes` without `--execute` is rejected.

Use repeatable `--account <TOKEN_ACCOUNT>` to restrict the plan to selected accounts. An address not discovered for the wallet is an error. `--format json` keeps stdout machine-readable; execution previews/prompts go to stderr. `--rpc-url`, request/confirmation timeouts, slippage, price-impact and priority-fee limits remain configurable. `--verbose` enables scanner diagnostics.

Quote requests run sequentially with a default 2,100 ms delay for keyless Jupiter access. Set `JUPITER_API_KEY` and adjust `--quote-interval-ms` for the rate limits of your account. The API client does not load `.env` automatically. Full-wallet planning may take time because each nonempty eligible account needs a genuine full-amount route check.

## Planning policy

| Category | Planned actions |
| --- | --- |
| Empty | Close an eligible zero-balance account to the owner wallet |
| Swappable | Swap its full raw balance to native SOL; confirm; verify this source is empty; close |
| Burnable | Explicit no-route result and eligible fungible account: burn full raw balance with correct decimals; confirm; verify zero; close |
| Unsupported | Skip, with a concrete reason |

`Swappable` estimates come from Jupiter Swap V2 `/build`, including expected output, minimum output and price impact. Price V3 is never used as a tradability gate. Authentication errors, 429s, timeouts, missing/malformed responses, excessive price impact, expired quotes or insufficient liquidity do **not** authorize burn.

Only positively classified fungibles are actionable. Classic/programmable NFTs, unverified assets, Core and cNFTs are not liquidated. Unknown assets remain visible as skipped. The current Token-2022 support allows base token accounts/ImmutableOwner and mint metadata, group metadata and mint-close-authority extensions. Withheld fees, confidential balances, transfer hooks, pausing and other unimplemented extension semantics remain Unsupported. This is intentionally conservative; no blanket claim of supporting every Token-2022 extension is made.

The wallet must be both token owner and the effective close authority. A frozen nonempty account is skipped. A frozen empty account can be closed when all other checks pass, consistent with [SPL CloseAccount semantics](https://solana.com/docs/tokens/basics/close-account). Nonempty WSOL is not burned; it requires explicit unwrap handling. [BurnChecked](https://solana.com/docs/tokens/basics/burn-tokens) verifies the mint decimals and destroys token units; closing returns lamports, not the value of burned tokens.

The plan preserves individual backing accounts, including several accounts of the same mint. Scan failures/undecodable accounts remain explicit. Partial inventory is labelled incomplete, and undiscovered accounts are never inferred to be empty.

## Execution and account changes

Each account is processed sequentially. Every transaction uses the same signature-verified simulation, enabled preflight, local signing and confirmed-status polling as the existing single-swap flow. Burn and CloseAccount are distinct confirmed transactions so the application can re-read and verify the zero balance between them.

Before each action the source is reloaded and its owner, program, mint, decimals, balance, authority and supported state are checked. A balance change from the approved plan stops that account; newly arrived tokens are not silently added to a burn or swap. CloseAccount is only constructed for zero balance and always pays the owner wallet. A post-close read checks account absence.

Before a swap, obtain a **fresh** route and recheck impact and the preview's approved minimum. Stale/API quote failures get bounded requotes (two attempts by default). A simulation failure or pre-send expiry can trigger one fresh rebuild. There is no swap-to-burn fallback during execution. A planned burn gets a second route check: a newly available route or an inconclusive response stops that burn and requires a new preview.

Jupiter normally spends from the input ATA. For an auxiliary source account, cleanup prepends an exact `TransferChecked` into that ATA to the **same swap transaction**. Existing ATA holdings are not added to `inAmount`. If a staging ATA must be created, create-idempotent and its final close are included in that atomic transaction. The original selected source is checked for zero and closed separately after confirmation. No standalone transfer can strand tokens if the swap fails. Extra instructions can exceed Solana's packet/compute limits for a complex route; such accounts fail explicitly instead of being burned.

Jupiter retains its native-SOL destination and WSOL unwrap cleanup. This may also unwrap an existing output WSOL account as part of the normal Jupiter flow; the reported swap wallet change is the observed net transaction effect.

An account failure does not stop unrelated accounts. Burn/swap receipts are retained if the subsequent close fails. An ambiguous submission/confirmation keeps its locally computed signature, is never automatically resent, and blocks further actions for that mint in the run; other mints continue. Check unresolved signatures before starting another execution. Confirmed on-chain failures and missing accounting data do not turn into fabricated successful outcomes.

## Results and accounting

Preview shows counts, all swap/burn/close/skip actions, quote estimates and actual account lamports potentially recoverable. Those lamports are not a fixed rent constant. Estimates exclude fees.

Execution reports each account's status, confirmed operations and signatures, failed/skipped accounts and:

- `known_swap_net_lamports`: the sum of observed owner-wallet post/pre balance changes for successful swap transactions, including their fees/rent effects; it is **not** a gross swap quote or total wallet P&L.
- `known_reclaimed_lamports`: actual source-account pre-balance from confirmed standalone close transactions, provided the recorded post-balance is zero. Close transaction fees are separate wallet costs.
- `accounting_complete`: false when relevant transaction metadata is missing or a submission/failure prevents complete accounting. Known subtotals remain available; unavailable values are never manufactured from preview estimates.

Amounts are serialized as decimal strings. JSON includes all account results and partial operation receipts. Exit 2 indicates execution failures/uncertainty or failed discovery; unsupported/skipped rows remain explicit even in otherwise successful runs. Cancellation and dry-run submit nothing.

## Verification

Regression tests cover category selection, no-route vs API/liquidity failures, authority/extension/frozen/NFT exclusions, explicit execution flags, fresh quotes, changed balances, zero checks, sequential processing, partial failures, unresolved signatures, same-mint accounts, exact burn/transfer decimals and amounts, atomic auxiliary-account assembly, confirmed-metadata accounting and RPC simulation/send/confirmation for burn/close. Existing scanner and swap tests remain in place.

A live **dry-run only** was performed against public wallet `EibQ2VYpzj18qSdEBkmxWVzde7FzamTxVG9rZyY689Yj`, restricted to an empty legacy account and its USDC account. The plan contained one Empty and one Swappable entry, two potential closes and 4,078,561 potentially reclaimed lamports. Jupiter returned a real USDC/SOL quote. No real swap, burn or close transaction was signed or submitted during development. Execution coverage uses mock RPC transports; no validator-backed transaction success is claimed.

Parallel execution, multi-swap transaction batching, NFT/cNFT cleanup and custom DEX routing are outside this implementation.
