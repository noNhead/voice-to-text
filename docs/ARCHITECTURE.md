# Архитектура

Приложение — один нативный процесс на Rust: **ядро-конвейер** с тремя **гнёздами для плагинов**:
Recognizer (модель речь→текст), Translator (модель перевода) и Output (формат вывода).
Контракты плагинов — в [PLUGINS.md](PLUGINS.md). Здесь описано, что делает ядро.

## Конвейер

```
 ┌──────────────┐ f32 16 кГц ┌─────┐ речь ┌───────────┐ окно ┌────────────────┐
 │ AudioCapture │───────────▶│ VAD │─────▶│ Segmenter │─────▶│ ⧉ Recognizer    │
 │ WASAPI: mic/ │    rtrb    └─────┘      └───────────┘      │   (плагин)     │
 │ loopback/app │                                            └───────┬────────┘
 └──────────────┘                                          Hypothesis │
                                          ┌──────────────────────────────────────▼─────────┐
                                          │ Stabilizer (LocalAgreement-2, для batch)       │
                                          │ HallucinationFilter                            │
                                          └──────────────────────────────┬─────────────────┘
                                                      TextSegment(partial|final)
                                                                          ▼
                     ┌──────────────── Router ◀──────────────────────────┘
                     │  partial ─────────────────────────────┐
                     │  final ──▶ ⧉ Translator(ы) (плагины)  │   пивот: src→en→dst
                     ▼                                       ▼
                   EventBus (tokio broadcast + очередь на каждый output)
          ┌──────────────┬──────────────┬──────────────┬──────────────┐
          ▼              ▼              ▼              ▼              ▼
      ⧉ overlay     ⧉ ws-stream   ⧉ text-file      ⧉ obs      ⧉ transcript-log
```

⧉ — гнездо плагина. Остальное — ядро, одинаковое для любых моделей.

### Ответственность

| Компонент | Ядро / плагин | Что делает |
|---|---|---|
| AudioCapture | ядро (`audio-win`) | WASAPI: устройства ввода, loopback вывода, Process Loopback конкретного приложения; MMCSS; сразу f32 моно 16 кГц (конвертирует аудиодвижок Windows); тишина в паузах loopback |
| VAD | ядро | Silero (ONNX): в тишине модели не вызываются |
| Segmenter | ядро | растущее окно для batch-моделей или куски для streaming-моделей; конец фразы |
| **Recognizer** | **плагин** | одна модель: аудио → слова с таймингами |
| Stabilizer | ядро | LocalAgreement-2: меняющиеся гипотезы → стабильный `partial` и `final` |
| HallucinationFilter | ядро | `no_speech_prob`, повторы n-грамм, стоп-лист фантомных фраз |
| Router | ядро | переводчики для источника, пивот-цепочки, батчи, fallback, таймауты |
| **Translator** | **плагин** | одна модель: пакет текстов → пакет переводов |
| EventBus | ядро | рассылка событий; у каждого output своя ограниченная очередь |
| **Output** | **плагин** | формат вывода |
| ModelManager | ядро (`host`) | скачивание моделей из манифестов, SHA-256, ленивая загрузка, выгрузка при простое |
| PluginHost | ядро (`host`) | реестр встроенных плагинов, внешние плагины-процессы, изоляция сбоев, учёт ресурсов |
| Server | ядро (`server`) | axum на 127.0.0.1: токены, Host/Origin-проверки, веб-части output-плагинов |
| Tray | бинарник (`app`) | иконка, меню, ссылка на настройки, пауза/старт |

Можно запустить **несколько источников одновременно** (например, «мой микрофон: RU→EN» и «Discord: EN→RU»).
Одна загруженная модель обслуживает все источники, которые на неё ссылаются.

## Потоки

| Поток | Приоритет | Что делает | Правило |
|---|---|---|---|
| `audio-<source>` (1 на источник) | MMCSS «Audio» | ждёт событие WASAPI, пишет PCM в `rtrb` | **никаких** аллокаций, блокировок, логов и syscalls, кроме WASAPI |
| `dsp-<source>` | normal | VAD, сегментация | блоки по 512 сэмплов (32 мс) |
| `asr-<recognizer>` (1 на экземпляр) | below normal | вызывает Recognizer | внутри модель использует не больше `cpu_threads` |
| `mt-<translator>` (1 на экземпляр) | below normal | вызывает Translator | батчит финальные сегменты |
| tokio (1–2 потока) | below normal + EcoQoS | Server, Router, EventBus, Output | никакого тяжёлого кода; блокирующий I/O — через `spawn_blocking` |
| main | normal | цикл сообщений трея (WinAPI) | — |

Связь между стадиями — **ограниченные каналы** (`crossbeam-channel`, `tokio::sync::mpsc`). При переполнении
выбрасываются **частичные** результаты, финальные — никогда. Счётчики потерь видны в `doctor` и UI.

## Стабилизация потока (для batch-моделей)

1. VAD помечает начало речи. Segmenter каждые `step_ms` (по умолчанию 500 мс) отдаёт плагину растущее окно до 15 с
   (вместе с pre-roll ~250 мс до начала речи).
2. **LocalAgreement-2**: слова, совпавшие в двух подряд гипотезах (без учёта регистра и пунктуации), подтверждаются.
   Подтверждённое никогда не меняется, остаток — «нестабильный хвост».
3. Пауза по VAD дольше `silence_ms` (600 мс) или окно достигло максимума → `final`. Окно сбрасывается, хвост
   предыдущего текста (до 200 символов) передаётся плагину как `prompt`.
4. Streaming-модели этот шаг пропускают: их `partial`/`final` идут сразу в HallucinationFilter.

## Перевод и события

- `partial` → в шину сразу, без перевода (перевод частичного текста — опция маршрута с задержкой 300–500 мс).
- `final` → событие с `complete=false` уходит сразу, чтобы оверлей показал оригинал без ожидания. Затем Router публикует
  то же событие (тот же `id`) с переводами и `complete=true`.
- Output-плагины подписываются на нужные стадии: оверлей — на все, лог — только на `complete`.

```jsonc
{
  "type": "segment",
  "id": "mic-000042",
  "state": "final",              // partial | final
  "complete": true,
  "source": "mic",
  "lang_src": "ru",
  "text_src": "Привет, сегодня мы посмотрим новую игру.",
  "text_src_unstable": "",
  "translations": {
    "en": { "text": "Hi, today we'll take a look at a new game.", "by": "ct2/opus-mt-ru-en" },
    "de": { "text": null, "error": "timeout", "by": "ct2/m2m100-418m" }
  },
  "t_start": "2026-11-16T14:03:12.481+03:00",
  "t_end":   "2026-11-16T14:03:14.020+03:00",
  "confidence": 0.87,
  "recognizer": "whisper-cpp/large-v3-turbo-q5_0"
}
```

Другие типы событий: `status` (источники, загрузка моделей, состояние плагинов), `clear`, `error`.

## Cargo workspace

```
Cargo.toml                    # workspace, общие версии зависимостей, профили сборки
crates/
  plugin-api/                 # ПУБЛИЧНЫЙ API плагинов: трейты, типы, манифесты, валидация (semver)
  core/                       # segmenter, stabilizer, hallucination, router, bus — без зависимостей от ОС
  vad/                        # Silero через ort
  audio-win/                  # WASAPI, loopback, Process Loopback, MMCSS (cfg(windows))
  host/                       # реестр плагинов, ModelManager, внешние плагины (IPC)
  server/                     # axum, auth, WebMount, встроенная статика
  app/                        # бинарник voice-to-text.exe: конфиг, трей, сборка конвейера, CLI
plugins/
  recognizer-whisper-cpp/     # whisper-rs; features: cuda, vulkan
  recognizer-ct2/             # ct2rs Whisper — для сравнения в фазе 0
  translator-ct2/             # ct2rs: opus-mt, m2m100, madlad
  translator-llm-local/       # OpenAI-совместимый API на localhost
  translator-passthrough/
  output-overlay/             # + web/ (HTML/CSS/JS, темы), встраивается rust-embed
  output-text-file/
  output-ws-stream/
  output-obs/                 # obws
  output-transcript-log/
web/settings/                 # Svelte UI (формы строятся по JSON Schema из конфигов плагинов)
tests/data/                   # эталонные WAV + тексты для golden-тестов
```

Правила зависимостей (проверяет `cargo-deny` + CI-скрипт):
- `plugins/*` зависят **только** от `plugin-api` (и своих внешних библиотек);
- `core` не зависит от ОС-специфичных крейтов и плагинов;
- конкретные плагины подключает только `app` — через feature-флаги и реестр.

## Источники звука (Windows)

| Тип | API | Примечание |
|---|---|---|
| Микрофон / любое устройство ввода | WASAPI shared, event-driven, `AUTOCONVERTPCM` | формат устройства → f32 моно 16 кГц силами Windows |
| Весь системный звук | WASAPI loopback на устройстве вывода | — |
| Отдельное приложение (Discord, игра, браузер) | **Process Loopback** (`ActivateAudioInterfaceAsync` + `AUDIOCLIENT_ACTIVATION_TYPE_PROCESS_LOOPBACK`) | Windows 10 2004+ (так же устроен «Захват звука приложения» в OBS); **проверено на Win10 22H2** ([замеры](BENCHMARKS.md)); захватывается процесс вместе с дочерними (`include_tree`) |

Пользователь выбирает источник в UI: список устройств и запущенных приложений с активным звуком (через `IAudioSessionManager2`).

Loopback (весь звук и Process Loopback) по документации WASAPI может не присылать пакеты, пока ничего не играет.
Чтобы время потока шло и сегментатор видел паузы, поток захвата досыпает тишину, если пакетов нет дольше 100 мс.
Досыпается только фактический пропуск, без сверки с часами всего потока, поэтому дрейф часов устройства не
накапливается.

## Локальный сервер

- Слушает **только `127.0.0.1`**, порт по умолчанию `7811`. Режим LAN включается явно (см. SECURITY.md).
- `GET /settings/` — UI настроек (admin-токен через одноразовую ссылку из трея).
- `/o/<output-id>/…` — веб-части output-плагинов (оверлей: `/o/overlay/?token=<read-token>`).
- `WS /ws/events?token=<read-token>` — поток событий только на чтение.
- `/api/*` — управление; `Authorization: Bearer <admin-token>`.

## Конфигурация

`%APPDATA%\voice-to-text\config.toml`, версия схемы — `schema_version`. Данные и модели лежат в
`%LOCALAPPDATA%\voice-to-text\`. Полный пример с recognizers/translators/sources/outputs — в
[PLUGINS.md](PLUGINS.md#конфигурация). Общие ключи ядра:

```toml
schema_version = 1
profile = "auto"            # auto | eco | balanced | quality | max
timezone = "auto"           # или "Europe/Moscow"
cpu_threads = "auto"        # общий бюджет потоков инференса
idle_unload_min = 10        # выгрузить модели после N минут без речи
eco_qos = true              # EcoQoS для фоновых потоков
```
