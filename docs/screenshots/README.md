# Актуальні скріншоти

Зображення в `preview/` показують поточний **браузерний дизайн-прев’ю** з видимою
позначкою `DESIGN PREVIEW · NO TRANSACTIONS`. Суми, лічильники й результат
очищення на цих зображеннях демонстраційні. Вони не описують тестовий гаманець
або виконану транзакцію.

Зображення в `devnet/` показують зібраний React/Tauri застосунок, підключений
виключно через публічну адресу, отриману з `dev/demo-wallet.json`:
`9FCR2PU1jZgCHyjWxzk2BNQHJxszAK24vBFiWmUyRNpv`.
Сценарій знімання дозволяє лише `connect_wallet` із публічним ключем і
`analyze_wallet`. Він не готує, не підписує й не виконує очищення.

## Браузерний дизайн-прев’ю

| Екран | 430 CSS px | 360 CSS px |
| --- | --- | --- |
| Вітання | [welcome.png](preview/welcome.png) | [welcome-360.png](preview/welcome-360.png) |
| Приклад сканування | [scanning.png](preview/scanning.png) | [scanning-360.png](preview/scanning-360.png) |
| Головне меню | [main.png](preview/main.png) | [main-360.png](preview/main-360.png) |
| Вибір для очищення | [cleanup.png](preview/cleanup.png) | [cleanup-360.png](preview/cleanup-360.png) |
| Приклад обробки | [processing.png](preview/processing.png) | [processing-360.png](preview/processing-360.png) |
| Демонстраційний успіх | [success.png](preview/success.png) | [success-360.png](preview/success-360.png) |

Обидва набори знято з DPR 2. `preview/manifest.json` містить URL, розміри viewport,
версію браузера, час знімання та ревізію коду. Сценарій перевіряє, що числові поля
категорій містять тільки цифри або `—`, і що сторінка не має горизонтального
переповнення.

Від кореня репозиторію:

```sh
cd apps/app
npm ci
npx playwright install chromium
npm run dev -- --host 127.0.0.1
```

В іншому терміналі:

```sh
cd apps/app
node tests/native/documentation-preview-screenshots.mjs
```

`DOCK_PREVIEW_URL` змінює URL локального прев’ю;
`DOCK_PREVIEW_SCREENSHOTS` змінює каталог результатів. За замовчуванням зображення
записуються в `docs/screenshots/preview/` від кореня репозиторію.

## Справжній devnet у десктопному застосунку

Зберіть і запустіть застосунок на Linux з інспектором WebKitGTK, прив’язаним до
loopback. Потрібна доступна графічна сесія.

```sh
cd apps/app
CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=2 npm run desktop:build -- --no-backdrop --width=430
GDK_BACKEND=x11 GDK_SCALE=1 GDK_DPI_SCALE=1 \
WEBKIT_INSPECTOR_SERVER=127.0.0.1:19222 \
WEBKIT_INSPECTOR_HTTP_SERVER=127.0.0.1:19223 \
DOCK_FLINTS_RPC_URL=https://api.devnet.solana.com \
DOCK_FLINTS_JOURNAL_PATH=/tmp/flints-docs-readonly/signatures.json \
../../target/release/dock-flints-app
```

В іншому терміналі отримайте й передайте тільки публічну адресу:

```sh
cd apps/app
export DOCK_NATIVE_OWNER="$(solana-keygen pubkey ../../dev/demo-wallet.json)"
DOCK_NATIVE_REPORT=../../docs/screenshots/devnet \
node tests/native/category-counters-readonly.mjs
```

Файл keypair не потрапляє до застосунку чи сценарію знімання. `DOCK_NATIVE_OWNER`
приймає публічну base58 адресу; сценарій не читає файл гаманця. Без перевизначення
використовується публічна тестова адреса, наведена вище.

Звіт містить справжню відповідь зареєстрованого Tauri IPC, видимі лічильники,
деталі категорій, вимірювання шрифтів і плиток та кількість надісланих
транзакцій — нуль. На devnet категорії можуть показувати `—`, коли немає
достовірних цін, risk records чи маршрутів для цієї мережі. Причини й coverage
залишаються в діалогах. Наявність активів у кожній категорії не припускається.

Для другого native-набору на 360 CSS px під X11 закрийте всі діалоги й залиште
відкритим завершений scan. Змініть розмір єдиного вікна Flint’s Dock, потім
використайте той самий результат сканування:

```sh
# За GTK scale 1 фізичні пікселі тут збігаються з CSS-пікселями.
python3 tests/native/resize-x11-window.py 360 779
DOCK_NATIVE_REUSE_SCAN=1 \
DOCK_NATIVE_REPORT=../../docs/screenshots/devnet/phone-360 \
node tests/native/category-counters-readonly.mjs
```

Інспектор потрібен лише для знімання; у звичайному запуску його змінні оточення
можна прибрати. Native-зображень `processing` і `success` з цієї devnet-сесії
немає, оскільки вона лише для читання. Ці екрани документують окремо позначені
браузерні приклади.
