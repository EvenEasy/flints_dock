# Flint’s Dock desktop

React + TypeScript + Vite відображає UI; Tauri 2 керує wallet session,
конфігурацією провайдерів, cleanup plans, jobs і signature journal.
Discovery, classification та swap/burn/close виконуються в `crates/core`.
CLI використовує те саме ядро. Поточний desktop реалізує analysis і виконання
погодженого cleanup plan.

Почніть із [загального quick start](../../docs/getting-started.md).
[Галерея screenshots](../../docs/screenshots/README.md) показує екрани з підписаними
джерелами даних; [current verification](../../docs/current-verification.md)
містить фактично виконані перевірки. Developer map:
[FRONTEND.md](FRONTEND.md), [IPC contract](frontend-contract/README.md),
[integration limits](BACKEND_GAPS.md).

## Системні передумови

Для native build встановіть Rust/Cargo, Node із діапазону `package.json` та
бібліотеки своєї ОС. Для Ubuntu/Debian потрібні WebKitGTK 4.1, GTK development
dependencies і build tools. Приклад підготовки:

```sh
sudo apt-get update
sudo apt-get install build-essential pkg-config libwebkit2gtk-4.1-dev \
  libssl-dev libxdo-dev libayatana-appindicator3-dev librsvg2-dev curl wget file
```

На macOS потрібні Xcode Command Line Tools; на Windows — Microsoft C++ Build
Tools із Desktop development with C++, WebView2 і Rust MSVC toolchain.
Пакети для Arch/Fedora та інструкції для інших ОС наведені в
[офіційних Tauri 2 prerequisites](https://v2.tauri.app/start/prerequisites/).
Solana CLI не потрібний для scan за public address; `solana-keygen` у прикладах
лише локально виводить public key тестового файлу.

Команди з `export`, inline environment variables і POSIX paths нижче призначені
для Linux/macOS shell. Windows launch environment налаштуйте у своєму shell.

## Запуск desktop та browser preview

Потрібні Node із діапазону [`package.json`](package.json), Rust workspace
toolchain і системні бібліотеки Tauri. Node ranges: `^22.22.2`, `^24.15.0` або
`>=26.0.0`. Перед запуском із `apps/app`:

```sh
npm ci

# Devnet desktop: Vite і Rust стартують разом.
DOCK_FLINTS_RPC_URL=https://api.devnet.solana.com npm run desktop

# Development browser, без native IPC/signing.
npm run dev
# http://127.0.0.1:1420/?preview=1#welcome
```

Desktop launcher сам запускає Vite на port 1420. Зупиніть окремий dev server
перед desktop запуском. `npm run dev` — браузерний UI preview; local wallet
connection і analysis доступні лише в Tauri window. Sample values та progress
у `?preview=1` не є результатами wallet scan.

```sh
# Production React assets у браузері.
npm run build
npm run preview
# http://127.0.0.1:1421/?preview=1#main

# Desktop executable зі вбудованим React dist.
npm run desktop:build

# Linux, якщо CARGO_TARGET_DIR не перевизначено.
DOCK_FLINTS_RPC_URL=https://api.devnet.solana.com ../../target/release/dock-flints-app
```

Launcher об’єднує `tauri.frontend.conf.json` з базовою Rust configuration. Для
вбудованого `dist` використовуйте `desktop:build`, а не саму базову config.
`bundle.active=false`: installer packages наразі не створюються.

## Display options

```sh
npm run desktop -- --width=390 --no-backdrop
npm run desktop -- --width=430 --backdrop
npm run dev -- --width=480 --backdrop
npm run desktop:build -- --width=390 --no-backdrop
```

Ці параметри працюють для `dev`, `preview`, `desktop` і `desktop:build`.
Width: **360–480 CSS px**, default **430**. Default backdrop зі зірками увімкнено.
URL overrides: `?width=390&backdrop=0`; design reference:
`?layout=reference&preview=1`. Phone layout адаптується до вузького viewport,
списки assets прокручуються незалежно. Конкретні capture sizes наведено в галереї.

## Backend configuration

Rust читає **успадковані environment variables**. `.env` і
[`backend.env.example`](backend.env.example) автоматично не завантажуються.
Експортуйте settings у shell, який запускає desktop/executable, та перезапустіть
app після зміни. Provider credentials залишаються у Rust process environment;
не додавайте їх у `VITE_*`, frontend source або screenshot URLs.

| Variable                          | Default / призначення                                                                                       |
| --------------------------------- | ----------------------------------------------------------------------------------------------------------- |
| `DOCK_FLINTS_RPC_URL`             | `https://api.mainnet.solana.com`; absolute HTTP(S) Solana RPC                                               |
| `DOCK_FLINTS_RPC_TIMEOUT_SECONDS` | `30`; дозволено 1–300 seconds                                                                               |
| `JUPITER_API_KEY`                 | Optional Rust-only `x-api-key` для mainnet Price/Tokens/Swap; keyless availability визначає провайдер       |
| `DOCK_FLINTS_DAS_URL`             | Optional HTTP(S) DAS того самого network; `getGenesisHash`, `getAssetsByOwner`, `getAsset`, `getAssetProof` |
| `DOCK_FLINTS_DUST_USD`            | `0.01`; finite positive USD dust threshold                                                                  |
| `DOCK_FLINTS_JOURNAL_PATH`        | `$XDG_DATA_HOME/dock-flints/signatures.json`; fallback `$HOME/.local/share/dock-flints/signatures.json`     |

```sh
# apps/app; settings успадковує Rust subprocess.
export DOCK_FLINTS_RPC_URL=https://api.devnet.solana.com
export DOCK_FLINTS_RPC_TIMEOUT_SECONDS=30
npm run desktop
```

Optional mainnet key без внесення значення у shell history:

```sh
read -rsp 'Jupiter API key: ' JUPITER_API_KEY
export JUPITER_API_KEY
npm run desktop
```

RPC genesis hash визначає actual network. Jupiter prices/risk/routes доступні
лише для **verified mainnet**: devnet RPC не перетворює Jupiter на devnet provider.
Analysis повторює тимчасову помилку `getGenesisHash` до 3 разів з bounded timeout
і backoff. Exhausted verification дає Failed для scoped checks і зберігає
незалежно знайдені holdings; unknown network не стає verified Unsupported/NoRoute.

Без DAS compressed coverage — Unsupported, а classic/Core assets залишаються.
DAS перевіряє відповідність genesis hash. Неправильний формат DAS URL відхиляє
**startup**; виправте змінну або приберіть її. Malformed Jupiter key, навпаки,
ізолюється від RPC discovery. Auth/rate-limit/network errors мають свої reasons.

## Read-only devnet walkthrough

Для документації використовується public address тестового
`dev/demo-wallet.json`:

```text
9FCR2PU1jZgCHyjWxzk2BNQHJxszAK24vBFiWmUyRNpv
```

1. Запустіть desktop із `DOCK_FLINTS_RPC_URL=https://api.devnet.solana.com`.
2. Натисніть `CONNECT WALLET`. У `WALLET SOURCE` виберіть
   `Public key (read only)` та вставте адресу вище.
3. Залиште `Native SOL`, `Tokens`, `All token-account assets`, `NFTs / MPL Core`,
   `Compressed NFT capability` увімкненими. Prices увімкнені за замовчуванням;
   на devnet вони матимуть конкретний mainnet-only reason.
4. Натисніть `SCAN WALLET`. Після analysis відкрийте `SCAM`, `NFT`, `DUST` або
   `DEAD TOKEN`, щоб побачити items, evidence і coverage.
5. `HOLDS` відкриває inventory: `TOKENS`, `ALL ACCOUNTS`, `NFT / CORE`, `cNFT`.
   `PROFILE` показує `PUBLIC ADDRESS · READ ONLY`, balance, prices diagnostics
   і `CHANGE WALLET / RESCAN` / `DISCONNECT WALLET`.
6. На головному екрані `RECOVER SOL` відкриває cleanup для перегляду плану.
   Public-key session не може його виконати: execution button disabled і
   показано `Read-only wallet`. Для documentation scan цього достатньо.

Вміст keypair file, seed і secret bytes не потрібні для цього walkthrough.
Actual holdings можуть змінюватися з часом. Без достовірних devnet valuation,
risk і route evidence результат відповідної плитки — `—`, а не придуманий zero.
Відсутність DAS не доводить відсутність compressed NFTs.

Scan rejection лишає `ANALYSIS INTERRUPTED` з `RETRY SCAN` і `CHANGE WALLET`.
Partial scanner results доступні в деталях. Rescan очищає старі results;
запізніла відповідь не замінює новий wallet. Dismiss відкидає UI response,
але не скасовує backend RPC operation.

## Чотири category counters

Tiles і CategoryDialog використовують **один normalized category result**.
Keys: `scam`, `nft`, `dust`, `dead_token`. Count описує активи, а не число
backing token accounts. Holdings агрегуються за mint/program; NFT IDs
дедуплікуються; category flags можуть перетинатися.

| Status / evidence                                                       | Що показує numeric field |
| ----------------------------------------------------------------------- | ------------------------ |
| Complete, підтверджені items                                            | Число унікальних items   |
| Complete, confirmed empty result                                        | `0`                      |
| Partial, підтверджені items                                             | Число знайдених items    |
| Partial без items / failed / unsupported / skipped без збережених items | `—`                      |
| Missing result / початок scan                                           | `—`                      |

В numeric field немає `Partial`, `Not checked`, `Unavailable`, `N/A` або `+`.
Усі counters мають однаковий numeric style. Причина й coverage містяться в
CategoryDialog, tooltip та accessible description. Partial number означає
«стільки знайдено», а не гарантує повний підсумок.

- **SCAM:** explicit suspicious signal із source — optional `audit.isSus: true`
  для exact mint у Jupiter Tokens V2. False — negative observation; відсутнє
  поле/record — unknown. Name, decimals, authorities або organic score не є доказом.
- **DUST:** nonzero fungible mint/program holding із достовірною aggregate
  valuation `0 < value <= DOCK_FLINTS_DUST_USD`. Unpriced holdings не стають dust.
- **DEAD TOKEN:** nonzero fungible holding з explicit Jupiter Swap V2 `/build`
  NoRoute для exact mint/raw amount у recorded provider/time scope. Це не
  твердження про permanent worthlessness. API/liquidity/parsing failures — unknown.
- **NFT:** verified standard та ownership для classic/programmable/Core і
  owner-verified, unburned compressed assets із DAS. Без compressed coverage
  нуль classic/Core не означає гарантований NFT=0.

RPC, prices, risk, routing і DAS мають незалежні statuses. Pricing/risk errors
не блокують NFT; routing failure не приховує достовірний DUST. Price requests
batch 50 mints, risk — 100; shared Jupiter limiter має bounded retries.
Prices cache 30 seconds, risks — 60, keyed by mainnet/mint. Routing sequential
і swap builds fresh. Production analysis не читає synthetic devnet manifest.

## Cleanup для signer session

Окрім public key, desktop підтримує `File (keypair)` з **absolute local path**
і `Seed (base64)` від **рівно 32 raw Ed25519 bytes**. Це не mnemonic і не
64-byte keypair. Rust читає файл та зберігає signer у session; seed input
очищається після submission. Connection не авторизує транзакції.

У cleanup checked asset означає include, unchecked — keep. React передає
canonical `assetIds` / `ignoredAssetIds`; backend також підтримує mint selection.
Mint ignore захищає всі backing accounts, включно з empty accounts. IPC
розрізняє `all`, `selected`, `none`; empty selected — none.

`prepare_cleanup` створює read-only saved plan, прив’язаний до
wallet/session/network/revision, чинний 120 seconds після planning. UI показує
Swap/Burn/Close/Skip, причини та estimates **до fees**. Selection change
інвалідує план. Public key може prepare, але не execute.

На cleanup screen **RECOVER SOL погоджує та виконує саме показаний saved plan**.
Перед required burn UI повідомляє про irreversible action. Окремого другого
approval dialog або policy selector немає. Backend приймає plan ID й action
approval, а не frontend-authored transaction. Default limits: 50 bps slippage,
100 bps max price impact, 1,000,000 lamports max priority fee.

Execution послідовне: fresh quotes, on-chain identity/amount/authority checks,
simulation, preflight, confirmation і zero-balance reread перед close.
Unsupported plugins/extensions/proofs лишаються skip/review reasons.
Failed swap не стає burn; temporary provider error не дозволяє burn.
Детальні supported standards та обмеження: [cleanup coverage](../../docs/cleanup.md).

Job recovery читає existing job; не повторює execute. Active job блокує wallet
change/disconnect. Final SOL — exact signed sum wallet delta із transaction
metadata, включно з fees, а не whole-wallet before/after difference. Missing
metadata означає incomplete accounting. Partial/failed jobs мають окремий report.
Inventory оновлюється після виконання.

Перед send Rust durably зберігає nonsensitive signature, owner, network,
source account, operation і expiry. OS file lock захищає concurrent instances.
Uncertain submission не відправляється повторно; journal не містить seed,
keypair або transaction bytes. Plans/full reports поки в пам’яті: restart
зберігає reconciliation safety, але не відновлює стару React session.

## Перевірки та матеріали

```sh
# apps/app
npm run check
npm run format:check
npx playwright install chromium
npm run test:e2e
npm run desktop:build

# Repository root
cargo fmt --all --check
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

Browser tests використовують mocked/recorded IPC для React behavior; registered
Tauri IPC regressions використовують MockRuntime та capability enforcement.
[Native fixture](tests/native/README.md) відтворює isolated loopback RPC smoke.
Це різні evidence sources; browser mock не підтверджує live wallet scan.

- [Поточні виконані перевірки](../../docs/current-verification.md)
- [Screenshots і capture provenance](../../docs/screenshots/README.md)
- [Історична desktop verification](docs/desktop-verification.md)
- [Історична devnet account audit](../../docs/devnet-cleanup-verification.md)
- [Історична local-validator / unified cleanup verification](../../docs/unified-cleanup-verification.md)
- [Artwork provenance](assets/art/README.md)

`?diagnostics=art` — dev-only plain-image/component comparison, відсутній у
production rendering. Linux native reports не замінюють перевірки macOS/Windows.
