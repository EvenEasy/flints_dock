# Документація Flint’s Dock

Почніть із [getting-started](getting-started.md). Поточний продукт — React/Tauri desktop і Rust CLI зі спільним core. Історичні звіти нижче описують конкретні минулі перевірки, а не сьогоднішній стан гаманця чи кожної інтеграції.

## Запуск і користування

| Документ                                     | Зміст                                                                       |
| -------------------------------------------- | --------------------------------------------------------------------------- |
| [Початок роботи](getting-started.md)         | Передумови, desktop/browser/CLI, перший devnet scan                         |
| [CLI guide](cli-guide.md)                    | Copyable команди, вибір scan categories, JSON, cleanup preview, quote/swap  |
| [Desktop README](../apps/app/README.md)      | Запуск і build, backend environment, підключення, план та звіт              |
| [Troubleshooting](troubleshooting.md)        | Browser/native режим, build dependencies, RPC, coverage й типові помилки    |
| [Галерея скріншотів](screenshots/README.md)  | Екрани, сценарії, viewport і джерело даних                                  |
| [Поточна перевірка](current-verification.md) | Результати перевірок і read-only devnet scan для цієї редакції документації |
| [Cleanup](cleanup.md)                        | Selection, swap/burn/close, NFT adapters, accounting і journal              |
| [Swaps](swaps.md)                            | Jupiter quote/swap, точні raw units, підтвердження та execution limits      |

## Розробка

| Документ                                                     | Зміст                                                                    |
| ------------------------------------------------------------ | ------------------------------------------------------------------------ |
| [Архітектура](architecture.md)                               | Core/app/infra, CLI/Tauri boundaries, правила змін                       |
| [Frontend](../apps/app/FRONTEND.md)                          | React structure, navigation/state, API boundary і перевірки UI           |
| [Frontend contract](../apps/app/frontend-contract/README.md) | Типізовані IPC-команди та DTO                                            |
| [Поточні обмеження](../apps/app/BACKEND_GAPS.md)             | Provider coverage, стандарти та непідтримувані сценарії                  |
| [Дизайн](../apps/app/UI_DESIGN.md)                           | Visual reference й UI assets                                             |
| [Visual review](../apps/app/docs/visual-review.md)           | Попередні viewport/geometry перевірки дизайну                            |
| [Дослідження cNFT](cnfts.md)                                 | Чому RPC-only history не доводить compressed ownership; DAS requirements |
| [Research](research.md)                                      | Історія технічних рішень і SDK/API boundaries                            |

## Історичні звіти й fixtures

- [Validation](validation.md) — початковий scanner/workspace migration і mainnet observations у вказані дати.
- [Devnet cleanup verification](devnet-cleanup-verification.md) — попередні devnet/account перевірки.
- [Unified cleanup verification](unified-cleanup-verification.md) — canonical inventory, NFT cleanup і native validation.
- [Native desktop verification](../apps/app/docs/desktop-verification.md) — попередні packaged desktop результати.
- [`docs/fixtures/`](fixtures/) — записані inputs/outputs для конкретних regression scenarios. Synthetic observations мають тестове призначення; вони не є ринковими даними реального analysis.

Поточні числа гаманця отримуйте новим scan. Збережені скріншоти й JSON відтворюють лише момент знімання.
