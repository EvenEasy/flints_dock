# Local desktop IPC contract

Import typed wrappers from `index.ts`. React sends user intent; Rust validates requests, reads chain data, stores signers/plans and creates transactions. This contract differs from CLI terminal JSON. Exact balances, raw amounts, rent, fees and signed deltas are decimal strings; USD valuations are approximate numbers.

## Registered commands

| Command             | Request                                                                                      | Response / purpose                                   |
| ------------------- | -------------------------------------------------------------------------------------------- | ---------------------------------------------------- |
| `connect_wallet`    | `{ request: { source } }`; one `publicKey` / `seed` / `keypairFile`                          | Session ID, address, sourceKind, canSign             |
| `disconnect_wallet` | No request                                                                                   | Clear inactive session; active jobs block disconnect |
| `analyze_wallet`    | `{ request: { walletAddress, selection?, noPrices? }, progress? }`                           | Snapshot, categories, independent scanner statuses   |
| `prepare_cleanup`   | `{ request: { sessionId, revision, selection, ignoredMints, ignoredAssetIds? }, progress? }` | Immutable plan ID/actions/expiry/limits/estimates    |
| `execute_cleanup`   | `{ request: { sessionId, planId, approval }, progress }`                                     | Existing or newly claimed job with progress/report   |
| `get_cleanup_job`   | `{ request: { sessionId, jobId } }` or `{ request: { sessionId, planId } }`                  | Read-only recovery without another send              |

DTO fields use camelCase; nested core `CleanupReport` retains snake_case. Rejections use `{ code, message, details }` AppError envelopes. Provider failures can be represented within a successfully returned partial `WalletAnalysis`.

## Analysis selection

```ts
import { analyzeWallet } from './index';

const result = await analyzeWallet({
  walletAddress: '9FCR2PU1jZgCHyjWxzk2BNQHJxszAK24vBFiWmUyRNpv',
  selection: {
    balance: true,
    tokens: true,
    allTokens: true,
    nfts: true,
    cnfts: true,
  },
  noPrices: false,
});
```

This is a read-only Tauri call example, not a recorded result. RPC is configured in Rust environment, not the request. Omitted/empty selection uses semantic defaults; `allTokens` is separately opt-in. The wallet dialog explicitly enables all five fields and prices by default.

`ScanStatus` is a tagged union: complete, partial, unsupported, failed, skipped. Every non-complete state carries `reason`. `AssetList.items=null` means unavailable inventory; `[]` differs and must be interpreted with status. `hasUsableResults` is not proof that every provider succeeded.

## Categories and numeric display

`WalletAnalysis.categories` contains `network` (genesis hash or `unknown`), `dustThresholdUsd`, `providers`, and `categories` keyed by **scam, nft, dust, dead_token**. Each category contains `items`, `count`, `status`, `checkedAt`, source/network/reason/coverage. Items retain stable IDs, backing accounts, evidence and provider scope.

`normalizeWalletCategories` in `src/features/assets/categoryPresentation.ts` deduplicates IDs, merges accounts and aligns count with confirmed items. If the authoritative NFT category is absent, compatibility fallback normalizes classic/Core/compressed discovery in the same place. MainScreen and CategoryDialog consume that result through `presentCategory`.

| Normalized evidence                                                 | Numeric field                             |
| ------------------------------------------------------------------- | ----------------------------------------- |
| Complete with items                                                 | Unique item count                         |
| Complete confirmed `count=0`, `items=[]`                            | `0`                                       |
| Partial with items                                                  | Number found, without words or `+`        |
| Empty partial                                                       | `—`                                       |
| Failed / Unsupported / Skipped without retained reliable items      | `—`                                       |
| Missing result / scan start / missing count without confirmed items | `—`                                       |
| Incomplete result with retained confirmed items                     | Number found; coverage remains in details |

Status/reason/coverage are separate from numeric text. Partial counts are lower bounds. Tooltip/accessible description and details explain coverage. Missing DAS/pricing/risk does not discard known NFTs. Devnet does not use mainnet prices/risk/routes. See [real DTO/tile mappings](../../../docs/current-verification.md).

## Cleanup selection and approval

Selection is `{mode:'all'}`, `{mode:'none'}` or `{mode:'selected', mints?: string[], assetIds?: string[]}`. Empty selected means **none**. React uses canonical asset IDs; mint selection remains supported and expands to all backing accounts. Core/cNFT targets use asset IDs.

`ignoredMints` protects all accounts of a mint; `ignoredAssetIds` protects corresponding targets. Ignore takes precedence. Revision must increase; selection changes invalidate old plans. Plans are bound to session/wallet/network/revision and expire 120 seconds after planning. Backend rejects stale/expired/mismatched plans, read-only execution, empty execution and missing approvals independently of UI.

Final **RECOVER SOL** approves the displayed plan and calls `executeCleanup` with action approval. Required burns have an irreversible notice. Category flags are not action approvals.

## Progress, recovery and accounting

`progressChannel` creates a Channel only in native IPC. Events contain session/job/sequence, stage, completion counts, operation/account/status. Reject foreign IDs and older sequences. Recover lost execution replies with `getCleanupPlanJob` and read-only job polling, not another execute call.

Estimates are gross swap output plus recoverable rent before fees. `report.known_net_wallet_lamports` is the exact signed wallet delta from transaction metadata, including fees and confirmed failures. `known_swap_net_lamports + known_reclaimed_lamports` is not final net recovery. Check `accounting_complete`, job status, receipts and unresolved signatures.

Capabilities are local/main-window only. IPC does not accept provider URLs/keys, arbitrary signers or transaction bytes. See [desktop configuration](../README.md) and [integration limits](../BACKEND_GAPS.md).
