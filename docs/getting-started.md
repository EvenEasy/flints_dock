# Початок роботи

Flint’s Dock можна запустити як native desktop-застосунок, browser design preview або Rust CLI. Для реального UI scan використовуйте desktop; для автоматизації read-only inventory — CLI.

## Передумови

- Rust із підтримкою edition 2024 та Cargo для workspace. Toolchain version не закріплено окремим `rust-toolchain.toml`.
- Node.js із діапазону `apps/app/package.json`: `^22.22.2 || ^24.15.0 || >=26.0.0`, npm.
- Для desktop — системні build dependencies Tauri/GTK/WebKit на Linux. Команди встановлення й native build наведені в [desktop README](../apps/app/README.md).
- Доступ до вибраного Solana RPC. Provider auth/rate-limit errors відображаються в coverage і не підтверджують порожній гаманець.

Перевірити локальні версії:

```bash
rustc --version
cargo --version
node --version
npm --version
```

Команди нижче починаються з кореня репозиторію, якщо явно не вказано інше.

## Desktop: перший devnet scan

```bash
cd apps/app
npm ci
export DOCK_FLINTS_RPC_URL=https://api.devnet.solana.com
export DOCK_FLINTS_RPC_TIMEOUT_SECONDS=30
npm run desktop
```

Launcher запускає Vite та native Tauri-вікно. Перед цим зупиніть інший dev server на порту `1420`. `.env` автоматично не завантажується; backend отримує variables із shell, у якому запускається команда.

1. Натисніть **CONNECT WALLET** і оберіть **Public key (read only)**.
2. Вставте `9FCR2PU1jZgCHyjWxzk2BNQHJxszAK24vBFiWmUyRNpv`.
3. Залиште всі inventory selections увімкненими. Це запускає SOL, token accounts, NFT/Core і compressed coverage. Для scan без цін зніміть **Include Jupiter USD prices**; discovery залишається активним.
4. Натисніть **SCAN WALLET** та дочекайтеся завершення analysis. Progress показує фактичні backend stages.
5. Відкрийте **SCAM**, **NFT**, **DUST** або **DEAD TOKEN**, щоб побачити items, evidence, причину й coverage.

Адреса відповідає локальному тестовому файлу `dev/demo-wallet.json`. Файл не потрібний для цього сценарію. Якщо встановлено Solana CLI, public address можна перевірити без виводу ключів:

```bash
solana-keygen pubkey dev/demo-wallet.json
```

Для поточного документаційного сценарію використовується лише public-key connection і read-only scan/preview. [Актуальний devnet звіт](current-verification.md) містить зафіксовані результати; новий scan може повернути інші balances/items.

![Головний екран read-only devnet analysis](screenshots/devnet/main.png)

*Реальний devnet scan. Походження, час знімання й решта екранів наведені у [галереї](screenshots/README.md).*

## Як читати лічильники

Числа описують активи, а не кількість token accounts. Одна fungible позиція за mint + token program може мати кілька backing accounts. NFT з повтореним ID враховується один раз. Один актив може відповідати кільком категоріям.

| Лічильник | Значення |
| --- | --- |
| `0` | Перевірка complete і підтверджено порожній список |
| Додатне число | Кількість підтверджених унікальних assets у category result |
| Додатне число при partial | Уже знайдені assets; загальний підсумок ще може бути більшим |
| `—` | Немає достовірного count/items: початковий стан, missing, failed, skipped, unsupported або порожній partial |

Статусні слова не заміняють numeric field. Пояснення доступні в tooltip/accessible description плитки й у category dialog. Rescan очищає попередній result; запізніла відповідь попереднього scan не переносить його числа в новий гаманець.

На devnet Jupiter mainnet pricing/risk/routes не використовуються. Тому SCAM/DUST/DEAD TOKEN можуть показувати `—` з відповідною причиною. Відсутність ціни не дорівнює нульовій ціні, а відсутній маршрутний provider не доводить NoRoute.

NFT discovery працює незалежно від pricing/risk. Якщо DAS не налаштовано, знайдені classic/Core NFT відображаються, а compressed coverage лишається неповним. Порожній classic/Core список без compressed coverage не доводить загального `NFT = 0`.

## Cleanup preview

**RECOVER SOL** із головного меню відкриває cleanup flow. Перегляньте canonical inventory, оберіть активи й прочитайте створений Rust план із Swap/Burn/Close/Skip та reasons. Checked asset включено в план; unchecked asset залишається в гаманці.

Public-key session може підготувати план, але не може його виконати. При signer connection фінальний **RECOVER SOL** погоджує й виконує показаний saved plan, включно з irreversible burns. Підключення саме по собі не дозволяє транзакцій. Детальні правила, expiry, fees і accounting — у [desktop README](../apps/app/README.md) та [cleanup](cleanup.md).

## Browser preview і скріншоти дизайну

У `apps/app`:

```bash
npm run dev
```

Відкрийте `http://127.0.0.1:1420/?preview=1`. Окремі екрани welcome, scanning, main, cleanup і success доступні через preview menu та hash URLs. Для processing відкрийте `http://127.0.0.1:1420/?preview=1#processing`.

Preview scanning — статичний приклад, який не запускає RPC й автоматично не переходить далі. Preview cleanup переходить одразу до sample success; processing переглядається окремо. Значення й progress тут демонстраційні; це матеріали дизайну, а не результат devnet scan.

Для перевірки компактного layout:

```bash
npm run dev -- --width=390 --no-backdrop
```

Launcher підтримує `--width=360…480` і `--backdrop`/`--no-backdrop`; default width — `430` CSS px. [Галерея](screenshots/README.md) містить знімки й provenance, а [desktop README](../apps/app/README.md) — display options для native-вікна.

## CLI та build

Із кореня перевірте CLI help і запустіть read-only token inventory:

```bash
cargo run --locked -- scan --help
cargo run --locked -- scan \
  --pubkey 9FCR2PU1jZgCHyjWxzk2BNQHJxszAK24vBFiWmUyRNpv \
  --rpc-url https://api.devnet.solana.com \
  --tokens --all-tokens --no-prices --details --include-empty
```

Для готових binary/frontend builds:

```bash
# Корінь репозиторію
cargo build --release --locked -p dock_flints

# apps/app
npm run build
npm run desktop:build
```

CLI binary — `target/release/dock_flints`; desktop артефакти генерує Tauri build. Використовуйте `npm run desktop:build`, щоб frontend `dist` потрапив у native build. Далі: [CLI guide](cli-guide.md), [desktop operation/configuration](../apps/app/README.md), [поточні результати перевірок](current-verification.md).
