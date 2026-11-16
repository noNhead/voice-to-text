# Плагины

## Идея

Конвейер (захват звука → VAD → стабилизация → роутинг → шина событий) — это **ядро**. В нём три **гнезда**:

| Гнездо | Что делает плагин | Вход | Выход |
|---|---|---|---|
| **Recognizer** | запускает модель речь → текст | аудио f32, 16 кГц, моно | слова с таймингами |
| **Translator** | запускает модель перевода | пакет текстов + контекст | пакет переводов |
| **Output** | выводит текст в нужном формате | события `segment` / `status` / `clear` | оверлей, файл, OBS, лог… |

Плагин отвечает **только за свою модель или свой формат**. Всё общее — захват звука, VAD, стабилизация частичного
текста, фильтр галлюцинаций, пивот-перевод, очереди, перезапуски, скачивание и проверка моделей — делает ядро.

### Плагин ≠ модель
**Плагин** — адаптер движка (whisper.cpp, CTranslate2, llama.cpp…). **Модель** — конкретные веса
(`large-v3-turbo-q5_0`, `opus-mt-ru-en`…). Один плагин обслуживает много моделей. Модели плагин объявляет
в манифесте, а скачивает и проверяет их ядро.

```toml
recognizer = "whisper-cpp"     # плагин (движок)
model      = "large-v3-turbo-q5_0"   # модель из каталога плагина
```

## Два вида плагинов

| | **Встроенные** (крейты) | **Внешние** (процессы) |
|---|---|---|
| Как подключаются | крейт в workspace реализует трейт из `plugin-api`, регистрируется в `app` (feature-флаг) | отдельный исполняемый файл, общение с ядром по IPC-протоколу |
| Накладные расходы | ноль (прямой вызов) | копирование данных между процессами (аудио — 64 КБ/с, это пренебрежимо мало) |
| Язык | Rust | любой (Rust, C++, Python, Go…) |
| Изоляция | в процессе приложения | отдельный процесс в Job Object (лимит памяти, kill-on-close) |
| Кому | движки, которые поставляет проект | сторонним авторам; экспериментальным моделям; языкам вне Rust |

**Динамические библиотеки (`.dll`) не используем.** У Rust нет стабильного ABI, а загрузка чужой DLL в процесс —
это и риск безопасности, и источник трудноуловимых падений.

## Место плагинов в конвейере

```
 AudioCapture ─▶ Resampler ─▶ VAD ─▶ Segmenter ─▶ ⧉ RECOGNIZER ─▶ Stabilizer + HallucinationFilter
                                                                              │ TextSegment
                                                                              ▼
                                        Router ── final ─▶ ⧉ TRANSLATOR(ы)  (пивот: ru→en→ja)
                                          │ partial                │
                                          ▼                        ▼
                                   EventBus (SegmentEvent: исходник + переводы)
                       ┌───────────┬───────────┼───────────┬────────────┐
                    ⧉ overlay  ⧉ ws-stream  ⧉ text-file  ⧉ obs  ⧉ transcript-log
```

## Крейт `vtt-plugin-api`

Единственная зависимость плагина от проекта. У него своя версия `PLUGIN_API_VERSION` (semver), ломающие изменения —
только при смене мажорной версии.

### Общие типы

```rust
pub const PLUGIN_API_VERSION: ApiVersion = ApiVersion { major: 1, minor: 0 };

pub struct ModelFile { pub path: String, pub url: String, pub sha256: String, pub size: u64 }

pub struct ModelSpec {
    pub id: String,                      // "large-v3-turbo-q5_0"
    pub files: Vec<ModelFile>,           // HTTPS + SHA-256 каждого файла
    pub license: String,                 // SPDX: "MIT", "Apache-2.0", "CC-BY-4.0"
    pub commercial_use: bool,
    pub attribution: Option<String>,     // для «О программе» / NOTICE
    pub languages: Languages,            // Any | Only(BTreeSet<String>)
    pub min_ram_mb: u32,
    pub min_vram_mb: Option<u32>,        // None — работает на CPU
    pub quality_tier: QualityTier,       // Eco | Balanced | Quality | Max
}

pub struct PluginManifest {
    pub id: String,                      // ^[a-z0-9][a-z0-9_.-]{1,63}$
    pub kind: PluginKind,                // Recognizer | Translator | Output
    pub name: String,
    pub version: String,
    pub api: ApiRange,                   // совместимые версии API: ">=1.0, <2"
    pub license: String,                 // лицензия кода плагина
    pub network: bool,                   // делает ли сетевые запросы сам
    pub devices: BTreeSet<Device>,       // Cpu | Cuda | Vulkan | Metal
    pub models: Vec<ModelSpec>,          // пусто для Output
    pub config_schema: serde_json::Value,// JSON Schema настроек (для UI и валидации)
    pub capabilities: BTreeMap<String, serde_json::Value>,
}

pub trait PluginContext: Send + Sync {
    fn device(&self) -> DeviceSelection;         // выбранное ядром устройство
    fn cpu_threads(&self) -> usize;              // бюджет потоков — плагин обязан соблюдать
    fn data_dir(&self) -> &Path;                 // личная папка плагина
    fn model_path(&self, model_id: &str) -> Result<PathBuf, PluginError>; // уже проверенная модель
    fn report_status(&self, status: PluginStatus);
    fn now(&self) -> jiff::Zoned;                // часы в таймзоне пользователя (подменяются в тестах)
}
```

Каждое гнездо состоит из двух частей. **Фабрика** (`*Factory`) регистрируется в реестре, отдаёт манифест
и создаёт экземпляр. **Экземпляр** — это загруженная модель или запущенный вывод. Так реестр хранит
`Box<dyn …Factory>` без обобщённых типов, а экземпляры живут в своих потоках.

### Recognizer

| Режим | Примеры | Что делает ядро |
|---|---|---|
| `Batch` | Whisper (whisper.cpp, CTranslate2), Parakeet | подаёт растущее окно; стабилизирует гипотезы (LocalAgreement-2); решает, когда сегмент `final` |
| `Streaming` | Vosk, Zipformer (sherpa-onnx) | подаёт куски аудио; плагин сам отдаёт `partial`/`final` |

```rust
pub struct Word { pub text: String, pub t_start: f32, pub t_end: f32, pub prob: f32 }
pub struct Hypothesis { pub words: Vec<Word>, pub lang: Option<String>, pub no_speech_prob: f32 }

pub trait RecognizerFactory: Send + Sync {
    fn manifest(&self) -> &PluginManifest;
    fn load(&self, model_id: &str, ctx: Arc<dyn PluginContext>, config: &serde_json::Value)
        -> Result<Recognizer, PluginError>;
}

pub enum Recognizer { Batch(Box<dyn BatchRecognizer>), Streaming(Box<dyn StreamingRecognizer>) }

pub trait BatchRecognizer: Send {
    /// Синхронно, в потоке asr-*. Тайминги слов — от начала `pcm16k`.
    fn transcribe(&mut self, pcm16k: &[f32], opts: &TranscribeOptions)
        -> Result<Hypothesis, PluginError>;
}

pub trait StreamingRecognizer: Send {
    fn open_stream(&mut self, lang: Option<&str>) -> Result<Box<dyn RecognizerStream>, PluginError>;
}

pub trait RecognizerStream: Send {
    fn feed(&mut self, pcm16k: &[f32]) -> Result<Vec<StreamResult>, PluginError>;
    fn flush(&mut self) -> Result<Vec<StreamResult>, PluginError>;   // конец речи по VAD
}
```

Правила:
- методы синхронные, вызываются в выделенном потоке ядра; плагин не создаёт свои потоки, кроме внутренних потоков движка в пределах `cpu_threads`;
- плагин не открывает аудиоустройства, не делает VAD и не скачивает модели;
- `transcribe(&mut self)` — ядро само сериализует доступ, когда одну модель используют несколько источников.

Whisper умеет сам переводить на английский: плагин объявляет `capabilities.translate_to = ["en"]`, и тогда роутер для
X→EN не вызывает переводчик (а передаёт `TranscribeOptions::translate_to_english`).

### Translator

```rust
pub struct TranslationRequest<'a> {
    pub text: &'a str, pub src: &'a str, pub dst: &'a str,
    pub context: &'a [String],                 // 1–3 предыдущих финальных сегмента
    pub glossary: &'a BTreeMap<String, String>,
}

pub trait TranslatorFactory: Send + Sync {
    fn manifest(&self) -> &PluginManifest;
    fn load(&self, model_id: &str, ctx: Arc<dyn PluginContext>, config: &serde_json::Value)
        -> Result<Box<dyn Translator>, PluginError>;
}

pub trait Translator: Send {
    fn pairs(&self) -> LanguagePairs;          // Any | Only(...)
    /// Синхронно, в потоке mt-*. Длина результата == длине batch.
    fn translate(&mut self, batch: &[TranslationRequest<'_>]) -> Result<Vec<String>, PluginError>;
}
```

Пивот делает ядро: если пары нет, Router строит цепочку `src→en→dst`, возможно из двух разных плагинов.

### Output

```rust
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub trait OutputFactory: Send + Sync {
    fn manifest(&self) -> &PluginManifest;
    fn create(&self, ctx: Arc<dyn PluginContext>, config: &serde_json::Value)
        -> Result<Box<dyn Output>, PluginError>;
}

pub trait Output: Send {
    fn accepts(&self) -> EventStages;          // PARTIAL | FINAL | COMPLETE | STATUS | CLEAR
    fn start<'a>(&'a mut self, web: Option<&'a mut dyn WebMount>) -> BoxFuture<'a, Result<(), PluginError>>;
    fn handle<'a>(&'a mut self, event: &'a Event) -> BoxFuture<'a, Result<(), PluginError>>;
    fn stop(&mut self) -> BoxFuture<'_, Result<(), PluginError>>;
}
```

`BoxFuture` вместо `async fn` в трейте нужен для того, чтобы трейт оставался dyn-совместимым без крейта `async-trait`.

- Работает в tokio. Блокирующий I/O — только через `spawn_blocking`.
- У каждого output своя ограниченная очередь: медленный вывод не тормозит остальные.
- `WebMount` отдаёт встроенную статику по пути `/o/<output-id>/…` (оверлей подключается к общему `/ws/events`). Проверки Host/Origin/токенов делает ядро.

## Внешние плагины: IPC-протокол

- Ядро запускает исполняемый файл плагина (`plugins/<id>/plugin.exe`, путь и SHA-256 — в `plugin.toml`) и общается
  с ним через **stdin/stdout**: кадры `u32 длина (LE) + u8 тип + полезная нагрузка`.
- Тип `0x01` — JSON-сообщение управления (`hello`/`manifest`/`load`/`transcribe`/`translate`/`event`/`status`/`error`),
  тип `0x02` — сырой PCM `f32le` для recognizer'а. stderr — логи плагина.
- Рукопожатие: ядро шлёт `hello {api}`, плагин отвечает `manifest`. Ядро валидирует манифест так же, как у встроенных.
- Процесс плагина живёт в **Job Object**: лимит памяти из `ModelSpec`, `KILL_ON_JOB_CLOSE`, приоритет ниже нормального.
- Таймауты на каждый вызов; при зависании или падении — перезапуск с backoff (см. «Устойчивость»).
- SDK: крейт `vtt-plugin-sdk` (тот же трейт, а транспорт берёт на себя SDK); позже — минимальный SDK для Python.

## Устойчивость

| Сбой | Поведение ядра |
|---|---|
| Ошибка / паника в recognizer | паника перехватывается (`catch_unwind` на границе потока), сегмент пропускается, статус `degraded`; после 3 сбоев подряд — перезагрузка модели с backoff |
| Ошибка / таймаут translator | финальный сегмент уходит без перевода с пометкой `error`, остальные переводы продолжаются |
| Ошибка в output | отключается только этот output (после N сбоев), остальные работают |
| Внешний плагин упал / завис | процесс убивается через Job Object, перезапуск с backoff |
| Плагин превышает бюджет | предупреждение в UI (время вызова, RAM/VRAM по плагинам) |

## Встроенные плагины

| Гнездо | id | Движок | Модели | Устройства |
|---|---|---|---|---|
| recognizer | `whisper-cpp` | whisper.cpp (`whisper-rs`) | base / small / medium / large-v3-turbo / large-v3 (GGML, q5_0/q8_0/f16) | cpu, cuda, vulkan |
| recognizer | `ct2-whisper` *(фаза 0, сравнение)* | CTranslate2 (`ct2rs`) | те же в формате CT2, int8/fp16 | cpu, cuda |
| translator | `ct2` | CTranslate2 (`ct2rs`) | opus-mt-*-* (CC-BY-4.0), m2m100-418m / 1.2b (MIT), madlad400-3b-mt (Apache-2.0) | cpu, cuda |
| translator | `llm-local` | OpenAI-совместимый API на localhost | модель пользователя; рекомендуемые — Apache-2.0/MIT | — |
| translator | `passthrough` | — | — (без перевода) | — |
| output | `overlay` | — | — | — |
| output | `ws-stream` | — | — | — |
| output | `text-file` | — | — | — |
| output | `obs` | obs-websocket v5 (`obws`) | — | — |
| output | `transcript-log` | — | — | — |

## Лицензионная политика моделей

Субтитры нужны прежде всего там, где есть деньги: монетизированные стримы, видео, рабочие звонки.
Поэтому **всё, что проект предлагает сам, можно использовать коммерчески.**

1. **Встроенный каталог** содержит только модели с лицензиями из allowlist:
   `MIT`, `Apache-2.0`, `BSD-2-Clause`, `BSD-3-Clause`, `CC0-1.0`, `CC-BY-4.0`, `MPL-2.0`. У всех
   `commercial_use = true`. Тест в `plugin-api` падает, если в каталог попадает что-то другое.
   Лицензия указывается **для каждой модели отдельно**: внутри одного семейства она может отличаться.
2. **Запрещено во встроенных плагинах:** NC-лицензии (NLLB, Seamless), «research only», лицензии с ограничениями
   использования (Llama Community, Gemma Terms), модели без явной лицензии.
3. **Сторонние плагины** могут объявлять любые модели, но `model = "auto"` **никогда** не выбирает модель с
   `commercial_use = false`. При ручном выборе такой модели UI показывает предупреждение и требует подтверждения.
4. **Атрибуция:** из `attribution` всех используемых моделей формируются «О программе» и `NOTICE`.
5. **`llm-local`** запускает модель, выбранную пользователем в его сервере. Проект показывает список рекомендуемых
   моделей под Apache-2.0/MIT, а за модели вне списка отвечает пользователь.

## Конфигурация

```toml
[recognizers.main]
plugin = "whisper-cpp"
model = "auto"                   # auto = выбор по железу из ModelSpec.quality_tier
[recognizers.main.config]
beam_size = 1

[translators.fast]
plugin = "ct2"
model = "auto"                   # подбирается под нужные пары (opus-mt, иначе m2m100)

[translators.smart]
plugin = "llm-local"
[translators.smart.config]
endpoint = "http://127.0.0.1:8080/v1"   # только loopback, иначе ошибка конфигурации

[[sources]]
id = "mic"
audio = { kind = "input", device = "default" }
recognizer = "main"
lang = "auto"
translate = [ { to = "en", with = "fast" }, { to = "de", with = "fast" } ]

[[sources]]
id = "discord"
audio = { kind = "app", process = "Discord.exe" }
recognizer = "main"
lang = "en"
translate = [ { to = "ru", with = "smart", fallback = "fast" } ]

[outputs.overlay]
plugin = "overlay"
sources = ["mic", "discord"]

[outputs.log]
plugin = "transcript-log"
[outputs.log.config]
formats = ["txt", "srt", "jsonl"]
```

## Как написать свой плагин
- **На Rust, в составе проекта:** крейт в `plugins/`, зависимость только от `vtt-plugin-api`, реализация трейта,
  регистрация в `app` под feature-флагом, контрактные тесты (`vtt_plugin_api::testing`).
- **Внешний (любой язык):** исполняемый файл с IPC-протоколом выше + `plugin.toml`. На Rust — через `vtt-plugin-sdk`.
- Контрактные тесты проверяют манифест, соблюдение `cpu_threads`, поведение на тишине и пустом вводе.
- Ограничение сети: Job Object сеть не ограничивает. Поэтому для внешних плагинов с `network = false` запуск
  в **AppContainer** без сетевых capabilities запланирован на v0.3. До этого сторонний внешний плагин —
  доверенный код, и UI предупреждает об этом при включении.
