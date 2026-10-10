# Поточна перевірка та документаційні матеріали

Дата: **2026-10-10**. Код застосунку: commit
`e013ea9e8b93f3ffc1a90c0890d0ce72e5686e06`; інструкції та capture harnesses
оновлено у робочій копії. Це звіт про конкретний запуск, а не гарантія
незмінного балансу або доступності зовнішніх providers.

## Середовище й завершені перевірки

Linux, Node `v26.11.1`, npm `12.2.0`, Rust/Cargo `1.98.0`.

| Перевірка                           | Результат                                                                                               |
| ----------------------------------- | ------------------------------------------------------------------------------------------------------- |
| `npm run check` у `apps/app`        | TypeScript, ESLint, 68 unit/component tests у 7 files та Vite production build пройшли                  |
| `npm run format:check` у `apps/app` | Пройшло                                                                                                 |
| `npm run test:e2e` у `apps/app`     | Playwright: **99 passed**, 12.5 min; `.last-run.json` має `status: passed`                              |
| `cargo fmt --all --check`           | Пройшло                                                                                                 |
| Preview screenshot harness          | 12 PNG: шість екранів на 430 і 360 CSS px, DPR 2; numeric fields лише digits/`—`, horizontal overflow 0 |

Browser E2E перевіряють React із mocked/recorded IPC, включно з category counts,
details, partial/missing/zero, NFT fallback, deduplication, rescan/late responses,
cleanup contracts і phone layouts. Вони не є live wallet scan.
Rust registered-IPC tests і native live capture мають окремі джерела evidence.

Повторити базові перевірки:

```bash
# apps/app
npm run check
npm run format:check
npx playwright install chromium
npm run test:e2e

# Корінь репозиторію
cargo fmt --all --check
cargo test --workspace --locked
```

## Матеріали й відтворення

[Галерея screenshots](screenshots/README.md) містить підписані екрани та команди
повторного capture. [Preview manifest](screenshots/preview/manifest.json)
зберігає UTC timestamp, source revision, browser version, URL, viewport і DPR.

Design preview має видимий напис **DESIGN PREVIEW · NO TRANSACTIONS**.
Значення `42 / 24 / 53 / 23`, reclaim estimate й success у цих знімках —
sample data для демонстрації UI. Preview processing/success не свідчать про
виконання cleanup на тестовому гаманці.

## Межі висновків

- Документаційний devnet-гаманець використовується через public-key connection.
  Keypair/seed не входять у матеріали й не потрібні для відтворення scan.
- На devnet Jupiter mainnet valuation/risk/routing не використовуються.
  Відсутність evidence означає `—` із причиною, а не підтверджений zero або NoRoute.
- Без DAS compressed NFT coverage неповний. Знайдені classic/Core assets
  залишаються доступними; порожній їхній список не підтверджує total NFT=0.
- Native Linux/WebKitGTK capture не підтверджує роботу macOS/Windows installers.
  `bundle.active=false`: поточний build створює executable зі вбудованим frontend.
- Live signing, burn, swap і close не виконуються в цьому документаційному
  сценарії. Попередні transaction receipts мають окремі дати й позначені як
  [історичні звіти](README.md#історичні-звіти-й-fixtures).

Практичні інструкції: [початок роботи](getting-started.md),
[CLI guide](cli-guide.md), [troubleshooting](troubleshooting.md),
[desktop configuration та користування](../apps/app/README.md).
