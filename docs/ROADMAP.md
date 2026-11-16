# Дорожная карта

Платформа: **Windows 10 (2004+) / 11** в первую очередь. Linux и macOS — после v1.0.

## Фаза 0 — прототипы и замеры (снятие рисков)
- [x] Cargo workspace, `plugin-api` и чистая логика ядра (segmenter, stabilizer, hallucination) с тестами.
- [ ] Silero VAD через `ort`: CPU на 32 мс блок, стабильность на шуме/музыке/игре.
- [x] WASAPI: захват микрофона, loopback вывода и **Process Loopback** конкретного процесса на Win10 22H2; MMCSS; утилита замеров `probe` ([результаты](BENCHMARKS.md)).
- [ ] WASAPI: проверка на Win11, под нагрузкой (игра + запись OBS), при отключении и смене устройства, долгий прогон.
- [ ] whisper.cpp (`whisper-rs`) против CTranslate2 (`ct2rs`): RTF, WER, RAM/VRAM на CPU, CUDA и Vulkan для base/small/large-v3-turbo.
- [ ] CTranslate2 (`ct2rs`) для Opus-MT и M2M100: скорость и качество на 5 парах (ru-en, en-ru, en-de, en-ja через пивот, es-en). Проверить, что `ct2rs` нормально собирается на Windows (MSVC, CUDA); если нет — запасной путь через ONNX (`ort`).
- [ ] Набор golden-аудио (речь, музыка, игра на фоне, тишина) с эталонными текстами.
- [ ] Решение по профилям по умолчанию — по замерам.

## v0.1 — MVP (Windows)
- [ ] `plugin-api` 1.0: трейты Recognizer / Translator / Output, манифесты, `ModelSpec`, валидация и лицензионная политика.
- [ ] `host`: реестр встроенных плагинов, ModelManager (скачивание + SHA-256), изоляция паник и сбоев.
- [ ] Ядро: AudioCapture (устройства + loopback), VAD, Segmenter, Stabilizer, HallucinationFilter, Router с пивотом.
- [ ] Плагины: `whisper-cpp` (CPU + CUDA + Vulkan), `ct2` (Opus-MT, M2M100), `passthrough`, `overlay` (тема `default`), `text-file`, `transcript-log` (`.txt` + `.jsonl`).
- [ ] Сервер (axum) с токенами и Host/Origin-проверками.
- [ ] CLI: `run`, `devices`, `models download/verify`, `plugins`, `doctor`.
- [ ] Автовыбор профиля по железу.
- [ ] CI: clippy, rustfmt, тесты, `cargo-deny`, бенчмарки ресурсов на Windows.

## v0.2 — удобство
- [ ] Трей, автозапуск, portable `.zip` и MSI, подпись релизов.
- [ ] UI настроек (Svelte), визуальный редактор темы оверлея.
- [ ] Захват **отдельного приложения** (Process Loopback) с выбором из списка активных аудиосессий.
- [ ] Несколько источников одновременно.
- [ ] `obs`: closed captions и текстовый источник; `.srt`/`.vtt`, синхронизация с началом записи OBS.
- [ ] `ws-stream`.

## v0.3 — расширяемость и качество
- [ ] Внешние плагины: IPC-протокол, `vtt-plugin-sdk`, Job Object, затем AppContainer.
- [ ] `madlad` и `llm-local` со списком рекомендуемых моделей под Apache-2.0/MIT.
- [ ] Глоссарий терминов и имён.
- [ ] EcoQoS и тонкая настройка приоритетов по результатам замеров.

## После v1.0
- Linux (PipeWire) и macOS (ScreenCaptureKit) — через `cpal` и отдельные крейты `audio-linux` / `audio-mac`.
- Окно субтитров поверх экрана (always-on-top, click-through).
- Диаризация, TTS-озвучка перевода (как output-плагин), локализация UI.
