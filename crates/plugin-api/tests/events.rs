use std::collections::BTreeMap;

use jiff::Zoned;
use vtt_plugin_api::{ClearEvent, Event, SegmentEvent, SegmentState, TranslationResult};

fn at(s: &str) -> Zoned {
    s.parse().unwrap()
}

#[test]
fn segment_event_json_contract() {
    let ev = Event::Segment(SegmentEvent {
        id: "mic-000042".into(),
        state: SegmentState::Final,
        complete: true,
        source: "mic".into(),
        lang_src: Some("ru".into()),
        text_src: "Привет".into(),
        text_src_unstable: String::new(),
        translations: BTreeMap::from([
            (
                "en".to_owned(),
                TranslationResult {
                    text: Some("Hi".into()),
                    by: "ct2".into(),
                    error: None,
                },
            ),
            (
                "de".to_owned(),
                TranslationResult {
                    text: None,
                    by: "ct2".into(),
                    error: Some("timeout".into()),
                },
            ),
        ]),
        t_start: at("2026-11-16T14:03:12.481+03:00[Europe/Moscow]"),
        t_end: at("2026-11-16T14:03:14+03:00[Europe/Moscow]"),
        confidence: Some(0.87),
        recognizer: "whisper-cpp/small-q5_0".into(),
    });
    let json = serde_json::to_value(&ev).unwrap();
    assert_eq!(json["type"], "segment");
    assert_eq!(json["state"], "final");
    assert_eq!(json["t_start"], "2026-11-16T14:03:12.481+03:00");
    assert_eq!(json["t_end"], "2026-11-16T14:03:14.000+03:00");
    assert_eq!(
        json["translations"]["en"],
        serde_json::json!({"text": "Hi", "by": "ct2"})
    );
    assert_eq!(json["translations"]["de"]["error"], "timeout");
    assert!(json["translations"]["de"]["text"].is_null());
}

#[test]
fn clear_event_json() {
    let json = serde_json::to_value(Event::Clear(ClearEvent { source: None })).unwrap();
    assert_eq!(json, serde_json::json!({"type": "clear", "source": null}));
}
