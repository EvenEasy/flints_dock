# Backend integration report

## Existing integration

The frontend calls the unmodified `analyzeWallet(request)` wrapper, which invokes `analyze_wallet` with `{ request }`. It consumes the existing `AnalyzeWalletRequest`, `WalletAnalysis`, `ScanStatus`, and `AppError` shapes. Category selection, optional prices, legacy/Token-2022 inventory, classic/Core NFTs, account closure assessments, exact amount strings, partial results and unsupported cNFTs are displayed. The backend remains authoritative for validation and classification.

There are no new command names registered, no changes to Tauri permissions, and no fabricated live responses. The only stubs are visible unavailable states and disabled mutation controls. Browser reference data never enters IPC.

## Needed before real cleanup

These are **proposed contract additions for a later backend task**, not APIs currently implemented by the frontend. Follow the existing camelCase DTOs, tagged status objects, exact decimal integer strings, optional fields, and structured error envelope. Current `AppError.code` supports only request/address/configuration errors; execution errors need an explicit extension or separate contract.

| Capability | Required request/response information | Current UI behavior |
| --- | --- | --- |
| TOKEN → SOL quote | Input `mint`, `tokenAccount`, `rawAmount` (string), backend network/context. Return route status, expected/min output lamports (strings), price impact, expiry/quote identity and error reason. Check Jupiter route, not Price API presence. | Shows NOT QUOTED and no live SOL estimate. |
| Cleanup planning | Request owner, selected account addresses/mints, ignored mint array and explicit permitted actions. Return stable plan ID, expiry, per-account classification/action/reason, NFT/unsupported exclusions, quote diagnostics, counts, estimate/rent/fee strings. | Displays inventory and unavailable plan; keep intent is local only. |
| Risk/dust/dead classification | Backend-owned criteria, route evidence, frozen/delegate/close-authority/Token-2022 restrictions and burnability assessment. | SCAM/DUST/DEAD counters remain unavailable; no burn decision from missing prices. |
| Simulation | Validated plan ID plus explicit account/action selection. Return per-account simulation status, failure reason, estimated fees; no state changes. | No simulation control pretending to work. |
| Signing and execution | Backend-managed signer/session; validated fresh plan/quotes and user approval. Re-check balance/owner/mint exclusions, handle expiry, sign/send/confirm, sequential per-account errors, unwrap WSOL, verify zero balance before closure. Private material stays outside frontend. | Live confirmation disabled; no keypair/seed fields or execution invoke. |
| Execution progress | Typed operation ID and confirmed per-account stages/errors, processed/total counters, cancellation semantics, final status. Backend event/channel API and narrow Tauri capability needed. | Live scan indeterminate; salvage stages only in labelled preview. |
| Confirmed report | Actual received SOL, reclaimed rent, transaction fees/net result as strings, per-account outcomes/signatures, skipped/protected assets and partial failure summary. | Live report unavailable; no recovered SOL claim. |

Ignoring must be enforced in the backend at **planning, simulation, fresh-quote, and execution** stages. Frontend switches are not authorization or protection. Symbols/names are only display labels; identity must use mint/account address.

A cleanup plan should distinguish `Empty`, `Swappable`, `Burnable`, and `Unsupported`, and include an explicit skip action for excluded mints. Quote failures/unavailable providers must remain distinct from a confirmed “no route” result, otherwise a provider outage could lead to an unsafe burn classification. Burning needs an explicit execution mode and per-plan approval; ownership and program restrictions require on-chain revalidation.

## Optional read-only additions

- cNFT enumeration requires the historical owner index already identified by the current contract. Keep its existing unsupported status until genuinely implemented.
- Real collectible artwork needs a backend image policy/proxy (allowed schemes/hosts, size/type limits, caching, private-network/SSRF protections). Existing metadata names and addresses are displayed safely; the webview never fetches arbitrary `imageUri`/`uri` values.
- Live scan stages or cancellation need an explicit progress/cancellation contract. Dismissing the current view discards late results but does not cancel the existing RPC command.

This frontend does not require changes to the underlying core for the existing read-only command. Future mutation contracts and capability permissions must be designed and implemented in a separate backend change.
