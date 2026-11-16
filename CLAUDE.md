# CLAUDE.md

Локальный open source переводчик речи в реальном времени. **Rust, Windows в первую очередь.**
Конвейер: WASAPI → VAD → Whisper → перевод → выходы (оверлей, текстовый файл/OBS, лог).

Перед работой прочитай:
- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) — конвейер, потоки, события, Cargo workspace.
- [docs/STACK.md](docs/STACK.md) — утверждённый стек и бюджет ресурсов. Не меняй стек без обсуждения.
- [docs/PLUGINS.md](docs/PLUGINS.md) — три гнезда (Recognizer, Translator, Output). Плагины зависят только от
  крейта `vtt-plugin-api`, общая логика живёт в ядре.

Скиллы проекта:
- `writing-code` — при любом написании или изменении кода.
- `security` — при работе с сервером, сетью, файлами, моделями, конфигом, секретами, IPC, `unsafe`/FFI и при ревью.

Жёсткие правила: всё бесплатно и open source; модели — только с коммерческой лицензией; никаких сетевых вызовов,
кроме скачивания моделей с проверкой SHA-256 и localhost; в аудиопотоке нет аллокаций и блокировок; без Electron/Tauri.

Команды: `cargo build --workspace`, `cargo test --workspace`,
`cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all`, `cargo deny check`,
`cargo xtask third-party` (после изменения зависимостей: список библиотек в README и THIRD_PARTY_LICENSES.md).
