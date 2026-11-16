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

### Входят в сборку (150)

| Библиотека | Версия | Лицензия | Исходный код |
|---|---|---|---|
| `anyhow` | 1.0.104 | MIT OR Apache-2.0 | <https://github.com/dtolnay/anyhow> |
| `anymap3` | 1.1.0 | BlueOak-1.0.0 OR MIT OR Apache-2.0 | <https://github.com/reivilibre/anymap3> |
| `autocfg` | 1.5.1 | Apache-2.0 OR MIT | <https://github.com/cuviper/autocfg> |
| `bit-set` | 0.11.1 | Apache-2.0 OR MIT | <https://github.com/contain-rs/bit-vec> |
| `bit-vec` | 0.10.2 | Apache-2.0 OR MIT | <https://github.com/contain-rs/bit-vec> |
| `borsh` | 1.8.1 | MIT OR Apache-2.0 | <https://github.com/near/borsh-rs> |
| `borsh-derive` | 1.8.1 | Apache-2.0 | <https://github.com/near/borsh-rs> |
| `byteorder` | 1.5.0 | Unlicense OR MIT | <https://github.com/BurntSushi/byteorder> |
| `bytes` | 1.12.1 | MIT | <https://github.com/tokio-rs/bytes> |
| `cc` | 1.11.2 | MIT OR Apache-2.0 | <https://github.com/rust-lang/cc-rs> |
| `cfg-if` | 1.0.5 | MIT OR Apache-2.0 | <https://github.com/rust-lang/cfg-if> |
| `cfg_aliases` | 0.2.2 | MIT | <https://github.com/katharostech/cfg_aliases> |
| `chacha20` | 0.10.2 | MIT OR Apache-2.0 | <https://github.com/RustCrypto/stream-ciphers> |
| `cpufeatures` | 0.3.1 | MIT OR Apache-2.0 | <https://github.com/RustCrypto/utils> |
| `derive-new` | 0.7.0 | MIT | <https://github.com/nrc/derive-new> |
| `displaydoc` | 0.2.7 | MIT OR Apache-2.0 | <https://github.com/yaahc/displaydoc> |
| `downcast-rs` | 2.0.2 | MIT OR Apache-2.0 | <https://github.com/marcianx/downcast-rs> |
| `dyn-clone` | 1.0.20 | MIT OR Apache-2.0 | <https://github.com/dtolnay/dyn-clone> |
| `dyn-hash` | 1.0.0 | MIT OR Apache-2.0 | <https://github.com/dtolnay/dyn-hash> |
| `either` | 1.19.0 | MIT OR Apache-2.0 | <https://github.com/rayon-rs/either> |
| `equivalent` | 1.0.3 | Apache-2.0 OR MIT | <https://github.com/indexmap-rs/equivalent> |
| `erased-serde` | 0.4.11 | MIT OR Apache-2.0 | <https://github.com/dtolnay/erased-serde> |
| `filetime` | 0.2.29 | MIT/Apache-2.0 | <https://github.com/alexcrichton/filetime> |
| `find-msvc-tools` | 0.1.15 | MIT OR Apache-2.0 | <https://github.com/rust-lang/cc-rs> |
| `foldhash` | 0.2.0 | Zlib | <https://github.com/orlp/foldhash> |
| `form_urlencoded` | 1.2.2 | MIT OR Apache-2.0 | <https://github.com/servo/rust-url> |
| `getrandom` | 0.4.3 | MIT OR Apache-2.0 | <https://github.com/rust-random/getrandom> |
| `half` | 2.7.1 | MIT OR Apache-2.0 | <https://github.com/VoidStarKat/half-rs> |
| `hashbrown` | 0.16.1 | MIT OR Apache-2.0 | <https://github.com/rust-lang/hashbrown> |
| `hashbrown` | 0.17.1 | MIT OR Apache-2.0 | <https://github.com/rust-lang/hashbrown> |
| `icu_collections` | 2.1.1 | Unicode-3.0 | <https://github.com/unicode-org/icu4x> |
| `icu_locale_core` | 2.1.1 | Unicode-3.0 | <https://github.com/unicode-org/icu4x> |
| `icu_normalizer` | 2.1.1 | Unicode-3.0 | <https://github.com/unicode-org/icu4x> |
| `icu_normalizer_data` | 2.1.1 | Unicode-3.0 | <https://github.com/unicode-org/icu4x> |
| `icu_properties` | 2.1.2 | Unicode-3.0 | <https://github.com/unicode-org/icu4x> |
| `icu_properties_data` | 2.1.2 | Unicode-3.0 | <https://github.com/unicode-org/icu4x> |
| `icu_provider` | 2.1.1 | Unicode-3.0 | <https://github.com/unicode-org/icu4x> |
| `idna` | 1.1.0 | MIT OR Apache-2.0 | <https://github.com/servo/rust-url/> |
| `idna_adapter` | 1.2.1 | Apache-2.0 OR MIT | <https://github.com/hsivonen/idna_adapter> |
| `indexmap` | 2.14.2 | Apache-2.0 OR MIT | <https://github.com/indexmap-rs/indexmap> |
| `inventory` | 0.3.25 | MIT OR Apache-2.0 | <https://github.com/dtolnay/inventory> |
| `itertools` | 0.15.0 | MIT OR Apache-2.0 | <https://github.com/rust-itertools/itertools> |
| `itoa` | 1.0.19 | MIT OR Apache-2.0 | <https://github.com/dtolnay/itoa> |
| `jiff` | 0.2.46 | Unlicense OR MIT | <https://github.com/BurntSushi/jiff> |
| `jiff-core` | 0.1.8 | Unlicense OR MIT | <https://github.com/BurntSushi/jiff> |
| `jiff-static` | 0.2.46 | Unlicense OR MIT | <https://github.com/BurntSushi/jiff> |
| `jiff-tzdb` | 0.1.10 | Unlicense OR MIT | <https://github.com/BurntSushi/jiff> |
| `jiff-tzdb-platform` | 0.1.3 | Unlicense OR MIT | <https://github.com/BurntSushi/jiff> |
| `lazy_static` | 1.5.1 | MIT OR Apache-2.0 | <https://github.com/rust-lang-nursery/lazy-static.rs> |
| `libm` | 0.2.16 | MIT | <https://github.com/rust-lang/compiler-builtins> |
| `litemap` | 0.8.3 | Unicode-3.0 | <https://github.com/unicode-org/icu4x> |
| `lock_api` | 0.4.14 | MIT OR Apache-2.0 | <https://github.com/Amanieu/parking_lot> |
| `log` | 0.4.35 | MIT OR Apache-2.0 | <https://github.com/rust-lang/log> |
| `maplit` | 1.0.2 | MIT/Apache-2.0 | <https://github.com/bluss/maplit> |
| `matrixmultiply` | 0.3.11 | MIT/Apache-2.0 | <https://github.com/bluss/matrixmultiply/> |
| `memchr` | 2.8.3 | Unlicense OR MIT | <https://github.com/BurntSushi/memchr> |
| `memmap2` | 0.9.12 | MIT OR Apache-2.0 | <https://github.com/RazrFalcon/memmap2-rs> |
| `metrics` | 0.24.6 | MIT | <https://github.com/metrics-rs/metrics> |
| `minijinja` | 3.1.0 | Apache-2.0 | <https://github.com/mitsuhiko/minijinja> |
| `ndarray` | 0.17.2 | MIT OR Apache-2.0 | <https://github.com/rust-ndarray/ndarray> |
| `nom` | 8.0.0 | MIT | <https://github.com/rust-bakery/nom> |
| `nom-language` | 0.1.0 | MIT | <https://github.com/rust-bakery/nom> |
| `num-bigint` | 0.4.8 | MIT OR Apache-2.0 | <https://github.com/rust-num/num-bigint> |
| `num-complex` | 0.4.6 | MIT OR Apache-2.0 | <https://github.com/rust-num/num-complex> |
| `num-integer` | 0.1.47 | MIT OR Apache-2.0 | <https://github.com/rust-num/num-integer> |
| `num-rational` | 0.4.2 | MIT OR Apache-2.0 | <https://github.com/rust-num/num-rational> |
| `num-traits` | 0.2.19 | MIT OR Apache-2.0 | <https://github.com/rust-num/num-traits> |
| `once_cell` | 1.21.4 | MIT OR Apache-2.0 | <https://github.com/matklad/once_cell> |
| `parking_lot` | 0.12.5 | MIT OR Apache-2.0 | <https://github.com/Amanieu/parking_lot> |
| `parking_lot_core` | 0.9.12 | MIT OR Apache-2.0 | <https://github.com/Amanieu/parking_lot> |
| `pastey` | 0.2.3 | MIT OR Apache-2.0 | <https://github.com/as1100k/pastey> |
| `percent-encoding` | 2.3.2 | MIT OR Apache-2.0 | <https://github.com/servo/rust-url/> |
| `potential_utf` | 0.1.6 | Unicode-3.0 | <https://github.com/unicode-org/icu4x> |
| `primal-check` | 0.3.4 | MIT OR Apache-2.0 | <https://github.com/huonw/primal> |
| `proc-macro-crate` | 3.5.0 | MIT OR Apache-2.0 | <https://github.com/bkchr/proc-macro-crate> |
| `proc-macro2` | 1.0.107 | MIT OR Apache-2.0 | <https://github.com/dtolnay/proc-macro2> |
| `prost` | 0.14.6 | Apache-2.0 | <https://github.com/tokio-rs/prost> |
| `prost-derive` | 0.14.6 | Apache-2.0 | <https://github.com/tokio-rs/prost> |
| `quote` | 1.0.47 | MIT OR Apache-2.0 | <https://github.com/dtolnay/quote> |
| `rand` | 0.10.4 | MIT OR Apache-2.0 | <https://github.com/rust-random/rand> |
| `rand_core` | 0.10.1 | MIT OR Apache-2.0 | <https://github.com/rust-random/rand_core> |
| `rand_distr` | 0.6.0 | MIT OR Apache-2.0 | <https://github.com/rust-random/rand_distr> |
| `rapidhash` | 4.5.1 | MIT OR Apache-2.0 | <https://github.com/hoxxep/rapidhash> |
| `rawpointer` | 0.2.1 | MIT/Apache-2.0 | <https://github.com/bluss/rawpointer/> |
| `rtrb` | 0.4.1 | MIT OR Apache-2.0 | <https://github.com/mgeier/rtrb> |
| `rustfft` | 6.4.1 | MIT OR Apache-2.0 | <https://github.com/ejmahler/RustFFT> |
| `rustversion` | 1.0.23 | MIT OR Apache-2.0 | <https://github.com/dtolnay/rustversion> |
| `same-file` | 1.0.6 | Unlicense/MIT | <https://github.com/BurntSushi/same-file> |
| `scopeguard` | 1.2.0 | MIT OR Apache-2.0 | <https://github.com/bluss/scopeguard> |
| `serde` | 1.0.229 | MIT OR Apache-2.0 | <https://github.com/serde-rs/serde> |
| `serde_core` | 1.0.229 | MIT OR Apache-2.0 | <https://github.com/serde-rs/serde> |
| `serde_derive` | 1.0.229 | MIT OR Apache-2.0 | <https://github.com/serde-rs/serde> |
| `serde_json` | 1.0.154 | MIT OR Apache-2.0 | <https://github.com/serde-rs/json> |
| `shlex` | 2.0.1 | MIT OR Apache-2.0 | <https://github.com/comex/rust-shlex> |
| `simd-adler32` | 0.3.10 | MIT | <https://github.com/mcountryman/simd-adler32> |
| `smallvec` | 1.16.5 | MIT OR Apache-2.0 | <https://github.com/servo/rust-smallvec> |
| `stable_deref_trait` | 1.2.1 | MIT OR Apache-2.0 | <https://github.com/storyyeller/stable_deref_trait> |
| `strength_reduce` | 0.2.4 | MIT OR Apache-2.0 | <http://github.com/ejmahler/strength_reduce> |
| `string-interner` | 0.20.0 | MIT/Apache-2.0 | <https://github.com/robbepop/string-interner> |
| `syn` | 2.0.119 | MIT OR Apache-2.0 | <https://github.com/dtolnay/syn> |
| `syn` | 3.0.10 | MIT OR Apache-2.0 | <https://github.com/dtolnay/syn> |
| `synstructure` | 0.14.0 | MIT | <https://github.com/mystor/synstructure> |
| `tar` | 0.4.46 | MIT OR Apache-2.0 | <https://github.com/composefs/tar-rs> |
| `thiserror` | 2.0.21 | MIT OR Apache-2.0 | <https://github.com/dtolnay/thiserror> |
| `thiserror-impl` | 2.0.21 | MIT OR Apache-2.0 | <https://github.com/dtolnay/thiserror> |
| `tinystr` | 0.8.5 | Unicode-3.0 | <https://github.com/unicode-org/icu4x> |
| `toml_datetime` | 1.1.3+spec-1.1.0 | MIT OR Apache-2.0 | <https://github.com/toml-rs/toml> |
| `toml_edit` | 0.25.19+spec-1.1.0 | MIT OR Apache-2.0 | <https://github.com/toml-rs/toml> |
| `toml_parser` | 1.1.5+spec-1.1.0 | MIT OR Apache-2.0 | <https://github.com/toml-rs/toml> |
| `tract-core` | 0.23.10 | MIT OR Apache-2.0 | <https://github.com/sonos/tract> |
| `tract-data` | 0.23.10 | MIT OR Apache-2.0 | <https://github.com/sonos/tract> |
| `tract-extra` | 0.23.10 | MIT OR Apache-2.0 | <https://github.com/sonos/tract> |
| `tract-hir` | 0.23.10 | MIT OR Apache-2.0 | <https://github.com/sonos/tract> |
| `tract-linalg` | 0.23.10 | MIT OR Apache-2.0 | <https://github.com/sonos/tract> |
| `tract-nnef` | 0.23.10 | MIT OR Apache-2.0 | <https://github.com/sonos/tract> |
| `tract-onnx` | 0.23.10 | MIT OR Apache-2.0 | <https://github.com/sonos/tract> |
| `tract-onnx-opl` | 0.23.10 | MIT OR Apache-2.0 | <https://github.com/sonos/tract> |
| `transpose` | 0.2.3 | MIT OR Apache-2.0 | <https://github.com/ejmahler/transpose> |
| `typeid` | 1.0.3 | MIT OR Apache-2.0 | <https://github.com/dtolnay/typeid> |
| `unicode-ident` | 1.0.27 | (MIT OR Apache-2.0) AND Unicode-3.0 | <https://github.com/dtolnay/unicode-ident> |
| `url` | 2.5.8 | MIT OR Apache-2.0 | <https://github.com/servo/rust-url> |
| `utf8_iter` | 1.1.0 | Apache-2.0 OR MIT | <https://github.com/hsivonen/utf8_iter> |
| `utf_types` | 1.0.0 | Apache-2.0 OR MIT | <https://github.com/hsivonen/utf_types> |
| `walkdir` | 2.5.0 | Unlicense/MIT | <https://github.com/BurntSushi/walkdir> |
| `wasapi` | 0.27.0 | MIT | <https://github.com/HEnquist/wasapi-rs> |
| `winapi-util` | 0.1.11 | Unlicense OR MIT | <https://github.com/BurntSushi/winapi-util> |
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
| `windows-sys` | 0.61.2 | MIT OR Apache-2.0 | <https://github.com/microsoft/windows-rs> |
| `windows-threading` | 0.2.1 | MIT OR Apache-2.0 | <https://github.com/microsoft/windows-rs> |
| `winnow` | 1.0.5 | MIT | <https://github.com/winnow-rs/winnow> |
| `writeable` | 0.6.4 | Unicode-3.0 | <https://github.com/unicode-org/icu4x> |
| `yoke` | 0.8.4 | Unicode-3.0 | <https://github.com/unicode-org/icu4x> |
| `yoke-derive` | 0.8.4 | Unicode-3.0 | <https://github.com/unicode-org/icu4x> |
| `zerocopy` | 0.8.63 | BSD-2-Clause OR Apache-2.0 OR MIT | <https://github.com/google/zerocopy> |
| `zerocopy-derive` | 0.8.63 | BSD-2-Clause OR Apache-2.0 OR MIT | <https://github.com/google/zerocopy> |
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
