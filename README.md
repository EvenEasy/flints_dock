# Flint’s Dock

Flint’s Dock аналізує Solana-гаманець і готує план повернення SOL із token accounts. Проєкт містить desktop-застосунок на React + Tauri 2 та Rust CLI. Обидва використовують спільне ядро для discovery, класифікації, swap, burn, close і перевірки результатів.

Scan і підготовка плану працюють у режимі читання. Виконання потребує локального signer та явного погодження показаних дій. Native SOL залишається в гаманці для комісій; підтримувані NFT обробляють окремі Metaplex/Core/Bubblegum adapters.

![Головний екран Flint’s Dock у design preview](docs/screenshots/preview/main.png)

*Design preview із демонстраційними значеннями. [Галерея](docs/screenshots/README.md) окремо позначає preview та реальний read-only devnet scan.*

## Швидкий старт

Потрібні Rust/Cargo та Node.js у діапазоні з [package.json](apps/app/package.json): `^22.22.2 || ^24.15.0 || >=26.0.0`. Для native desktop також потрібні системні залежності Tauri; подробиці — у [desktop README](apps/app/README.md).

Із кореня репозиторію запустіть desktop на devnet:

```bash
cd apps/app
npm ci
export DOCK_FLINTS_RPC_URL=https://api.devnet.solana.com
npm run desktop
```

У вікні **DOCK A WALLET** оберіть **Public key (read only)** та вставте тестову адресу:

```text
9FCR2PU1jZgCHyjWxzk2BNQHJxszAK24vBFiWmUyRNpv
```

Це public address локального `dev/demo-wallet.json`. Для документації достатньо public key; файл ключів не входить до матеріалів.

Read-only CLI scan із кореня репозиторію:

```bash
cargo run --locked -- scan \
  --pubkey 9FCR2PU1jZgCHyjWxzk2BNQHJxszAK24vBFiWmUyRNpv \
  --rpc-url https://api.devnet.solana.com \
  --all --all-tokens --no-prices --details --include-empty --format json
```

Для перегляду дизайну в браузері запустіть `npm run dev` у `apps/app` та відкрийте `http://127.0.0.1:1420/?preview=1`. Browser preview показує приклади екранів; реальне підключення гаманця й analysis працюють через Tauri IPC у desktop-вікні.

## Що показує аналіз

- SOL, legacy SPL/Token-2022 accounts, fungible holdings, Unknown assets і підтверджені classic/programmable/MPL Core NFT.
- Desktop-плитки **SCAM**, **NFT**, **DUST**, **DEAD TOKEN** використовують один category result для числа й списку деталей. Лічильник містить лише число або `—`; complete-zero дає `0`, partial із відомими items показує кількість знайденого, partial без items дає `—`. Причини й coverage відкриваються в деталях.
- Holdings агрегуються за mint + token program із збереженням усіх backing accounts. Повторені accounts/NFT IDs не додають активів до лічильників. Категорії можуть перетинатися.
- Невідома ціна залишається невідомою. SCAM потребує явного suspicious signal із джерелом; DUST — достовірної оцінки ненульового holding; DEAD TOKEN — підтвердженого, scoped NoRoute. Ці категорії самі не дозволяють burn.
- Jupiter pricing/risk/routing працюють лише для підтвердженої mainnet. На devnet можливі `—` із конкретною причиною. Помилка provider не прибирає вже знайдені on-chain assets.
- Desktop і cleanup можуть використовувати network-matched `DOCK_FLINTS_DAS_URL` для compressed NFT. Без DAS відомі classic/Core NFT залишаються видимими, але загальний NFT total може бути неповним. Standalone CLI `scan --cnfts` наразі не перелічує compressed inventory.

Rust читає успадковані environment variables; `.env` автоматично не завантажується. CLI передає RPC через `--rpc-url`; desktop — через `DOCK_FLINTS_RPC_URL`. [Backend configuration](apps/app/backend.env.example) описує Jupiter, DAS, dust threshold і signature journal. Credentials залишаються в backend, не у `VITE_*`.

## Документація

| Матеріал | Для чого |
| --- | --- |
| [Початок роботи](docs/getting-started.md) | Вибрати режим, запустити, просканувати devnet і прочитати результат |
| [CLI: команди та приклади](docs/cli-guide.md) | Scan selection, JSON, read-only cleanup preview, quote і execution |
| [Desktop: запуск і користування](apps/app/README.md) | Системні залежності, UI flow, налаштування й Tauri build |
| [Скріншоти](docs/screenshots/README.md) | Екрани з описами та походженням даних |
| [Поточна перевірка](docs/current-verification.md) | Команди, результати, реальний devnet evidence та обмеження |
| [Повний індекс](docs/README.md) | Архітектура, контракти, cleanup/NFT coverage та історичні звіти |

## Структура та перевірки

```text
crates/core/                Rust core, use cases, Solana/Jupiter adapters
apps/cli/                   CLI scan / quote / swap / cleanup
apps/app/src/               React UI
apps/app/src-tauri/         Tauri commands, sessions, plans, jobs, journal, DTOs
apps/app/frontend-contract/ typed TypeScript IPC boundary
docs/                      guides, reports, fixtures, screenshot materials
```

Із кореня:

```bash
cargo fmt --all --check
cargo check --workspace --locked
cargo test --workspace --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
```

У `apps/app`:

```bash
npm run check
npm run test:e2e
npm run desktop:build
```

E2E/fixtures, native IPC і live RPC перевіряються окремо; актуальні результати наведені у [звіті](docs/current-verification.md).
