# Участие в разработке

## Окружение (Windows)
- Rust stable (`rustup`, toolchain `x86_64-pc-windows-msvc`)
- Visual Studio 2022 Build Tools (C++), CMake ≥ 3.22
- LLVM/clang — для сборки `whisper-rs` (bindgen); путь задаётся в `LIBCLANG_PATH`
- Опционально: CUDA Toolkit 12 (feature `cuda`), Vulkan SDK (feature `vulkan`)
- Node.js 22 LTS — только для сборки `web/settings`
- `cargo install cargo-deny cargo-audit`

```bash
cargo build --workspace
```
```bash
cargo test --workspace
```
```bash
cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings
```
```bash
cargo deny check
```

## Правила
- Стиль и архитектурные правила — в [.claude/skills/writing-code/SKILL.md](.claude/skills/writing-code/SKILL.md).
  Они обязательны и для людей, и для ИИ-ассистентов.
- Изменения в сервере, сети, файлах, моделях, IPC, `unsafe` и FFI проверяются по чек-листу
  [.claude/skills/security/SKILL.md](.claude/skills/security/SKILL.md).
- Новая зависимость = обоснование в PR (лицензия, размер, число транзитивных зависимостей, альтернативы) и запись
  в [docs/STACK.md](docs/STACK.md). `cargo deny check` должен проходить.
- После любого изменения зависимостей запустите `cargo xtask third-party`. Команда обновит таблицу библиотек в README
  и тексты лицензий в [THIRD_PARTY_LICENSES.md](THIRD_PARTY_LICENSES.md). Лицензии MIT, BSD и Apache-2.0 требуют
  прикладывать уведомления об авторских правах к каждой копии, поэтому этот файл входит в каждый релиз.
- Изменения горячего пути (аудио → VAD → ASR → MT) сопровождаются результатами бенчмарка `cargo bench` до и после.
- Коммиты — [Conventional Commits](https://www.conventionalcommits.org/) (`feat:`, `fix:`, `perf:`, `docs:` …).
