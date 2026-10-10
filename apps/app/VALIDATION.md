# Перевірки desktop app

Поточні команди, результати, real devnet DTO та межі перевірок зібрані в одному
[звіті від 2026-10-10](../../docs/current-verification.md).

`npm run check` запускає TypeScript, ESLint, Vitest і production build.
`npm run test:e2e` окремо перевіряє browser rendering та поведінку з IPC fixtures;
це не live wallet scan. Native React/Tauri IPC і фактичні devnet reads описуються
окремо у звіті та [галереї скріншотів](../../docs/screenshots/README.md).

```sh
# З apps/app
npm run check
npm run format:check
npm run test:e2e
npm run desktop:build

# З кореня репозиторію
cargo fmt --all --check
cargo test --workspace --locked
```

`desktop:build` використовує frontend overlay і вбудовує `dist`. Поточна
конфігурація створює executable без installer bundle (`bundle.active=false`).
Сам `cargo build -p dock-flints-app` із базовим Tauri config не замінює цю збірку.

Збережені історичні матеріали:

- [Frontend/identity перевірки 2026-10-07](docs/history/validation-2026-10-07.md).
- [Native runtime та raster перевірки 2026-10-09](docs/desktop-verification.md).
- [Unified cleanup fixtures/validator verification](../../docs/unified-cleanup-verification.md).

Старі числа тестів і твердження про ще не exposed cleanup у звіті 2026-10-07
стосуються того commit, а не поточного app. Нинішні команди та обмеження описані
в [README](README.md), [FRONTEND](FRONTEND.md) та [BACKEND_GAPS](BACKEND_GAPS.md).
