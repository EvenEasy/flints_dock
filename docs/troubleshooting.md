# Якщо запуск або scan не вдався

Спочатку перевірте режим запуску й RPC. [Початок роботи](getting-started.md)
містить перевірений devnet walkthrough, а [desktop README](../apps/app/README.md)
— системні передумови й конфігурацію.

## У браузері не підключається гаманець

`npm run dev` запускає frontend у браузері. Реальні wallet commands потребують
Tauri IPC: запустіть `npm run desktop` у `apps/app`. Для перегляду sample screens
у браузері відкрийте `http://127.0.0.1:1420/?preview=1#welcome`.

Preview scanning статичний. Перейдіть до іншого екрана через preview menu або
hash URL. Він не очікує RPC response й не є завислим реальним scan.

## Port 1420 уже зайнятий

Desktop launcher сам запускає Vite. Зупиніть окремий `npm run dev` у терміналі,
де його було запущено, і повторіть `npm run desktop`. Browser production preview
використовує port `1421`.

## Native build не знаходить GTK/WebKit або linker

Звірте встановлені build dependencies з
[Tauri prerequisites у desktop README](../apps/app/README.md#системні-передумови).
На Linux перевірте, що development packages видимі через `pkg-config`:

```bash
pkg-config --modversion gtk+-3.0 webkit2gtk-4.1
rustc --version
node --version
```

Для Rust workspace потрібна підтримка edition 2024. Node має відповідати
`engines` у [package.json](../apps/app/package.json). `npm ci` встановлює
dependency versions із lockfile.

Якщо збірка завершується через нестачу диска або пам’яті, перевірте вільне місце
й повторіть із меншою кількістю compiler jobs:

```bash
# apps/app
CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0 npm run desktop:build
```

## App відкривається зі старим frontend

Збирайте через `npm run desktop:build` у `apps/app`. Launcher додає
`tauri.frontend.conf.json`, яка вбудовує актуальний `dist`. Базова Rust config
сама по собі не є командою збірки поточного React desktop.

На Linux стандартний executable — `target/release/dock-flints-app` від кореня
репозиторію. Якщо задано `CARGO_TARGET_DIR`, шукайте його у відповідному target
directory. Installer packages поки вимкнено через `bundle.active=false`.

## Зміна `.env` не змінює network або provider

Rust не завантажує `.env` автоматично. Експортуйте variables перед запуском
і перезапустіть застосунок:

```bash
# apps/app
export DOCK_FLINTS_RPC_URL=https://api.devnet.solana.com
export DOCK_FLINTS_RPC_TIMEOUT_SECONDS=30
npm run desktop
```

Для CLI задайте `--rpc-url`: `DOCK_FLINTS_RPC_URL` налаштовує desktop.
Provider credentials мають залишатися у backend environment; `VITE_*`
призначені для публічних frontend settings.

## Плитка показує `—`

Відкрийте категорію й прочитайте status, reason та coverage. Прочерк означає,
що scan не має достовірного числа. Він може супроводжувати unsupported network,
disabled check, missing provider, auth/rate-limit error або порожній partial.
Complete й підтверджено порожній список відображає `0`.

На devnet SCAM, DUST і DEAD TOKEN не використовують mainnet Jupiter evidence.
Без відповідних devnet observations їхній `—` очікуваний. Не змінюйте RPC на
mainnet заради ненульових показників тестового devnet-гаманця.

NFT discovery незалежний від pricing/risk. Без DAS знайдені classic/Core NFT
залишаються доступними, але compressed coverage неповний. Нуль classic/Core
при цьому не підтверджує загальний `NFT = 0`.

## RPC, auth або rate limit

Перевірте, що RPC доступний і відповідає потрібній мережі. Analysis має bounded
retry для тимчасових genesis/provider errors. Якщо перевірка network не
завершилася, категорії з network scope отримують причину failure; це не
підтверджена unsupported network або NoRoute.

Після відновлення provider повторіть scan через **CHANGE WALLET / RESCAN**.
Якщо весь analysis відхилено, екран **ANALYSIS INTERRUPTED** має **RETRY SCAN**.
Partial results і вже знайдені незалежними scanners assets перегляньте в
деталях. [Поточні integration limits](../apps/app/BACKEND_GAPS.md) описують
діючі provider restrictions.

## DAS configuration не працює

`DOCK_FLINTS_DAS_URL` має бути absolute HTTP(S) URL провайдера тієї самої мережі.
Desktop перевіряє genesis hash. Невідповідна мережа дає Unsupported; недоступний
endpoint має reason failure. Malformed URL може відхилити startup: виправте
значення або приберіть optional setting і перезапустіть app.

Standalone CLI `scan --cnfts` наразі не перелічує compressed inventory навіть
при наявності DAS environment. Для цього використовуйте desktop analysis;
детальні вимоги — у [cNFT guide](cnfts.md).

## RECOVER SOL недоступний для виконання

Public-key connection показує **READ ONLY**. Він дозволяє scan і підготовку
плану, але не підписує транзакції. Для документаційного devnet walkthrough
використовуйте саме цей режим. Signer connection описаний окремо в
[desktop README](../apps/app/README.md#cleanup-для-signer-session).

План прив’язаний до wallet session/network і має строк дії. Після зміни
selection підготуйте новий план. Active cleanup job блокує wallet change;
recovery читає існуючий job. Signature journal зберігає reconciliation data,
але restart поки не відновлює всю попередню UI session.

## Browser E2E не знаходить Chromium

У `apps/app` встановіть browser, який використовує Playwright, і повторіть
перевірку:

```bash
npx playwright install chromium
npm run test:e2e
```

Якщо browser раніше встановлювали з `PLAYWRIGHT_BROWSERS_PATH`, використовуйте
те саме значення при запуску tests. Browser regressions перевіряють React із
mocked/recorded IPC; live desktop scan та native screenshots документуються
окремо в [поточному звіті](current-verification.md).
