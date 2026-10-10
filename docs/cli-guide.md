# CLI: команди та приклади

Команди виконуються з кореня репозиторію. Cargo за замовчуванням запускає пакет `dock_flints`; готовий release binary має шлях `target/release/dock_flints`.

```bash
cargo run --locked -- --help
cargo run --locked -- scan --help
cargo run --locked -- cleanup --help
cargo run --locked -- quote --help
cargo run --locked -- swap --help
```

## Wallet identity і RPC

Кожна команда приймає рівно один із `--pubkey`/`-p`, `--keypair`, `--seed`.

| Input                                  | Призначення                                                                                             |
| -------------------------------------- | ------------------------------------------------------------------------------------------------------- |
| `--pubkey ADDRESS`                     | Read-only identity; scan, quote і cleanup preview                                                       |
| `--keypair /absolute/path/wallet.json` | Локальний Solana JSON keypair; address виводиться локально; signing доступний лише в execution commands |
| `--seed BASE64`                        | Рівно 32 raw Ed25519 bytes у base64; це не mnemonic і не 64-byte keypair                                |

Для read-only прикладів використовуйте public address. Keypair також працює зі `scan`, який нічого не підписує. Secret CLI arguments можуть потрапити у shell history/process list, тому для signer workflow зручніше використовувати файл.

CLI RPC передається через `--rpc-url`/`-r`; default — `https://api.mainnet.solana.com`. `DOCK_FLINTS_RPC_URL` налаштовує desktop, а не CLI. `--timeout-seconds` обмежує один RPC request, default `30`, allowed `1…300`; retries можуть подовжити загальну тривалість.

Налаштуємо змінні з публічними даними для наступних прикладів:

```bash
DEMO_WALLET=9FCR2PU1jZgCHyjWxzk2BNQHJxszAK24vBFiWmUyRNpv
DEVNET_RPC=https://api.devnet.solana.com
```

## Read-only scan

SOL balance:

```bash
cargo run --locked -- scan --pubkey "$DEMO_WALLET" \
  --rpc-url "$DEVNET_RPC" --balance --no-prices
```

Fungible та Unknown tokens, із backing accounts і точними raw amounts:

```bash
cargo run --locked -- scan --pubkey "$DEMO_WALLET" \
  --rpc-url "$DEVNET_RPC" --tokens --no-prices --details --show-mint
```

Повний raw token-account view, включно з empty accounts і NFT backing accounts:

```bash
cargo run --locked -- scan --pubkey "$DEMO_WALLET" \
  --rpc-url "$DEVNET_RPC" --all-tokens --no-prices --details --include-empty
```

Classic/programmable/MPL Core NFT:

```bash
cargo run --locked -- scan --pubkey "$DEMO_WALLET" \
  --rpc-url "$DEVNET_RPC" --nfts --no-prices --details --show-mint
```

Зберегти read-only результат у локальний JSON:

```bash
cargo run --locked -- scan --pubkey "$DEMO_WALLET" \
  --rpc-url "$DEVNET_RPC" --all --all-tokens --no-prices \
  --details --include-empty --format json > /tmp/flints-devnet-scan.json
```

JSON stdout не змішується з `--verbose` diagnostics на stderr. Наведений файл — місце для нового локального scan; він не замінює збережений evidence зі [звіту](current-verification.md).

| Flag                           | Що вибирає/змінює                                                                |
| ------------------------------ | -------------------------------------------------------------------------------- |
| `--balance`                    | Native SOL                                                                       |
| `--tokens`                     | Fungible SPL/Token-2022 й Unknown; verified NFT виключені                        |
| `--all-tokens`                 | Один рядок на raw token account; перетинається із semantic views                 |
| `--nfts`                       | Classic, programmable й uncompressed Core NFT                                    |
| `--cnfts`                      | RPC-only compressed capability result; enumeration у standalone scan ще відсутня |
| `--all`                        | SOL + tokens + nfts + cnfts; raw `--all-tokens` треба додати явно                |
| `--no-prices`                  | Не викликає pricing provider                                                     |
| `--include-empty`              | Показує zero-balance token asset rows                                            |
| `--details`                    | Raw amounts, decimals, accounts, mint/metadata й account summary                 |
| `--show-mint` / `--show-price` | Full IDs / unit prices у таблиці                                                 |
| `--format table\|json`         | Table default або structured JSON                                                |
| `--verbose` / `-v`             | Scanner diagnostics на stderr                                                    |

Без selection flags scan використовує `--all`. Flags комбінуються; `--all` задає всі semantic categories, але не додає raw view автоматично. Сумісний shorthand `cargo run -- -p ADDRESS --tokens` також збережено.

Confirmed fungibles агрегуються за mint/program; Unknown залишаються видимими, NFT не потрапляють у fungible pricing. Назва токена, decimals `0` чи balance `1` самі не підтверджують NFT. Default output приховує zero-balance asset rows. Навіть без `--include-empty`, `--details` JSON зберігає повний `discovered_accounts` inventory.

## JSON і status

CLI `scan` повертає `wallet` і тільки вибрані category objects: `sol`, `tokens`, `all_tokens`, `nfts`, `cnfts`. Кожен містить `status` і можливий `reason`. Цей terminal JSON відрізняється від Tauri `WalletAnalysis` DTO із frontend-категоріями `scam`, `nft`, `dust`, `dead_token`.

Приклад форми SOL-only JSON, із умовними значеннями:

```json
{
  "wallet": "PUBLIC_ADDRESS",
  "sol": {
    "status": "complete",
    "lamports": 1000000000,
    "balance": "1",
    "usd_value": null,
    "price": null,
    "pricing": { "status": "skipped", "reason": "Disabled by --no-prices" }
  }
}
```

Raw token amounts і агреговані суми — decimal strings. Інші CLI `u64` JSON fields потребують integer-aware parser. Prices/USD — approximate numbers або `null`; missing quote ніколи не стає `0`.

Scan exit code `0` означає, що хоча б одна вибрана SOL/token/NFT category дала complete або partial usable result. Це не гарантує повне coverage всіх categories. Code `2` означає, що usable result немає; JSON status/reason все одно доступні. Invalid input чи інші errors обробляє CLI error path.

Standalone `scan --cnfts` зараз повертає `historical_index_required`, `items: null` та exit `2`. `[]` означав би неправдиве підтвердження порожнього compressed inventory. `DOCK_FLINTS_DAS_URL` використовується desktop analysis і complete cleanup planner; він наразі не додає enumeration до standalone CLI scan.

## Pricing і provider scope

CLI scan викликає Jupiter pricing тільки коли потрібна оцінка, pricing не вимкнено, RPC genesis підтверджує mainnet і задано `JUPITER_API_KEY`. Наявна `.env` автоматично не завантажується. На devnet використовуйте `--no-prices`; mainnet ціни не підставляються до devnet holdings.

Для mainnet налаштуйте API key у environment запуску та вкажіть public mainnet wallet:

```bash
# MAINNET_WALLET має містити public address; JUPITER_API_KEY уже задано в environment
cargo run --locked -- scan --pubkey "$MAINNET_WALLET" \
  --rpc-url https://api.mainnet.solana.com --tokens --show-price --format json
```

SOL-only pricing запитує SOL. NFT-only scan не конструює price provider. Pricing failures не скасовують blockchain inventory. NFT/Unknown assets не отримують fungible valuations. Детальні provider boundaries — у [desktop configuration](../apps/app/README.md) та [swaps](swaps.md).

## Read-only cleanup preview

На документаційному devnet-гаманці використовуйте тільки preview:

```bash
cargo run --locked -- cleanup --pubkey "$DEMO_WALLET" \
  --rpc-url "$DEVNET_RPC" --dry-run --format json \
  > /tmp/flints-devnet-cleanup-plan.json
```

`cleanup` без `--execute` також preview. JSON містить `mode: "dry_run"`, `transactions_submitted: 0` і план. RPC/provider запити можуть знадобитися для побудови плану, але signing/send не відбуваються.

| Selection option     | Scope                                                     |
| -------------------- | --------------------------------------------------------- |
| `--account ADDRESS`  | Вибрані token accounts; repeatable                        |
| `--ignore-mint MINT` | Зберегти всі backing accounts цього mint, включно з empty |
| `--asset ID`         | Standalone NFT IDs; classic NFT ID — mint                 |
| `--ignore-asset ID`  | Захист Core/compressed ID                                 |

Swap/burn/close eligibility визначає backend із fresh evidence. Unsupported network не перетворюється на NoRoute. Structural routing unavailability може дати окремо пояснений burn/close plan; виконання такого плану потребує approval. [Cleanup guide](cleanup.md) пояснює точні правила й NFT limitations.

## Quote, swap і cleanup execution

Ці приклади описують окремий signer workflow; вони не використовуються для документаційного demo-wallet. Jupiter quote/swap орієнтовані на mainnet.

Read-only quote для exact mint/raw amount:

```bash
# MAINNET_WALLET — public mainnet address; це не token symbol
cargo run --locked -- quote --pubkey "$MAINNET_WALLET" \
  --mint EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v \
  --raw-amount 1000000 --format json
```

Для mainnet USDC `1000000` raw units = 1 USDC. Quote отримує маршрут без signing/send, не потребує Price V3 чи фактичного holding. `quote` не має `--rpc-url`; маршрут будує Jupiter.

Execution приклади для власного signer:

```bash
cargo run --locked -- swap --keypair /absolute/path/to/wallet.json \
  --rpc-url https://api.mainnet.solana.com \
  --mint EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v \
  --raw-amount 1000000 --slippage-bps 50 --max-price-impact-bps 100

cargo run --locked -- cleanup --keypair /absolute/path/to/wallet.json \
  --rpc-url https://api.mainnet.solana.com \
  --ignore-mint EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v --execute
```

Swap показує preview, просить `y`/`yes` і отримує fresh route. Cleanup показує план і просить ввести `cleanup`; погодження включає irreversible burns. `--yes` — явне noninteractive approval, cleanup вимагає також `--execute`. Native SOL потрібний для fees. Default slippage — 50 bps, max price impact — 100 bps, priority fee cap — 1,000,000 lamports, confirmation timeout — 90 s.

Після uncertain send CLI повертає signature: перевірте її стан перед новим запуском. Durable cleanup journal і restart reconciliation реалізовані в Tauri desktop; CLI не має persistent desktop journal. Спільні execution rules і ці межі описані в [cleanup](cleanup.md), single-token execution guarantees — у [swaps](swaps.md).

## Перевірки

```bash
cargo fmt --all --check
cargo check --workspace --locked
cargo test --workspace --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
```

Наявність mock fixtures чи успішного offline test не означає live provider coverage. [Поточний звіт](current-verification.md) відокремлює фактичний devnet scan, UI/IPC checks і решту тестів.
