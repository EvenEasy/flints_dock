# Desktop integration: реалізоване й обмеження

Цей файл описує поточні можливості й реальні прогалини. Контракт включає
analysis, immutable planning, approved execution, job recovery, категорії
та compressed NFT discovery.

## Що реалізовано

| Можливість          | Поточний backend / frontend                                                                                            |
| ------------------- | ---------------------------------------------------------------------------------------------------------------------- |
| Identity            | `connect_wallet`, `disconnect_wallet`; public session DTO, signer у Rust                                               |
| Analysis            | `analyze_wallet`; snapshot, token accounts, classic/pNFT/Core, optional DAS, independent scanner statuses              |
| Pricing             | Jupiter Price V3 із bounded retry/cache/batching; snapshot valuation лише verified mainnet                             |
| Категорії           | `scam`, `nft`, `dust`, `dead_token`; count/items/evidence/coverage із core, одна frontend normalization                |
| Planning            | `prepare_cleanup`; session/network/revision-bound план із expiresAt, actions, reasons та estimates                     |
| Execution           | `execute_cleanup`; approval збереженого плану, fresh validation/quotes, simulation, preflight, sequential confirmation |
| NFT cleanup         | Стандарт-aware Metaplex/pNFT/Core/compressed adapters; unsupported targets лишаються видимими                          |
| Progress / recovery | Typed Tauri Channel; `get_cleanup_job` читає job за job ID або plan ID                                                 |
| Accounting          | Підтверджені operation receipts і exact signed per-transaction wallet deltas; incomplete metadata позначається окремо  |
| Durable journal     | Nonsensitive signatures і reconcile uncertain sends, міжпроцесний file lock                                            |

Усі шість IPC commands зареєстровано в `src-tauri/src/lib.rs` і дозволено лише
локальному `main` webview. Frontend не передає endpoints, provider keys, arbitrary
instructions або transaction bytes. Деталі: [IPC contract](frontend-contract/README.md).

## Provider та coverage обмеження

| Сценарій                                  | Поточна поведінка / дія користувача                                                                                                                          |
| ----------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Devnet / інша verified non-mainnet мережа | Jupiter prices, risk та routing — Unsupported; assets зберігаються. Без надійних observations плитки SCAM/DUST/DEAD TOKEN показують `—`.                     |
| Тимчасова помилка `getGenesisHash`        | Analysis виконує до 3 спроб з timeout/backoff. Якщо network невідомий, scoped checks — Failed, а не достовірно Unsupported; on-chain inventory лишається.    |
| DAS не налаштовано                        | Compressed coverage — Unsupported; classic/Core inventory й відомі NFT зберігаються. Порожні classic/Core без compressed coverage не означають повний NFT=0. |
| DAS не працює або network не збігається   | Failed/Partial diagnostics у NFT coverage; genesis mismatch дає Unsupported. DAS має працювати на тій самій мережі, що RPC.                                  |
| Malformed `DOCK_FLINTS_DAS_URL`           | **Startup відхиляється** з configuration error. Виправте HTTP(S) URL або приберіть змінну; це відрізняється від відсутнього необов’язкового DAS.             |
| Неправильний формат `JUPITER_API_KEY`     | Jupiter client вимикається з конкретною помилкою; RPC discovery не вимикається.                                                                              |
| Provider auth/rate limit/outage           | Bounded retries та statuses; відсутні ціна/risk/route не підміняються zero/SCAM/NoRoute. Доступність keyless endpoint визначає провайдер.                    |
| Risk record без explicit `audit.isSus`    | Unknown observation. Назва, decimals, authorities чи verification самі по собі не доводять SCAM.                                                             |
| Великі inventories                        | Routing checks послідовні; analysis/planning можуть тривати довше за невеликий wallet.                                                                       |

`noPrices=true` пропускає valuation; це обмежує DUST coverage, але не блокує NFT
або routing. Missing price не перетворюється на нуль. DEAD TOKEN вимагає explicit
NoRoute у записаному provider/time/amount scope. Synthetic observations існують
для fixture tests; production analysis не читає
`DOCK_FLINTS_DEVNET_TEST_MANIFEST`.

## Product і runtime межі

- Локальний signer — keypair file або raw 32-byte base64 seed. Browser wallets,
  hardware wallets та Wallet Adapter поки не інтегровані.
- Keypair file вводиться через absolute path; native file chooser не реалізовано.
- Arbitrary NFT `imageUri`/`uri` не завантажуються у webview. Для реального artwork
  потрібен окремий backend image policy/cache/proxy.
- Dismiss scan ігнорує пізню UI відповідь, але не скасовує backend RPC. Окремої
  cancellation-команди немає.
- Plans і повні jobs/reports живуть у пам’яті. Signature journal переживає restart
  та захищає від повторного uncertain send, але не відновлює старий React session.
- `HANGAR` не має окремого live swap screen; TOKEN → SOL доступний через cleanup.
  `MISSIONS` не має backend даних.
- Збірка desktop executable не створює installer: `bundle.active=false`. Linux
  native evidence не замінює перевірки macOS/Windows.

Category membership сама по собі не дозволяє burn. План й execution повторно
перевіряють identity, ownership, amount, program та придатність операції.
Provider failure не дозволяє fallback burn; failed swap не перетворюється на burn.
Окремі unsupported NFT plugins/extensions/proofs описано в
[cleanup coverage](../../docs/cleanup.md).

## Evidence

Актуальні команди, screenshots і фактично виконані перевірки:
[current verification](../../docs/current-verification.md),
[галерея](../../docs/screenshots/README.md).
Попередні native/devnet/local-validator результати зберігаються як історичні
матеріали в [desktop verification](docs/desktop-verification.md),
[devnet verification](../../docs/devnet-cleanup-verification.md) та
[unified verification](../../docs/unified-cleanup-verification.md).
Mocked browser E2E, local validator й live devnet scan мають різні джерела даних
і не підтверджують доступність mainnet Jupiter routes.
