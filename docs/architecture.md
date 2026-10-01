# Архітектура та підтримка

`dock_flints` — бібліотечне ядро й CLI для читання Solana wallet, swap і послідовного cleanup. Один Rust crate, без DI-контейнера, event bus чи repository. gRPC у проєкті немає.

## Результат аудиту

| Було | Проблема | Стало |
| --- | --- | --- |
| `models/`, `classification/`, `portfolio/aggregate.rs` | Типи й чисті правила розкидані | `core/{asset,wallet,classification,amount}.rs` |
| `swap/mod.rs` | Типи, use case і Jupiter `BuildResponse` змішані | `core/swap.rs`, `app/swap/`, DTO в `infra/jupiter/` |
| `portfolio/service.rs` | RPC, HTTP factory, оркестрація та stderr | `app/scan_wallet.rs`, `infra/solana/scan/inventory.rs`, `cli/scan.rs` |
| `cleanup/`, частина CLI | CLI сканував і відбирав accounts | `app/cleanup::{plan_wallet,plan,execute}` |
| `rpc/`, `scanner/`, `pricing/jupiter.rs` | Перетин відповідальності й зайвий переекспорт | `infra/solana/`, `infra/jupiter/`; переекспорт видалено |
| `output/`, `main.rs` | CLI-адаптер розкиданий | `cli/output/`, `cli::run`; main запускає runtime |
| Повторні `.find()` для кожного account | Квадратичні проходи mint/metadata списків | Локальні `BTreeMap`/`BTreeSet` індекси |
| `solana-client` | Невикористані TPU/QUIC/pubsub dependencies | RPC-only client + API |

Усі прямі dependencies мають використання. RPC-only прибрав 127 записів із lockfile (654 → 527), без нових пакетів і оновлень версій. Дві версії Pubkey потрібні через різні Solana/Metaplex SDK: конвертація за байтами залишається в adapter. Перевірені декодери не замінюємо власними заради меншої кількості dependencies.

## Де що змінювати

```text
src/
  core/                 типи й чисті правила, без HTTP/RPC/CLI
    asset.rs            token accounts, mint, metadata, closure eligibility
    wallet.rs           WalletSnapshot, selection, partial scan statuses
    classification.rs   fungible/NFT classification
    amount.rs           точні суми й агрегація
    swap.rs             request, quote, limits, neutral transaction ingredients
    cleanup.rs          plan/results, eligibility, protected mint selection
    error.rs            semantic errors: NoRoute відмінний від API failure
  app/
    scan_wallet.rs      WalletReader + orchestration сканування
    pricing.rs          PriceProvider + необов'язкова оцінка портфеля
    swap/               SwapProvider, SwapExecutor, preview/execute
    cleanup/            plan_wallet, build_plan, execute_plan, CleanupExecutor
  infra/
    wallet.rs           keypair/base64 seed → address + optional signer
    jupiter/            HTTP, auth, Price/Swap DTO → core types
    solana/             RPC, instructions, simulation, send/confirm, accounting
      scan/             token/mint/Metaplex/Core decoders, inventory enrichment
  cli/
    args.rs             спільні wallet/RPC/quote/execution аргументи
    scan.rs             scan dependency wiring та вивід
    swap.rs             quote/swap commands та підтвердження
    cleanup.rs          cleanup command та підтвердження
    output/             table/JSON presentation
```

`core` не імпортує `app`, `infra`, `cli`, HTTP або RPC клієнтів. Solana public keys/instructions/blockhashes у core — дані цільового blockchain, а не transport. DTO Jupiter не виходять із adapter: base64/string fields перетворюються на `SwapTransaction` з native Solana instructions. Компіляція, signing і submission живуть у `infra/solana`.

`app` не імпортує adapters. Traits лише на зовнішніх межах: читання chain, ціни, побудова swap, виконання swap/cleanup. `infra` реалізує ці контракти, CLI з'єднує implementations з use cases. Dependency direction: CLI → APP → CORE; INFRA реалізує APP boundaries й використовує CORE. Немає зворотного імпорту APP → INFRA. Async traits використовують static dispatch; DI framework не потрібен.

## Типові зміни

- **Інший swap provider:** реалізувати `app::swap::SwapProvider`, перетворити відповідь на `core::swap::{SwapQuote, SwapTransaction}` в adapter і замінити factory `cli::args::swap_provider`. Quote/build зараз один виклик: Jupiter повертає їх разом; окремий quote-only trait не потрібен.
- **Політика cleanup:** `core/cleanup.rs`, `app/cleanup/plan.rs`, тести `tests/cleanup.rs`. Burn/no-route не визначаються в CLI чи renderer.
- **Спосіб підписання:** execution boundary і `infra/wallet.rs`. Snapshot/plan містять лише public address. Поточний MVP працює з локальним signer.
- **Формат виводу:** `cli/output/`, без blockchain-запитів.
- **CLI параметри:** спочатку перевірити `cli/args.rs`. Wallet/RPC/quote/execution конфігурація визначена один раз і згрупована в `--help`. Scan-specific display flags не потрапляють у swap/cleanup.
- **cNFT:** як і раніше, чесний `Unsupported`, а не порожній inventory. Повна підтримка потребує індексу; див. `cnfts.md`.

## Контракти, які не можна послаблювати

Рівно один `--pubkey`, `--keypair` або `--seed`. Public key лише читає; інші можуть підписувати, але read-only команда нічого не відправляє. Seed — base64 від **рівно 32 Ed25519 bytes**, не mnemonic і не 64-byte keypair. Не логувати secret material і не додавати йому `Debug`/`Serialize`.

`--ignore-mint` сильніший за вибір account: захищає всі balances та empty accounts, без quote/burn/close. Selection зберігається в plan і перевіряється execution. Якщо захищений WSOL, swap до native SOL пропускається, оскільки provider міг би unwrap наявний WSOL account. Нові execution restrictions можуть додати пропуски; вони не скасовують збережені restrictions.

Burn дозволений лише для явно схваленого cleanup, перевіреного fungible account і повторного семантичного `NoRoute`. API/auth/timeout/liquidity errors не означають «мертвий токен». Після send з невідомим результатом зберігати signature, не перебудовувати економічну транзакцію. Перед close перечитати source і перевірити нульовий balance.

## Перевірки

```bash
cargo fmt --check
cargo check --offline
cargo test --offline
cargo clippy --offline --all-targets --all-features -- -D warnings
```

Тести використовують binary fixtures, mock HTTP/RPC, локально згенеровані test keys і провайдери. Перевіряють behavior: запити за категоріями, класифікацію, fresh quotes, signing/simulation/confirmation, помилки, mint exclusions та CLI identity. Архітектурний рефакторинг не потребує реальних swap, burn чи close.
