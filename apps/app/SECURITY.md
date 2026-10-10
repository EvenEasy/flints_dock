# Межі доступу та дані гаманця

Актуально для коду після `e013ea9`. React відображає результати та надсилає типізовані запити; Rust зберігає signer, перевіряє дані й виконує blockchain-операції.

## IPC та локальна сесія

Лише локальний webview `main` має capability для шести зареєстрованих команд: `connect_wallet`, `disconnect_wallet`, `analyze_wallet`, `prepare_cleanup`, `execute_cleanup`, `get_cleanup_job`. Інші вікна та remote origins не отримують цей доступ. Перевірка desktop runtime у React допомагає користувачеві; фактичні обмеження забезпечують Tauri та Rust.

Підключення підтримує публічну адресу, base64 від рівно 32 байтів Ed25519 seed або абсолютний шлях до локального Solana JSON keypair. Rust читає keypair самостійно; вміст файла не потрапляє у React. Для файла перевіряються тип, абсолютний шлях і розмір до 4096 байтів. Seed надсилається один раз, після чого поле очищується. Очищення поля не гарантує криптографічного стирання пам'яті webview.

Frontend отримує тільки публічну адресу, `sessionId`, `sourceKind` та `canSign`. Signer залишається в Rust і звільняється після disconnect, заміни неактивної сесії або завершення app. Під час активного cleanup зміна сесії та disconnect відхиляються.

## Читання та виконання

`analyze_wallet` приймає публічну адресу, scan selection та `noPrices`. Scan не підписує й не надсилає транзакцій. Public-key session може переглядати read-only cleanup plan, але не може виконувати його. Design preview використовує демонстраційні дані та не виконує blockchain-операцій.

`prepare_cleanup` створює незмінний план у Rust, прив'язаний до сесії, гаманця, мережі та revision. `execute_cleanup` приймає ID цього плану й погодження дій; він не приймає транзакцій, інструкцій чи довільного signer від React. Rust перевіряє термін дії, selection, ownership, amounts, authorities, restrictions і актуальні provider/proof дані. Виконання послідовне, із simulation, preflight та confirmation. Підключення гаманця саме по собі не погоджує виконання; кнопка **RECOVER SOL** на екрані плану погоджує показані Swap/Burn/Close дії.

Provider failure, відсутня ціна або невідома мережа не перетворюються на `NoRoute`. Категорія SCAM/DUST/DEAD TOKEN/NFT сама по собі не дозволяє burn. Після невизначеного send результату відновлення перевіряє вже збережену signature, а не повторює транзакцію.

## Зберігання та зовнішні дані

Secrets не записуються у browser storage, аналітику, DTO або signature journal. Налаштування RPC, Jupiter та DAS надходять із backend environment, без `VITE_*`. Provider endpoint не задається через frontend IPC. Devnet не використовує mainnet market/risk/routing records.

Journal зберігає публічні signatures, wallet/network, account, operation, expiry та відомі transaction deltas. OS file lock захищає одночасний доступ. Повні плани, jobs і frontend session живуть у пам'яті; restart не відновлює старий UI report. [Cleanup guide](../../docs/cleanup.md) описує reconciliation та обмеження.

Metadata, reasons та errors відображаються як escaped React text. Довільний HTML і remote artwork/metadata не завантажуються. RPC/provider diagnostics обмежені й не повинні містити credentials. Точні on-chain суми передаються рядками; USD valuations — приблизні числа.

CSP обмежує production assets/scripts локальними джерелами та IPC. Vite dev/preview слухають loopback, забороняють framing і додають заголовки безпеки; development дозволяє потрібні локальні HMR/WebSocket connections. Remote inspector використовується лише локально для документованих перевірок.

Залежності закріплені manifests і lockfiles. Результати dependency audit та native перевірок мають дату й не є гарантією для наступних версій. Поточні виконані перевірки й точний scope наведені у [звіті](../../docs/current-verification.md); чинні функції та обмеження — у [BACKEND_GAPS.md](BACKEND_GAPS.md).
