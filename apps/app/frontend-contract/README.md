# Local desktop IPC

Import wrappers from `index.ts`. React does not construct transactions or obtain
secrets from Rust. Exact balances, raw amounts, fees and signed deltas are decimal
strings. Status/count/valuation are separate: unknown coverage is not zero.

| Command             | Request                                                                            | Response / purpose                                                    |
| ------------------- | ---------------------------------------------------------------------------------- | --------------------------------------------------------------------- |
| `connect_wallet`    | One `source`: publicKey / seed / keypairFile                                       | Session ID, public address, sourceKind, canSign                       |
| `disconnect_wallet` | None                                                                               | Clears inactive session; active execution rejects disconnect          |
| `analyze_wallet`    | Public address, category selection, noPrices; optional Channel                     | Snapshot, categories, independent coverage statuses                   |
| `prepare_cleanup`   | sessionId, strictly increasing revision, selection, ignoredMints; optional Channel | Immutable Rust-owned planId, actions, limits, expiry, exact estimates |
| `execute_cleanup`   | sessionId, planId, explicit swap/burn/close approval; Channel                      | Existing or newly claimed job, confirmed report                       |
| `get_cleanup_job`   | sessionId and exactly one jobId or planId                                          | Read-only recovery; never resubmits                                   |

Selection is `all`, `selected` with exact mint addresses, or `none`. An empty
selected array is none. Selected mints expand to all backing token accounts,
including empty ones. Ignored mints override selection and protect all accounts.
Any selection revision invalidates older plans; late responses cannot replace a
newer UI selection. Execute rejects expired/session/network/signer-mismatched
plans, read-only wallets, missing action approval and zero eligible accounts.

Optional channels use `progressChannel` to bind callbacks only with a native IPC
runtime. Execution events carry sessionId/jobId/sequence, actual stage/counts,
operation/account and status. Ignore foreign sessions/jobs and older sequences.
A lost execute reply is recovered with `getCleanupPlanJob`, then read-only job
polling if still running. Never retry execute to recover an uncertain submission.

A plan estimate is gross swap output plus recoverable account rent before fees.
The final report’s `known_net_wallet_lamports` is the signed exact per-transaction
wallet delta including fees. `known_swap_net_lamports + known_reclaimed_lamports`
is **not** a net total. Check `accounting_complete`, job status, operation receipts
and unresolved signatures before describing a result as complete.

Capabilities are local/main-window only. Frontend requests cannot select endpoints,
provider keys, arbitrary instructions, transaction bytes or another signer.
Backend configuration and operational limitations are in `../README.md`.
