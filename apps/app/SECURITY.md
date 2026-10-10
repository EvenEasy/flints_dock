# Desktop security boundaries

The local Tauri webview calls six registered commands: `connect_wallet`, `disconnect_wallet`, `analyze_wallet`, `prepare_cleanup`, `execute_cleanup`, `get_cleanup_job`. Capabilities scope these commands to the `main` window; real RPC/provider access stays in Rust.

## Identity and authorization

Public-key connection is read-only. File/seed connection creates a Rust-held signer; React receives public session data, not secret bytes. Seed input is cleared after submission. Connecting does not authorize a transaction.

`prepare_cleanup` creates an immutable wallet/session/network/revision-bound plan. `execute_cleanup` accepts that plan’s ID and approval; it does not accept frontend-authored instructions or transaction bytes. Rust checks expiry, selection, ownership, amounts, authorities, restrictions and fresh provider/proof data. The final **RECOVER SOL** approves the displayed actions, including explicitly explained irreversible burns.

Execution is sequential with simulation, preflight and confirmation. Uncertain submissions are reconciled rather than economically repeated. Failed swaps and temporary provider errors cannot authorize burn fallback. Category labels do not determine execution eligibility.

## Storage and recovery

Before send, the durable journal stores nonsensitive signature, owner, network, source account, operation and expiry. Interprocess locking coordinates instances. The journal does not contain a seed, keypair or serialized transaction. Full plans/reports remain in memory; restarting preserves reconciliation safety but not the old UI session.

The persistent journal is a desktop feature; standalone CLI returns signatures for reconciliation without that journal.

## Webview and providers

Local assets are allowlisted; wallet metadata cannot select arbitrary media URLs. Backend credentials stay in process environment, never `VITE_*`. The frontend does not choose RPC/provider endpoints. CSP and capability configuration should be reviewed when adding network/media features.

Pricing/risk/routing are network-scoped. Devnet does not receive mainnet observations. Missing evidence is not zero, suspicious status or NoRoute. Synthetic fixture observations are excluded from live analysis.

Native inspector settings in screenshot instructions are for local capture on loopback. Omit them from normal launches. See [current verification](../../docs/current-verification.md), [IPC contract](frontend-contract/README.md) and [integration limitations](BACKEND_GAPS.md).
