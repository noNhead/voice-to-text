# voice-to-text

> Перевод речи в реальном времени — **локально, бесплатно, open source**.
> Репозиторий: https://github.com/noNhead/voice-to-text

Нативное приложение для Windows на Rust. Оно слушает выбранный источник звука (микрофон, весь системный звук или
отдельное приложение), распознаёт речь, переводит её и выводит:

1. **поток текста**: для OBS (текстовый файл, obs-websocket, closed captions) и своих интеграций (WebSocket);
2. **браузерный оверлей** с настраиваемым внешним видом (Browser Source в OBS или вкладка браузера);
3. **текстовый лог** с временем в вашем часовом поясе (`.txt`, `.srt`, `.vtt`, `.jsonl`).

## Принципы
- **Бесплатно и open source.** Никаких платных API и подписок.
- **Можно использовать коммерчески.** Все модели из каталога проекта это разрешают: монетизированные стримы, видео, рабочие звонки.
- **Локально.** Звук и текст не покидают компьютер. Телеметрии нет.
- **Качество.** Whisper для распознавания, специализированные модели для перевода, стабилизация потокового текста.
- **Экономно.** Один `.exe` на Rust: в тишине — меньше 0.5 % CPU и около 30 МБ RAM без моделей. Аудиопоток работает в реальном времени, поэтому игра, OBS и звонок не страдают.

## Плагины
Модели и форматы вывода сменные: распознавание (whisper.cpp, CTranslate2…), перевод (Opus-MT, M2M100, MADLAD,
локальная LLM…) и вывод — это плагины в общем конвейере. Подробнее — в [docs/PLUGINS.md](docs/PLUGINS.md).

## Платформы
**Windows 10 (2004+) / 11** — первая платформа. Работает на CPU; GPU ускоряет работу: NVIDIA (CUDA), AMD и Intel (Vulkan).
Linux и macOS — после v1.0.

## Статус
🚧 Фаза 0: прототипы и замеры. Подробнее — в [дорожной карте](docs/ROADMAP.md).

## Документация
| Документ | О чём |
|---|---|
| [docs/ANALYSIS.md](docs/ANALYSIS.md) | постановка задачи, риски, открытые вопросы |
| [docs/STACK.md](docs/STACK.md) | технологии, почему Rust, бюджет ресурсов |
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | конвейер, потоки, события, Cargo workspace, конфиг |
| [docs/PLUGINS.md](docs/PLUGINS.md) | плагины: трейты, внешние плагины, лицензионная политика |
| [docs/OUTPUTS.md](docs/OUTPUTS.md) | все форматы вывода и их настройки |
| [docs/ROADMAP.md](docs/ROADMAP.md) | фазы разработки |
| [docs/BENCHMARKS.md](docs/BENCHMARKS.md) | результаты замеров и команды для их повторения |
| [SECURITY.md](SECURITY.md) | модель угроз и меры защиты |
| [CONTRIBUTING.md](CONTRIBUTING.md) | как участвовать в разработке |

## Сторонние библиотеки

<!-- third-party:start -->
<!-- Генерируется командой `cargo xtask third-party`. Не редактируйте вручную. -->

Проект использует следующие open source библиотеки. Полные тексты их лицензий и уведомления об авторских правах — в [THIRD_PARTY_LICENSES.md](THIRD_PARTY_LICENSES.md); этот файл прикладывается к каждому релизу.

### Входят в сборку (65)

| Библиотека | Версия | Лицензия | Исходный код |
|---|---|---|---|
| `autocfg` | 1.5.1 | Apache-2.0 OR MIT | <https://github.com/cuviper/autocfg> |
| `displaydoc` | 0.2.7 | MIT OR Apache-2.0 | <https://github.com/yaahc/displaydoc> |
| `form_urlencoded` | 1.2.2 | MIT OR Apache-2.0 | <https://github.com/servo/rust-url> |
| `icu_collections` | 2.1.1 | Unicode-3.0 | <https://github.com/unicode-org/icu4x> |
| `icu_locale_core` | 2.1.1 | Unicode-3.0 | <https://github.com/unicode-org/icu4x> |
| `icu_normalizer` | 2.1.1 | Unicode-3.0 | <https://github.com/unicode-org/icu4x> |
| `icu_normalizer_data` | 2.1.1 | Unicode-3.0 | <https://github.com/unicode-org/icu4x> |
| `icu_properties` | 2.1.2 | Unicode-3.0 | <https://github.com/unicode-org/icu4x> |
| `icu_properties_data` | 2.1.2 | Unicode-3.0 | <https://github.com/unicode-org/icu4x> |
| `icu_provider` | 2.1.1 | Unicode-3.0 | <https://github.com/unicode-org/icu4x> |
| `idna` | 1.1.0 | MIT OR Apache-2.0 | <https://github.com/servo/rust-url/> |
| `idna_adapter` | 1.2.1 | Apache-2.0 OR MIT | <https://github.com/hsivonen/idna_adapter> |
| `itoa` | 1.0.19 | MIT OR Apache-2.0 | <https://github.com/dtolnay/itoa> |
| `jiff` | 0.2.46 | Unlicense OR MIT | <https://github.com/BurntSushi/jiff> |
| `jiff-core` | 0.1.8 | Unlicense OR MIT | <https://github.com/BurntSushi/jiff> |
| `jiff-static` | 0.2.46 | Unlicense OR MIT | <https://github.com/BurntSushi/jiff> |
| `jiff-tzdb` | 0.1.10 | Unlicense OR MIT | <https://github.com/BurntSushi/jiff> |
| `jiff-tzdb-platform` | 0.1.3 | Unlicense OR MIT | <https://github.com/BurntSushi/jiff> |
| `litemap` | 0.8.3 | Unicode-3.0 | <https://github.com/unicode-org/icu4x> |
| `log` | 0.4.35 | MIT OR Apache-2.0 | <https://github.com/rust-lang/log> |
| `memchr` | 2.8.3 | Unlicense OR MIT | <https://github.com/BurntSushi/memchr> |
| `num-integer` | 0.1.47 | MIT OR Apache-2.0 | <https://github.com/rust-num/num-integer> |
| `num-traits` | 0.2.19 | MIT OR Apache-2.0 | <https://github.com/rust-num/num-traits> |
| `percent-encoding` | 2.3.2 | MIT OR Apache-2.0 | <https://github.com/servo/rust-url/> |
| `potential_utf` | 0.1.6 | Unicode-3.0 | <https://github.com/unicode-org/icu4x> |
| `proc-macro2` | 1.0.107 | MIT OR Apache-2.0 | <https://github.com/dtolnay/proc-macro2> |
| `quote` | 1.0.47 | MIT OR Apache-2.0 | <https://github.com/dtolnay/quote> |
| `rtrb` | 0.4.1 | MIT OR Apache-2.0 | <https://github.com/mgeier/rtrb> |
| `serde` | 1.0.229 | MIT OR Apache-2.0 | <https://github.com/serde-rs/serde> |
| `serde_core` | 1.0.229 | MIT OR Apache-2.0 | <https://github.com/serde-rs/serde> |
| `serde_derive` | 1.0.229 | MIT OR Apache-2.0 | <https://github.com/serde-rs/serde> |
| `serde_json` | 1.0.154 | MIT OR Apache-2.0 | <https://github.com/serde-rs/json> |
| `smallvec` | 1.16.5 | MIT OR Apache-2.0 | <https://github.com/servo/rust-smallvec> |
| `stable_deref_trait` | 1.2.1 | MIT OR Apache-2.0 | <https://github.com/storyyeller/stable_deref_trait> |
| `syn` | 2.0.119 | MIT OR Apache-2.0 | <https://github.com/dtolnay/syn> |
| `syn` | 3.0.10 | MIT OR Apache-2.0 | <https://github.com/dtolnay/syn> |
| `synstructure` | 0.14.0 | MIT | <https://github.com/mystor/synstructure> |
| `thiserror` | 2.0.21 | MIT OR Apache-2.0 | <https://github.com/dtolnay/thiserror> |
| `thiserror-impl` | 2.0.21 | MIT OR Apache-2.0 | <https://github.com/dtolnay/thiserror> |
| `tinystr` | 0.8.5 | Unicode-3.0 | <https://github.com/unicode-org/icu4x> |
| `unicode-ident` | 1.0.27 | (MIT OR Apache-2.0) AND Unicode-3.0 | <https://github.com/dtolnay/unicode-ident> |
| `url` | 2.5.8 | MIT OR Apache-2.0 | <https://github.com/servo/rust-url> |
| `utf8_iter` | 1.1.0 | Apache-2.0 OR MIT | <https://github.com/hsivonen/utf8_iter> |
| `utf_types` | 1.0.0 | Apache-2.0 OR MIT | <https://github.com/hsivonen/utf_types> |
| `wasapi` | 0.27.0 | MIT | <https://github.com/HEnquist/wasapi-rs> |
| `windows` | 0.62.2 | MIT OR Apache-2.0 | <https://github.com/microsoft/windows-rs> |
| `windows-collections` | 0.3.2 | MIT OR Apache-2.0 | <https://github.com/microsoft/windows-rs> |
| `windows-core` | 0.62.2 | MIT OR Apache-2.0 | <https://github.com/microsoft/windows-rs> |
| `windows-future` | 0.3.2 | MIT OR Apache-2.0 | <https://github.com/microsoft/windows-rs> |
| `windows-implement` | 0.60.2 | MIT OR Apache-2.0 | <https://github.com/microsoft/windows-rs> |
| `windows-interface` | 0.59.3 | MIT OR Apache-2.0 | <https://github.com/microsoft/windows-rs> |
| `windows-link` | 0.2.1 | MIT OR Apache-2.0 | <https://github.com/microsoft/windows-rs> |
| `windows-numerics` | 0.3.1 | MIT OR Apache-2.0 | <https://github.com/microsoft/windows-rs> |
| `windows-result` | 0.4.1 | MIT OR Apache-2.0 | <https://github.com/microsoft/windows-rs> |
| `windows-strings` | 0.5.1 | MIT OR Apache-2.0 | <https://github.com/microsoft/windows-rs> |
| `windows-threading` | 0.2.1 | MIT OR Apache-2.0 | <https://github.com/microsoft/windows-rs> |
| `writeable` | 0.6.4 | Unicode-3.0 | <https://github.com/unicode-org/icu4x> |
| `yoke` | 0.8.4 | Unicode-3.0 | <https://github.com/unicode-org/icu4x> |
| `yoke-derive` | 0.8.4 | Unicode-3.0 | <https://github.com/unicode-org/icu4x> |
| `zerofrom` | 0.1.8 | Unicode-3.0 | <https://github.com/unicode-org/icu4x> |
| `zerofrom-derive` | 0.1.8 | Unicode-3.0 | <https://github.com/unicode-org/icu4x> |
| `zerotrie` | 0.2.5 | Unicode-3.0 | <https://github.com/unicode-org/icu4x> |
| `zerovec` | 0.11.9 | Unicode-3.0 | <https://github.com/unicode-org/icu4x> |
| `zerovec-derive` | 0.11.7 | Unicode-3.0 | <https://github.com/unicode-org/icu4x> |
| `zmij` | 1.0.23 | MIT | <https://github.com/dtolnay/zmij> |

### Только для разработки и тестов (1)

Эти библиотеки не входят в распространяемые файлы.

| Библиотека | Версия | Лицензия | Исходный код |
|---|---|---|---|
| `hound` | 3.5.1 | Apache-2.0 | <https://github.com/ruuda/hound> |

<!-- third-party:end -->

## Сборка из исходников
```bash
cargo build --release
```
```bash
cargo test --workspace
```
