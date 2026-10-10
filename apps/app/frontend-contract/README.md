# Local desktop IPC contract

Імпортуйте typed wrappers із `index.ts`. React передає user intent; Rust
валідує запит, читає chain, зберігає signer/plans і створює транзакції.
Контракт не є terminal JSON CLI. Exact balances, raw amounts, rent, fees і signed
deltas передаються decimal strings. USD valuations — approximate numbers.

## Registered commands

| Command             | Request                                                                                      | Response / призначення                                               |
| ------------------- | -------------------------------------------------------------------------------------------- | -------------------------------------------------------------------- |
| `connect_wallet`    | `{ request: { source } }`, один `publicKey` / `seed` / `keypairFile`                         | Session ID, public address, sourceKind, canSign                      |
| `disconnect_wallet` | Без request                                                                                  | Скидає inactive session; active job блокує disconnect                |
| `analyze_wallet`    | `{ request: { walletAddress, selection?, noPrices? }, progress? }`                           | `WalletAnalysis`: snapshot, categories, independent scanner statuses |
| `prepare_cleanup`   | `{ request: { sessionId, revision, selection, ignoredMints, ignoredAssetIds? }, progress? }` | Immutable Rust plan: ID, actions, expiry, limits, estimates          |
| `execute_cleanup`   | `{ request: { sessionId, planId, approval }, progress }`                                     | Existing або newly claimed job з progress/report                     |
| `get_cleanup_job`   | `{ request: { sessionId, jobId } }` або `{ request: { sessionId, planId } }`                 | Read-only recovery, без повторного send                              |

Транспорт використовує camelCase поля DTO; вкладений `CleanupReport` зберігає
core snake_case keys. Command rejections мають `AppError` envelope:
`{ code, message, details }`; scanner/provider failure може бути успішно
отриманим partial `WalletAnalysis`.

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

Це приклад read-only виклику в Tauri, а не записаний результат. RPC network
конфігурується у Rust process environment, не в request. Omitted/empty selection
вмикає semantic defaults; `allTokens` окремо opt-in. Wallet dialog явно вмикає
всі п’ять параметрів і prices за замовчуванням.

`ScanStatus` — tagged union `complete`, `partial`, `unsupported`, `failed`,
`skipped`; усі стани, крім `complete`, мають `reason`. `AssetList.items=null`
означає unavailable inventory; `[]` відрізняється від null і читається разом із
status. `hasUsableResults` говорить про наявні корисні scan data, а не про повну
успішність усіх провайдерів.

## Category result і numeric display

`WalletAnalysis.categories` містить `network` (genesis hash або `unknown`),
`dustThresholdUsd`, `providers` та `categories` з ключами **`scam`, `nft`, `dust`,
`dead_token`**. Кожен `AssetCategory` містить `items`, `count`, `status`,
`checkedAt`, а також `source`, `network`, `reason`, `coverage`. Items мають
стабільний `id`, backing `accounts`, evidence/risk і provider scope.

Дані після IPC проходять `normalizeWalletCategories` у
`src/features/assets/categoryPresentation.ts`. Воно дедуплікує IDs, об’єднує
accounts і узгоджує `count` із підтвердженим списком. Compatibility NFT fallback
працює лише за відсутності `categories.nft` і в одному місці збирає classic,
Core та compressed items. MainScreen і CategoryDialog читають той самий
нормалізований category result через `presentCategory`.

| Нормалізований category result                                             | Numeric field                                |
| -------------------------------------------------------------------------- | -------------------------------------------- |
| Complete + підтверджені items                                              | Число унікальних items                       |
| Complete + confirmed `count=0`, `items=[]`                                 | `0`                                          |
| Partial + підтверджені items                                               | Число знайдених items, без `+` або `Partial` |
| Partial без items                                                          | `—`                                          |
| Failed / Unsupported / Skipped без збережених items                        | `—`                                          |
| Missing result, початок scan, undefined/null count без підтверджених items | `—`                                          |
| Incomplete/failed result зі збереженими підтвердженими items               | Число знайдених items, coverage у деталях    |

Status, reason та coverage не є текстом лічильника. Partial number — lower bound.
Tooltip/accessible description і CategoryDialog пояснюють повноту. Known NFTs
не губляться через unavailable DAS, pricing або risk. Devnet не використовує
mainnet ціни, risk records або routes. Реальні DTO та результати плиток дивіться
в [актуальному звіті](../../../docs/current-verification.md).

## Cleanup selection та approval

Selection: `{mode:'all'}`, `{mode:'none'}` або
`{mode:'selected', mints?: string[], assetIds?: string[]}`. Порожній `selected`
означає **none**. React надсилає `assetIds` із canonical inventory; `mints`
залишаються підтриманими для сумісності. Mint selection розгортається в усі
backing accounts. Core/cNFT selection використовує asset IDs.

`ignoredMints` захищає всі accounts mint; `ignoredAssetIds` — відповідні targets.
Ignore сильніший за selection. Revision має строго зростати; його зміна
інвалідує старі plans. План прив’язано до session/wallet/network/revision і має
120-second lifetime після planning. Expired, stale або signer/network-mismatched
plans, read-only sessions, empty execution та відсутній action approval
відхиляються backend незалежно від UI.

`RECOVER SOL` на live cleanup screen погоджує показаний план і викликає
`executeCleanup`; wrapper передає approval для swap/burn/close. При required burn
UI показує irreversible notice. Category flags не є action approval.

## Progress, recovery, accounting

`progressChannel` створює Channel лише за native IPC runtime. Events містять
sessionId/jobId/sequence, stage, completed/total, operation/account/status.
Відкидайте foreign session/job та старі sequence. Lost execute reply
відновлюється через `getCleanupPlanJob`, потім read-only polling job; це не
причина повторно викликати execute.

Estimate — gross swap output + recoverable rent перед fees.
`report.known_net_wallet_lamports` — exact signed wallet delta за transaction
metadata, включно з fees та confirmed failed transactions.
`known_swap_net_lamports + known_reclaimed_lamports` не є final net total.
Перевіряйте `accounting_complete`, job status, operation receipts та unresolved
signatures перед твердженням про completed result.

Capabilities — local/main-window only. IPC не дозволяє вибирати provider URL/key,
інший signer або arbitrary transaction bytes. Backend config та operation limits:
[desktop README](../README.md), [integration limits](../BACKEND_GAPS.md).
