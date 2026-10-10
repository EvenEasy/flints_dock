# Desktop integration and limitations

Current desktop implements analysis, immutable planning, approved execution, job recovery, categories and optional compressed-NFT discovery. See [current verification](../../docs/current-verification.md) for actual checks rather than treating implementation as proof of every live provider scenario.

## Implemented boundaries

| Capability        | Backend / frontend                                                                                       |
| ----------------- | -------------------------------------------------------------------------------------------------------- |
| Identity          | `connect_wallet`, `disconnect_wallet`; public session DTO, Rust-only signer                              |
| Analysis          | `analyze_wallet`; snapshot, token accounts, classic/pNFT/Core, optional DAS, independent statuses        |
| Pricing           | Mainnet Jupiter Price V3 with bounded retry/cache/batching                                               |
| Categories        | `scam`, `nft`, `dust`, `dead_token`; core evidence/coverage and one frontend normalization               |
| Planning          | `prepare_cleanup`; immutable session/network/revision-bound plan with expiry/actions/reasons/estimates   |
| Execution         | `execute_cleanup`; stored-plan approval, fresh validation/quotes, simulation and sequential confirmation |
| NFT cleanup       | Standard-aware Metaplex/pNFT/Core/compressed adapters; unsupported targets remain visible                |
| Progress/recovery | Typed Channel; `get_cleanup_job` by job ID or plan ID                                                    |
| Accounting        | Confirmed receipts and exact signed per-transaction wallet deltas; missing metadata is incomplete        |
| Journal           | Durable nonsensitive signatures, uncertain-send reconciliation and interprocess file lock                |

All six IPC commands are registered in `src-tauri/src/lib.rs` and scoped to the local `main` webview. Frontend requests do not supply endpoints, credentials, arbitrary instructions or transaction bytes. See [IPC contract](frontend-contract/README.md).

## Provider and coverage limits

| Scenario                        | Behavior / action                                                                                                                                                                                                                  |
| ------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Verified non-mainnet RPC        | Jupiter price/risk/routing are Unsupported; inventory remains available. SCAM/DUST/DEAD show `—` without reliable evidence.                                                                                                        |
| Transient genesis error         | Up to three bounded attempts; exhausted verification is not verified Unsupported or NoRoute. Independent inventory survives. A current regression expects Failed where actual composed category status is Partial; see the report. |
| Missing DAS                     | Compressed coverage Unsupported; known classic/Core NFTs remain. Empty classic/Core does not prove total NFT=0.                                                                                                                    |
| Unavailable/mismatched DAS      | Failure/partial diagnostics; verified genesis mismatch is Unsupported. Use the same network as RPC.                                                                                                                                |
| Malformed DAS URL               | Startup rejects configuration. Correct the absolute HTTP(S) URL or remove the setting.                                                                                                                                             |
| Malformed Jupiter key           | Jupiter client is disabled with a concrete reason; RPC discovery remains active.                                                                                                                                                   |
| Auth/rate limit/outage          | Bounded retries and diagnostics; missing observations do not become zero/SCAM/NoRoute. Keyless availability depends on the provider.                                                                                               |
| Risk record lacks `audit.isSus` | Unknown; names, decimals, authorities or verification alone are insufficient.                                                                                                                                                      |
| Large inventories               | Sequential routing can make analysis/planning slow.                                                                                                                                                                                |

`noPrices=true` skips valuation and limits DUST coverage, but does not block NFT/routing. DEAD TOKEN requires explicit NoRoute in recorded provider/time/amount scope. Production analysis does not read `DOCK_FLINTS_DEVNET_TEST_MANIFEST`; synthetic observations are fixture-only.

## Product/runtime limits

- Local signer only: keypair file or raw 32-byte base64 seed. Browser/hardware wallets and Wallet Adapter are not integrated.
- File input accepts an absolute path; no native file chooser yet.
- Arbitrary NFT metadata media are not loaded into the webview. Remote artwork needs a separate backend policy/cache/proxy.
- Dismiss ignores late UI responses but does not cancel RPC; no cancellation IPC exists.
- Full plans/jobs/reports remain in memory. Journal survives restart but does not restore React sessions.
- **HANGAR** has no separate live swap screen; token-to-SOL is available through cleanup. **MISSIONS** has no backend data.
- Installer bundles are disabled. Linux native evidence does not establish macOS/Windows behavior.

Category membership alone does not authorize destruction. Planning/execution recheck identity, ownership, amount, program and eligibility. Provider errors and failed swaps do not authorize fallback burns. See [NFT/plugin/proof limits](../../docs/cleanup.md).

## Evidence

[Current report](../../docs/current-verification.md) and [gallery](../../docs/screenshots/README.md) distinguish real scans from mocks. Earlier [desktop](docs/desktop-verification.md), [devnet](../../docs/devnet-cleanup-verification.md) and [unified](../../docs/unified-cleanup-verification.md) reports are historical. Local validators, mocked browsers and live devnet do not establish mainnet Jupiter route availability.
