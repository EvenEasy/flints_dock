# React frontend Flint’s Dock

Frontend — React + TypeScript + Vite всередині Tauri 2. Робочий desktop має
підключення локального wallet, read-only analysis, категорії, підготовку cleanup,
виконання погодженого плану та звіт. Бізнес-правила, RPC, провайдери, signer і
транзакції належать Rust. Це опис поточного коду; результати конкретних запусків
дивіться в [актуальному звіті](../../docs/current-verification.md).

## Запуск

Версії Node беріть з [`package.json`](package.json): `^22.22.2`, `^24.15.0` або
`>=26.0.0`. Для desktop потрібні Rust та системні бібліотеки Tauri;
[початок роботи](../../docs/getting-started.md) описує підготовку середовища.

```sh
cd apps/app
npm ci

# Живий frontend + Rust; Vite стартує автоматично.
DOCK_FLINTS_RPC_URL=https://api.devnet.solana.com npm run desktop

# Лише браузерний перегляд UI.
npm run dev
```

Dev server: `http://127.0.0.1:1420`. Два сервери не можуть одночасно займати цей
порт; перед `npm run desktop` зупиніть окремий `npm run dev`. Звичайний браузер
показує `DESKTOP APP REQUIRED` під час підключення й не збирає wallet secrets.
Для демонстраційних екранів відкрийте `/?preview=1#welcome`.

```sh
npm run build
npm run preview
# http://127.0.0.1:1421/?preview=1#main

npm run desktop:build
# Linux release executable: ../../target/release/dock-flints-app
```

`desktop:build` вбудовує `dist` через `tauri.frontend.conf.json`. Базовий
`src-tauri/tauri.conf.json` сам по собі не містить React assets. У конфігурації
`bundle.active=false`: збирається executable, а не готовий `.deb`/`.dmg` installer.

## Display та маршрути

За замовчуванням ширина phone frame — **430 CSS px**, backdrop зі зірками
увімкнено. Діапазон ширини — **360–480 CSS px**. Єдиний launcher обробляє параметри
для `dev`, `preview`, `desktop` і `desktop:build`:

```sh
npm run dev -- --width=390 --no-backdrop
npm run desktop -- --width=430 --backdrop
npm run desktop:build -- --width=390 --no-backdrop
```

URL overrides: `?width=390&backdrop=0`. `?layout=reference&preview=1` вмикає
reference layout. Phone frame орієнтується на пропорцію 390:844; responsive CSS
адаптує його до viewport. Фактичні матеріали та розміри captures є в
[галереї](../../docs/screenshots/README.md).

Канонічні hash routes: `#welcome`, `#scanning`, `#main`, `#cleanup`, `#processing`,
`#success`. Наприклад:

```text
http://127.0.0.1:1420/?preview=1&width=390&backdrop=0#main
http://127.0.0.1:1420/?preview=1&width=390&backdrop=0#cleanup
```

Preview містить п’ять reference screens і окремий processing screen; sample
суми та progress є демонстраційними. Preview scanning статичний; cleanup
переходить одразу до sample success. Processing відкривається окремо через
`/?preview=1#processing`, а п’ять reference screens — через preview menu чи hash.
Live `App` додатково перевіряє стан: зміна
hash не створює analysis або успішне виконання транзакцій.

## Analysis і категорії

Повний шлях даних:

```text
ConnectWalletDialog
  → connect_wallet → public WalletConnection
  → AnalyzeWalletRequest (address, selection, noPrices)
  → shared/api/wallet.ts → frontend-contract/wallet.ts
  → registered analyze_wallet → core scan snapshot + classification
  → WalletAnalysisDto → WalletAnalysis
  → useWalletAnalysis → normalizeWalletCategories
  → MainScreen + CategoryDialog
```

`useWalletAnalysis` очищає попередній snapshot на початку нового scan, перевіряє
`result.owner` і відкидає запізнілі відповіді за request generation. Dismiss,
перехід до welcome під час pending scan та disconnect скидають analysis state.
Перехід між main, inventory і cleanup зберігає прийнятий snapshot.
Dismiss не скасовує вже запущений RPC
запит: окремої backend cancel-команди немає.

`categoryPresentation.ts` нормалізує дані **один раз після IPC** й використовує
`presentCategory` для плиток та CategoryDialog. Ключі: `scam`, `nft`, `dust`,
`dead_token`. Items дедуплікуються за ID з об’єднанням backing accounts. Якщо
старий DTO не містить `categories.nft`, один compatibility fallback збирає
classic/Core/compressed inventory; незалежного підрахунку в MainScreen немає.

Numeric field кожної плитки містить виключно число або `—`, з однаковим
`type-numeric type-numeric--count` незалежно від status. Complete empty list із
підтвердженим нульовим count показує `0`. Неповний або невдалий результат без
підтверджених items показує `—`; з items показує число унікальних знайдених
активів. Причина, status і coverage містяться в tooltip, доступному описі та
CategoryDialog. Partial count означає «знайдено», а не повний підсумок.

Pricing, risk, routing і NFT discovery мають незалежні statuses. Ціна `null` не
стає нулем. Devnet не отримує mainnet Jupiter valuation/risk/routes. Без DAS
known classic/Core NFTs зберігаються; нуль у цих двох списках не доводить NFT=0.

## Cleanup та session state

`useCleanup` надсилає канонічні `assetIds` із inventory; unchecked активи
передаються як `ignoredAssetIds`. Backend також підтримує `mints`/`ignoredMints`
для сумісності. Кожна зміна selection підвищує revision та інвалідує старий plan.

`prepare_cleanup` повертає збережений Rust plan з діями й estimates.
`execute_cleanup` отримує лише `sessionId`, `planId`, action approval та Channel.
`RECOVER SOL` на live cleanup screen погоджує й виконує цей план; notices про
irreversible burn стоять перед кнопкою. Public-key session може inspect/prepare,
але `canSign=false` вимикає виконання. Після завершення UI відображає backend job
report і оновлює inventory.

Загублена IPC відповідь відновлюється через `get_cleanup_job` за job ID або plan
ID; execute повторно не викликається для recovery. Events фільтруються за
session/job/sequence. Exact balances і signed net deltas залишаються decimal
strings; integer арифметика використовує `BigInt`.

## Де змінювати

| Директорія / файл                             | Відповідальність                                               |
| --------------------------------------------- | -------------------------------------------------------------- |
| `src/app`                                     | Навігація, public session state, phone shell, display settings |
| `src/features/wallet`                         | Connect, scan, main screen, lifecycle analysis                 |
| `src/features/assets/categoryPresentation.ts` | Єдина нормалізація та presentation категорій                   |
| `src/features/assets`                         | CategoryDialog, inventory і selectable asset presentation      |
| `src/features/cleanup`                        | Plan selection, execution progress, actual result              |
| `src/features/preview`                        | Окремі позначені demonstration fixtures                        |
| `src/shared/api/wallet.ts`                    | Desktop transport і безпечні readable errors                   |
| `frontend-contract`                           | Typed invoke wrappers, statuses та DTO types                   |
| `src-tauri/src/commands`                      | Registered identity, analysis і cleanup IPC                    |
| `src/shared/format.ts`                        | Відображення exact amounts та приблизних USD values            |
| `src/styles.css`, `src/styles/`               | Спільні styles, phone/reference layout, typography             |
| `scripts/launch.ts`                           | Display arguments, Vite/Tauri запуск через argv                |
| `assets`                                      | Поставлені artwork/fonts та [provenance](assets/art/README.md) |

## Перевірки

```sh
# apps/app
npm run check
npm run format:check
npx playwright install chromium
npm run test:e2e
npm run desktop:build

# Repository root, якщо змінювався Rust/IPC.
cargo test -p dock-flints-app
cargo test -p dock-flints-core
```

Browser E2E використовує mocked або recorded IPC й перевіряє React; це не live
wallet scan. Registered IPC regressions використовують Tauri MockRuntime і
capability checks. Для реального native rendering потрібен запуск desktop
executable; відтворюваний локальний fixture описано в
[tests/native/README.md](tests/native/README.md). Актуальні виконані перевірки,
screenshots і обмеження наведено в [current verification](../../docs/current-verification.md).
