---
name: security
description: Информационная безопасность проекта voice-to-text — нативный Rust-переводчик речи для Windows с HTTP/WebSocket-сервером (axum) на localhost, скачиванием ML-моделей, FFI к whisper.cpp/CTranslate2/ONNX Runtime, WinAPI (WASAPI/COM), внешними плагинами-процессами и OBS-интеграцией. Используй ВСЕГДА при изменении server, auth, API, WebSocket, оверлея/тем, ModelManager, работы с файлами и путями, конфига и секретов, IPC внешних плагинов, unsafe/FFI, зависимостей и CI, а также при security-ревью и разборе сообщений об уязвимостях.
---

# Информационная безопасность voice-to-text

Модель угроз — в `SECURITY.md`. Этот скилл — рабочий процесс и чек-листы.
Главный принцип: **приложение обрабатывает голос и переписку пользователя; браузер на той же машине — недоверенная
среда; всё внешнее (распознанный текст, темы, файлы моделей, сообщения IPC, ответы сети) — данные, а не код.**

## Процесс
1. Определи затронутые поверхности (таблица ниже).
2. Пройди соответствующие чек-листы. Каждый пункт: выполнен / неприменим (с причиной).
3. Добавь негативные тесты: неверный Host, чужой Origin, `../` в имени, неверный SHA-256, слишком большое сообщение IPC.
4. Если меняется модель угроз — обнови `SECURITY.md`.
5. В отчёте перечисли находки с уровнем (critical/high/medium/low) и что сделано.

| Поверхность | Где в коде | Чек-лист |
|---|---|---|
| HTTP API / WebSocket / статика | `crates/server` | A |
| Оверлей, темы, UI настроек | `plugins/output-overlay`, `web/`, `%APPDATA%\voice-to-text\themes` | B |
| Модели и скачивание | `crates/host` (ModelManager), `ModelSpec` в плагинах | C |
| Файлы и пути | `plugins/output-*`, `app::config`, модуль путей | D |
| Секреты и приватность | `server::auth`, `plugins/output-obs`, `tracing` | E |
| `unsafe`, FFI, WinAPI | `crates/audio-win`, `crates/vad`, плагины-движки | F |
| Зависимости, CI, релизы | `Cargo.toml`, `Cargo.lock`, `deny.toml`, `.github/` | G |
| Плагины: API, реестр, IPC | `crates/plugin-api`, `crates/host`, `plugins/*` | H |

## A. Локальный сервер (axum)
- [ ] Бинд по умолчанию — `127.0.0.1`. `0.0.0.0` только при `lan.enabled = true` + отдельный токен + allowlist IP.
- [ ] Middleware проверяет `Host` ∈ {`127.0.0.1:<port>`, `localhost:<port>`, `[::1]:<port>`}, иначе `421`. Это защита от DNS rebinding.
- [ ] WebSocket: проверка `Origin` (свой origin; для OBS Browser Source — отсутствующий или `null` Origin только вместе с валидным read-token).
- [ ] CORS-слой не подключается.
- [ ] `/api/*` — только `Authorization: Bearer <admin-token>`; токен не в cookie и не в query.
- [ ] read-token в query допускается только для `/o/<output-id>/` и `/ws/events` и даёт только чтение.
- [ ] Сравнение токенов — `subtle::ConstantTimeEq`; генерация — `getrandom` (32 байта).
- [ ] Изменение состояния — только `POST/PUT/DELETE` с `Content-Type: application/json`.
- [ ] Входные данные — `serde` со строгими типами (`deny_unknown_fields`), ограничения длины и диапазонов проверяются явно.
- [ ] Лимиты: `DefaultBodyLimit`, размер WS-сообщения, число WS-клиентов, таймауты (`tower-http`).
- [ ] Заголовки: CSP, `X-Content-Type-Options: nosniff`, `Referrer-Policy: no-referrer`, `Cache-Control: no-store` для API.
- [ ] Ошибки не раскрывают клиенту пути, содержимое конфига и цепочки ошибок.

## B. Оверлей, темы, UI
- [ ] Текст попадает в DOM только через `textContent`/`createTextNode`; `innerHTML`, `insertAdjacentHTML`, `document.write`, Svelte `{@html}` с данными запрещены.
- [ ] CSP: `default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self' data:; connect-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'`.
- [ ] Пользовательские темы — только `.css`, имя `^[a-z0-9_-]{1,64}$`, отдаются как `text/css`.
- [ ] Никаких внешних ресурсов (CDN, Google Fonts).
- [ ] admin-токен в UI — `sessionStorage`; после первичной ссылки убирается из URL (`history.replaceState`).

## C. Модели и скачивание
- [ ] Каждый файл модели в `ModelSpec`: HTTPS-URL из allowlist доменов (`huggingface.co`, `*.hf.co`, GitHub Releases), SHA-256, размер, лицензия. `validate_manifest` в `plugin-api` это проверяет.
- [ ] `reqwest` с `rustls`, без отключения проверки сертификатов, редиректы только в пределах allowlist.
- [ ] SHA-256 считается потоково при скачивании и при `models verify`; несовпадение → удалить файл, ошибка.
- [ ] Скачивание во временный файл в той же папке → проверка → атомарная замена (`MoveFileExW(MOVEFILE_REPLACE_EXISTING)`).
- [ ] Ограничение размера по `ModelFile.size` — защита от заполнения диска.
- [ ] Форматы без исполняемого кода: GGML/GGUF, CTranslate2, ONNX, safetensors, sentencepiece. Никаких pickle/`.pt`.
- [ ] Встроенный каталог — только лицензии из allowlist с `commercial_use = true` (тест). Модели сторонних плагинов с `commercial_use = false` никогда не выбираются через `auto` и скачиваются только после явного подтверждения.
- [ ] Скачивание никогда не происходит в фоне без согласия; в офлайн-режиме сетевых вызовов нет.

## D. Файлы и пути
- [ ] Базовые папки — только из модуля путей (`%APPDATA%`, `%LOCALAPPDATA%` через `directories`).
- [ ] Пользовательские имена (тема, источник, плагин, модель) валидируются regex до построения пути; итоговый путь после `canonicalize` проверяется через `starts_with(base)`.
- [ ] `ModelFile.path` — относительный, без `..`, без `:` и ведущего разделителя (проверяет `plugin-api`).
- [ ] Пользовательский путь для `text-file` и логов разрешён, но мы не следуем reparse points/симлинкам при создании и не перезаписываем чужой файл без подтверждения.
- [ ] Атомарная запись для live-файлов и конфига (временный файл + `ReplaceFileW`/`MoveFileExW`).
- [ ] Имена файлов логов формируются из даты и времени, а не из распознанного текста.
- [ ] TOML парсится только `toml` + `serde` (`deny_unknown_fields`).

## E. Секреты и приватность
- [ ] Пароль obs-websocket и токены — только в Windows Credential Manager (`keyring`).
- [ ] Тип `Secret<String>` без `Debug`/`Display`/`Serialize`; явный `expose()` только в месте использования; обнуление при drop (`zeroize`).
- [ ] `tracing`: нет токенов, паролей и распознанного текста на `info`+; поля с секретами помечены `skip`.
- [ ] `doctor` и отчёты об ошибках анонимизируют пути (`%USERPROFILE%` → `~`) и не включают транскрипты.
- [ ] Нет телеметрии и crash-reporter'ов во внешние сервисы.
- [ ] Новый исходящий сетевой вызов (не loopback и не скачивание модели) — нарушение правила «локально»; требуется обсуждение и опция, выключенная по умолчанию.
- [ ] loopback-эндпоинты (OBS, `llm-local`) валидируются: хост — только `127.0.0.1`/`::1`/`localhost`.
- [ ] Удаление «всего» удаляет и `live/`, и `transcripts/`.

## F. `unsafe`, FFI, WinAPI
- [ ] `#![forbid(unsafe_code)]` во всех крейтах, кроме `audio-win`, `vad` и плагинов-движков.
- [ ] Каждый `unsafe` блок — минимальный, с `// SAFETY:` (какие инварианты, кто их гарантирует).
- [ ] Буферы WASAPI: длина берётся из `GetBuffer`, данные копируются до `ReleaseBuffer`, проверяется формат (`WAVEFORMATEXTENSIBLE`), не предполагается f32.
- [ ] COM: RAII-обёртки, `CoInitializeEx` на поток, все `HRESULT` проверяются.
- [ ] FFI с C++-движками: паника не пересекает границу FFI; входные срезы с проверенной длиной; время жизни контекстов привязано к Rust-типам.
- [ ] `cargo miri` или тесты с санитайзерами для собственных `unsafe` обёрток, где это применимо.
- [ ] Никаких запросов прав администратора; процесс не повышает свой приоритет выше нормального (кроме MMCSS для аудиопотока).

## G. Зависимости, CI, релизы
- [ ] Новая зависимость: лицензия (allowlist `deny.toml`), поддержка, число транзитивных зависимостей, нативный код и откуда он берётся (build.rs, скачивающий бинарники, — отдельное обсуждение).
- [ ] `Cargo.lock` закоммичен; сборка в CI с `--locked`.
- [ ] `cargo deny check` (licenses, bans, advisories, sources — только crates.io) и `cargo audit` в CI.
- [ ] GitHub Actions закреплены по commit SHA; `permissions:` минимальные; без `pull_request_target` с checkout кода PR.
- [ ] Релизы: SHA-256 артефактов, SBOM (`cargo cyclonedx`), подпись кода; сборка только в CI.

## H. Плагины
- [ ] DLL-плагины не поддерживаются: встроенные — крейты, сторонние — только внешние процессы.
- [ ] Манифест любого плагина проходит `validate_manifest` (id, версия API, URL/хэши/пути моделей, лицензии); невалидный — не загружается.
- [ ] Сторонние плагины выключены по умолчанию; включение — явным действием с показом манифеста (автор, лицензия, `network`, устройства) и предупреждением о доверии к коду.
- [ ] Внешний плагин: путь к исполняемому файлу внутри `plugins/<id>/`, SHA-256 из `plugin.toml` проверяется перед запуском; запуск через `std::process::Command` со списком аргументов, без shell.
- [ ] Процесс плагина — в Job Object (`KILL_ON_JOB_CLOSE`, лимит памяти, пониженный приоритет); позже — AppContainer без сетевых capabilities для `network = false`.
- [ ] IPC: лимит размера кадра, проверка типа кадра, валидация каждого JSON-сообщения по схеме, таймауты на каждый вызов; некорректное сообщение → разрыв и перезапуск, а не паника ядра.
- [ ] Плагины получают только `PluginContext` (своя папка, проверенные пути к моделям); никакого доступа к токенам, конфигу других плагинов и серверу.
- [ ] Веб-части output-плагинов — только через `WebMount` под `/o/<id>/` с проверками ядра.
- [ ] Ошибки плагинов логируются без распознанного текста и без секретов из их конфига.

## Быстрые проверки при ревью
```bash
rg -n "innerHTML|insertAdjacentHTML|document\.write|\{@html" plugins/ web/
```
```bash
rg -n "unsafe" crates/ plugins/ --type rust
```
```bash
rg -n "0\.0\.0\.0|CorsLayer|danger_accept_invalid|Command::new\(\"cmd|libloading" crates/ plugins/ --type rust
```
```bash
rg -n "(info|warn|error)!\(.*text" crates/ plugins/ --type rust
```
```bash
cargo deny check && cargo audit
```

## Формат отчёта
```
## Security review: <что проверялось>
Поверхности: A, C, D
Находки:
- [high] <описание> — <файл:строка> — <исправлено / рекомендация>
Неприменимо: B (UI не затронут), F, G
Тесты добавлены: <список>
```
