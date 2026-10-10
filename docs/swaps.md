# Single-token swaps to native SOL

`dock_flints quote` and `dock_flints swap` use Jupiter's current [Swap V2 Router `/build` API](https://github.com/jup-ag/docs/blob/main/swap/build/index.mdx), which returns a quote and raw instructions together. This path uses Jupiter's Metis routing and supports submission through the application's existing Solana RPC client. It does not use the managed `/order` + `/execute` flow or implement its own DEX routing. [Official API schema](https://github.com/jup-ag/docs/blob/main/openapi-spec/swap/v2/swap.yaml).

## Commands

Preview only, using a public wallet address and integer token units:

```bash
cargo run -- quote \
  -p EibQ2VYpzj18qSdEBkmxWVzde7FzamTxVG9rZyY689Yj \
  --mint EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v \
  --raw-amount 1000000 --format json
```

For this USDC mint, `1000000` means 1 USDC. Amounts are always raw `u64` units, never UI floats. The public taker address is required by `/build`. Preview requires no private key, does not require holding the amount, and makes no transaction submission.

Execute one swap from a local Solana CLI JSON keypair:

```bash
cargo run -- swap \
  --keypair /path/to/wallet.json \
  --mint EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v \
  --raw-amount 1000000 \
  --slippage-bps 50 --max-price-impact-bps 100 \
  --rpc-url https://api.mainnet.solana.com
```

Every command accepts exactly one wallet identity: `--pubkey`, `--keypair` or `--seed`. Quotes can use all three; execution requires keypair or a base64 32-byte Ed25519 seed. Execution derives the wallet public key locally. It displays a preview and asks for `y`/`yes`; any other answer cancels. `--yes` explicitly approves the preview minimum for noninteractive use. With `--format json`, preview and confirmation prompts go to stderr, and stdout contains one final JSON result. Scanner flags remain separate and continue working as before.

The shared client uses `JUPITER_API_KEY` when set. The client can omit this header, but keyless endpoint availability is provider-controlled; configure a key for reliable mainnet access. Authentication/rate-limit errors remain explicit. Files such as `.env` are not loaded automatically. Existing Price V3 configuration remains unchanged. No price lookup is required to obtain a swap quote: an unpriced token may have a valid route.

## Execution guarantees

1. Preview validates the exact input mint, raw amount, WSOL output mint, ExactIn mode, slippage, nonzero output/minimum and finite price impact.
2. Before execution, a mint-filtered owner query reuses the existing legacy/Token-2022 decoders, mint/Metaplex scanner and classifier. Insufficient holdings, NFTs and unverified classifications stop execution.
3. After approval, the service always requests a **new** `/build` response. It never executes preview instructions. Fresh price impact must pass the configured limit, and fresh `min_out` must be at least the preview's approved minimum. Otherwise rerun to review a new preview.
4. Build instructions and address lookup tables are compiled into a versioned transaction. Setup, swap and WSOL cleanup order is preserved. The only signer and fee payer is the selected local keypair. No private key is sent to Jupiter or RPC.
5. The transaction is simulated with signature verification and a maximum compute limit, then recompiled/signed with a 20% compute margin. Priority fees are capped. Submission keeps RPC preflight enabled.
6. Quote age and last valid block height are checked before submission. Confirmation polling requires `confirmed` or stronger commitment, including for on-chain failures. A timeout/send ambiguity preserves the locally computed signature and does not rebuild/resubmit a new transaction automatically.

The request uses `wrapAndUnwrapSol=true`, `nativeDestinationAccount=<wallet>` and the WSOL output mint. Jupiter's cleanup unwraps the output to native SOL. This is swap-specific WSOL cleanup, not a general empty-account cleanup feature.

Quote output includes exact expected/minimum SOL, raw lamport strings, percent price impact, route labels and `route_exists: true`. A no-route result exits 2 and includes `route_exists: false` in JSON. Liquidity failures, invalid responses, excessive impact, stale quotes, simulation failures and RPC/on-chain failures are reported separately. A successful execution returns `status: confirmed`, signature, native output asset and the **fresh quote**. Quote amounts are estimates/bounds before network fees and account rent; the receipt does not claim a measured net wallet balance increase.

## Limits and scope

| Option / rule | Default |
| --- | --- |
| Slippage | 50 bps (0.5%) |
| Maximum absolute price impact | 100 bps (1%) |
| Maximum quote age, measured from request start | 30 seconds |
| Maximum priority fee | 1,000,000 lamports; `--max-priority-fee-lamports` |
| Confirmation timeout | 90 seconds; `--confirmation-timeout-seconds` |
| RPC request timeout | 30 seconds; `--timeout-seconds` |

These swaps target mainnet liquidity; use a mainnet RPC. The wallet needs native SOL for fees and any required account creation. Jupiter uses its default input token-account selection; the standalone swap command does not consolidate balances from auxiliary token accounts. Simulation can reject an amount even if aggregate wallet holdings are sufficient but the usable input account is short. Token-2022 routes depend on Jupiter/AMM support for that mint's extensions.

After an uncertain result, check the reported signature before retrying. No automatic new-transaction retry is performed. This standalone swap command does not perform batch liquidation, NFT liquidation, custom routing or unrelated account cleanup. The separate [cleanup command](cleanup.md) adds sequential account-scoped swaps, approved burns and empty-account closes.

## Validation (2026-10-01)

- Existing scan tests remain intact.
- Mock HTTP tests exercise real client requests, exact mint/amount/native-SOL parameters, shared authentication/keyless access, no-route errors and an unpriced token that still has a route.
- Application/RPC tests cover fresh-route selection, changed minimums, excessive impact, stale quotes, signer mismatch, input validation for both token programs, NFT exclusion, local versioned signing, lookup tables, WSOL cleanup, simulation/send/confirmation failures, priority-fee bounds and no duplicate submission.
- A live read-only USDC → SOL `/build` request for the public example wallet succeeded, including minimum output, compute pricing, lookup-table mapping and WSOL close instruction. No real swap transaction was signed or submitted during development.
