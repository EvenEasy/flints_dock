# Архітектура та підтримка

`dock_flints` — Cargo workspace зі спільним `dock-flints-core`, CLI та React/Tauri 2 desktop для читання Solana wallet, swap і послідовного cleanup. Внутрішні `core`, `app`, `infra` відокремлюють правила, сценарії та зовнішні інтеграції. React відповідає за presentation/navigation; Rust — за discovery, classification, signer, immutable plans, виконання й accounting. DI-контейнера, event bus і gRPC немає.

Інструкції для користувача: [початок роботи](getting-started.md), [desktop](../apps/app/README.md), [CLI](cli-guide.md). Фактично виконані поточні перевірки — у [current verification](current-verification.md), screenshots — у [галереї](screenshots/README.md).

## Історія архітектурного рефакторингу

Нижче збережено відповідність попередніх modules після workspace refactoring. Це пояснення структури, а не звіт про новий запуск перевірок.

| Було                                                   | Проблема                                         | Стало                                                                 |
| ------------------------------------------------------ | ------------------------------------------------ | --------------------------------------------------------------------- |
| `models/`, `classification/`, `portfolio/aggregate.rs` | Типи й чисті правила розкидані                   | `core/{asset,wallet,classification,amount}.rs`                        |
| `swap/mod.rs`                                          | Типи, use case і Jupiter `BuildResponse` змішані | `core/swap.rs`, `app/swap/`, DTO в `infra/jupiter/`                   |
| `portfolio/service.rs`                                 | RPC, HTTP factory, оркестрація та stderr         | `app/scan_wallet.rs`, `infra/solana/scan/inventory.rs`, `cli/scan.rs` |
| `cleanup/`, частина CLI                                | CLI сканував і відбирав accounts                 | `app/cleanup::{plan_wallet,plan,execute}`                             |
| `rpc/`, `scanner/`, `pricing/jupiter.rs`               | Перетин відповідальності й зайвий переекспорт    | `infra/solana/`, `infra/jupiter/`; переекспорт видалено               |
| `output/`, `main.rs`                                   | CLI-адаптер розкиданий                           | `cli/output/`, `cli::run`; main запускає runtime                      |
| Повторні `.find()` для кожного account                 | Квадратичні проходи mint/metadata списків        | Локальні `BTreeMap`/`BTreeSet` індекси                                |
| `solana-client`                                        | Невикористані TPU/QUIC/pubsub dependencies       | RPC-only client + API                                                 |

Усі прямі dependencies мають використання. Перехід на RPC-only прибрав зайві транспортні залежності; NFT adapters використовують офіційні Metaplex/Bubblegum SDK. Дві версії Pubkey потрібні через різні Solana/Metaplex SDK: конвертація за байтами залишається в adapter. Перевірені декодери не замінюємо власними заради меншої кількості dependencies.

## Де що змінювати

```text
crates/core/src/
  core/                 типи й чисті правила, без HTTP/RPC/CLI
    asset.rs            token accounts, mint, metadata, closure eligibility
    wallet.rs           WalletSnapshot, selection, partial scan statuses
    classification.rs   fungible/NFT classification
    inventory.rs        нормалізовані holdings, backing accounts і standalone IDs
    categories.rs       category items/count/status, risk/valuation/tradability evidence
    nft_cleanup.rs      стандарт-aware NFT targets та prepared operations
    amount.rs           точні суми й агрегація
    swap.rs             request, quote, limits, neutral transaction ingredients
    cleanup.rs          plan/results, eligibility, protected mint selection
    error.rs            semantic errors: NoRoute відмінний від API failure
  app/
    scan_wallet.rs      WalletReader + orchestration сканування
    categories.rs       independent risk/routing/NFT coverage та classification
    pricing.rs          PriceProvider + необов'язкова оцінка портфеля
    swap/               SwapProvider, SwapExecutor, preview/execute
    cleanup/            plan_wallet, build_plan, execute_plan, CleanupExecutor
  infra/
    wallet.rs           keypair/base64 seed → address + optional signer
    jupiter/            HTTP, auth, Price/Swap DTO → core types
    solana/             RPC, instructions, simulation, send/confirm, accounting
      scan/             token/mint/Metaplex/Core/DAS discovery
      nft_cleanup/      Metaplex/Core/Bubblegum preparation, validation і burn
apps/cli/src/cli/
    args.rs             спільні wallet/RPC/quote/execution аргументи
    scan.rs             scan dependency wiring та вивід
    swap.rs             quote/swap commands та підтвердження
    cleanup.rs          cleanup command та підтвердження
    output/             table/JSON presentation
apps/app/src-tauri/src/
  commands/identity.rs  connect/disconnect: public session і Rust-only signer
  commands/wallet.rs    analyze: network verification → snapshot → classify → DTO
  commands/cleanup.rs   prepare/execute/job: server plans, approval і progress
  cleanup.rs            durable public signature journal та reconciliation
  state.rs              shared RPC/Jupiter clients initialized once
  journal.rs            durable nonsensitive signatures і міжпроцесний file lock
  dto/                  camelCase IPC, exact integers as strings
  error.rs              stable frontend-safe errors
apps/app/frontend-contract/
                        TypeScript types and typed invoke boundary
apps/app/src/
  app/                  session/navigation guards, phone shell та display defaults
  features/wallet/      connect dialog, scan lifecycle, MainScreen
  features/assets/      inventory/details, category normalization/presenter
  features/cleanup/     selection, immutable plan, actual progress/report
  features/preview/     окремо позначені demonstration fixtures
  shared/               desktop transport, exact formatting, UI primitives
```

`core` не імпортує `app`, `infra`, `cli`, HTTP або RPC клієнтів. Solana public keys/instructions/blockhashes у core — дані цільового blockchain, а не transport. DTO Jupiter не виходять із adapter: base64/string fields перетворюються на `SwapTransaction` з native Solana instructions. Компіляція, signing і submission живуть у `infra/solana`.

`app` не імпортує adapters. Traits лише на зовнішніх межах: читання chain, ціни, побудова swap, виконання swap/cleanup. `infra` реалізує ці контракти, CLI з'єднує implementations з use cases. Dependency direction: CLI → APP → CORE; INFRA реалізує APP boundaries й використовує CORE. Немає зворотного імпорту APP → INFRA. Async traits використовують static dispatch; DI framework не потрібен.

## Типові зміни

- **Інший swap provider:** реалізувати `app::swap::SwapProvider`, перетворити відповідь на `core::swap::{SwapQuote, SwapTransaction}` в adapter і замінити factory `cli::args::swap_provider`. Quote/build зараз один виклик: Jupiter повертає їх разом; окремий quote-only trait не потрібен.
- **Політика cleanup:** `core/cleanup.rs`, `app/cleanup/plan.rs`, тести `apps/cli/tests/cleanup.rs`. Burn/no-route не визначаються в CLI чи renderer.
- **Спосіб підписання:** execution boundary і `infra/wallet.rs`. Snapshot/plan містять лише public address. Поточний MVP працює з локальним signer.
- **Формат виводу:** `cli/output/`, без blockchain-запитів.
- **CLI параметри:** спочатку перевірити `cli/args.rs`. Wallet/RPC/quote/execution конфігурація визначена один раз і згрупована в `--help`. Scan-specific display flags не потрапляють у swap/cleanup.
- **NFT/cNFT:** `infra/solana/nft_cleanup/` для стандарт-aware burn. DAS забезпечує owner inventory та актуальні proofs; без відповідного network-scoped endpoint compressed coverage чесно Unsupported. Див. [coverage](cleanup.md) та історичне дослідження `cnfts.md`.

## Контракти, які не можна послаблювати

Рівно один `--pubkey`, `--keypair` або `--seed`. Public key лише читає; інші можуть підписувати, але read-only команда нічого не відправляє. Seed — base64 від **рівно 32 Ed25519 bytes**, не mnemonic і не 64-byte keypair. Не логувати secret material і не додавати йому `Debug`/`Serialize`.

`--ignore-mint` сильніший за вибір account: захищає всі balances та empty accounts, без quote/burn/close. Selection зберігається в plan і перевіряється execution. Якщо захищений WSOL, swap до native SOL пропускається, оскільки provider міг би unwrap наявний WSOL account. Нові execution restrictions можуть додати пропуски; вони не скасовують збережені restrictions.

Burn дозволений лише у погодженому незмінному плані: для fungible після підтвердженого provider-specific `NoRoute` або структурної відсутності swap capability, з повторною перевіркою цієї підстави. NFT використовують власні стандарт-aware adapters. API/auth/timeout/liquidity errors не означають «мертвий токен». Після send з невідомим результатом зберігати signature, не перебудовувати економічну транзакцію. Перед close перечитати source і перевірити нульовий balance.

## Перевірки

```bash
cargo fmt --all --check
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

Тести використовують binary fixtures, mock HTTP/RPC, локально згенеровані test keys і провайдери. Перевіряють behavior: запити за категоріями, класифікацію, fresh quotes, signing/simulation/confirmation, помилки, mint exclusions та CLI identity. Архітектурний рефакторинг не потребує реальних swap, burn чи close.

## Desktop boundary

CLI і Tauri залежать від `dock-flints-core`; ядро не залежить від adapters. CLI має compatibility re-exports для попередніх library paths. Tauri викликає спільні scan/plan/execute use cases; signer і prepared instructions залишаються у Rust. Execute приймає лише ID збереженого плану та погодження дій. Public-key session не може виконувати транзакції. DTO не використовують terminal JSON або serialization внутрішніх моделей. Повні raw account blobs не є IPC-контрактом. Shared fixture лишається в `tests/fixtures/`.

Налаштування клієнтів, точні TypeScript types, capability для локального main window і кроки запуску React frontend і desktop описано в [apps/app/README.md](../apps/app/README.md). React відображає канонічний inventory, immutable plan, фактичний progress і confirmed report.

### Analysis: від запиту до category details

```text
ConnectWalletDialog → connect_wallet → public WalletConnection
  → AnalyzeWalletRequest (walletAddress, selection, noPrices)
  → shared/api/wallet.ts → frontend-contract/wallet.ts → analyze_wallet
  → bounded getGenesisHash verification
  → scan_wallet_observed → WalletSnapshot
  → optional network-scoped DAS → canonical inventory → categories::classify
  → WalletAnalysisDto → WalletAnalysis
  → useWalletAnalysis → normalizeWalletCategories
  → MainScreen і CategoryDialog → presentCategory
```

Звичайний wallet dialog явно вмикає balance, tokens, allTokens, nfts і cnfts; prices увімкнені за замовчуванням. On-chain inventory читається один раз, далі snapshot enrichments зберігають незалежні results. Category keys узгоджені через core/DTO/TypeScript: `scam`, `nft`, `dust`, `dead_token`.

Core inventory агрегує fungible holdings за mint/program із backing accounts та окремими NFT IDs. Frontend category normalization дедуплікує item IDs, об’єднує accounts і узгоджує count зі списком; compatibility NFT fallback classic/Core/compressed працює лише в одному місці за відсутності `categories.nft`. MainScreen не додає NFT counts незалежно від CategoryDialog.

Єдиний presenter відокремлює counter, status, reason і coverage. Numeric field — число або `—`; confirmed complete empty list — `0`. Неповний результат із items показує число реально знайдених унікальних активів, без `Partial`/`+`; incomplete empty result — `—`. Coverage і lower-bound explanation доступні в tooltip/description та деталях. Missing count не підміняється нулем, demonstration values не входять у live analysis.

Jupiter prices/risk/routes використовуються лише після verified mainnet genesis hash. Тимчасова невдача network verification повторюється bounded до трьох спроб і лишається Failed, а не підтверджено Unsupported. On-chain holdings залишаються доступними. Devnet не отримує mainnet observations; unsupported network не є NoRoute. NFT не залежить від pricing/risk, DUST не залежить від routing. DAS перевіряє network/owner/burned state і дедуплікує IDs; missing DAS не приховує classic/Core NFT, але не дозволяє стверджувати complete NFT=0.

`useWalletAnalysis` очищає попередній snapshot на початку rescan, перевіряє returned owner і відкидає stale request generations. `App` додатково відкидає dismissed results та guards live hash routes. Dismiss не є RPC cancellation: cancel IPC поки не існує.

### Cleanup: selection і recovery

React надсилає `assetIds` та `ignoredAssetIds` із canonical inventory. IPC також підтримує legacy `mints`/`ignoredMints`. `selected` без IDs означає none; `all` — окремий mode. Ignored mint захищає всі backing accounts, зокрема empty ones. Revision/session/network/expiry прив’язують plan до конкретного selection; public-key session може prepare, але не execute.

Registered commands: `connect_wallet`, `disconnect_wallet`, `analyze_wallet`, `prepare_cleanup`, `execute_cleanup`, `get_cleanup_job`. Lost execute response відновлюється read-only lookup за job ID або plan ID. Signature journal переживає restart; full session/plans/job reports поки зберігаються в пам’яті. Recovery не створює повторний economic send.

Реальні прогалини й provider configuration failures документуються в [BACKEND_GAPS.md](../apps/app/BACKEND_GAPS.md); [IPC README](../apps/app/frontend-contract/README.md) описує точні request/response semantics.
