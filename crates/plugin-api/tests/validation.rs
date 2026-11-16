use std::collections::{BTreeMap, BTreeSet};

use vtt_plugin_api::{
    ApiRange, ApiVersion, Device, Languages, ModelFile, ModelSpec, PluginKind, PluginManifest,
    QualityTier, manifest_problems, validate_manifest,
};

fn good_file() -> ModelFile {
    ModelFile {
        path: "ggml-small-q5_0.bin".into(),
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small-q5_0.bin".into(),
        sha256: "a".repeat(64),
        size: 1000,
    }
}

fn good_model() -> ModelSpec {
    ModelSpec {
        id: "small-q5_0".into(),
        files: vec![good_file()],
        license: "MIT".into(),
        commercial_use: true,
        attribution: None,
        languages: Languages::Any,
        min_ram_mb: 600,
        min_vram_mb: None,
        quality_tier: QualityTier::Balanced,
    }
}

fn good() -> PluginManifest {
    PluginManifest {
        id: "whisper-cpp".into(),
        kind: PluginKind::Recognizer,
        name: "whisper.cpp".into(),
        version: "1.0.0".into(),
        api: ">=1.0, <2".into(),
        license: "MIT".into(),
        network: false,
        devices: BTreeSet::from([Device::Cpu, Device::Cuda, Device::Vulkan]),
        models: vec![good_model()],
        config_schema: serde_json::json!({}),
        capabilities: BTreeMap::new(),
    }
}

fn with_model(f: impl FnOnce(&mut ModelSpec)) -> PluginManifest {
    let mut m = good();
    f(&mut m.models[0]);
    m
}

fn with_file(f: impl FnOnce(&mut ModelFile)) -> PluginManifest {
    with_model(|m| f(&mut m.files[0]))
}

#[test]
fn good_manifest_is_valid_as_builtin() {
    validate_manifest(&good(), true).unwrap();
}

#[test]
fn api_ranges() {
    let v = ApiVersion { major: 1, minor: 0 };
    for (spec, ok) in [
        (">=1.0, <2", true),
        ("==1.0", true),
        (">=1.1", false),
        ("<1", false),
        (">=0.9, <1.5", true),
    ] {
        assert_eq!(spec.parse::<ApiRange>().unwrap().matches(v), ok, "{spec}");
    }
    for bad in ["", "~1.0", ">=one", ">= 1.x"] {
        assert!(bad.parse::<ApiRange>().is_err(), "{bad:?}");
    }
}

#[test]
fn bad_plugin_ids() {
    for id in ["A", "x", "../evil", "a b", "-lead", "Ab", &"a".repeat(65)] {
        let mut m = good();
        m.id = id.to_owned();
        assert!(!manifest_problems(&m, false).is_empty(), "{id:?}");
    }
}

#[test]
fn problems_are_collected_not_first_only() {
    let mut bad = with_file(|f| {
        f.url = "http://evil.example/model.bin".into();
        f.sha256 = "xyz".into();
    });
    bad.id = "BAD".into();
    let err = validate_manifest(&bad, false).unwrap_err();
    assert!(err.problems.len() >= 3, "{:?}", err.problems);
}

#[test]
fn model_url_policy() {
    for url in [
        "http://huggingface.co/x",
        "https://evil.example/x",
        "https://huggingface.co.evil.example/x",
        "https://user:pass@huggingface.co/x",
        "not a url",
    ] {
        assert!(
            !manifest_problems(&with_file(|f| f.url = url.into()), false).is_empty(),
            "{url}"
        );
    }
}

#[test]
fn hf_cdn_subdomain_allowed() {
    let m = with_file(|f| f.url = "https://cdn-lfs.hf.co/repo/model.bin".into());
    assert!(manifest_problems(&m, true).is_empty());
}

#[test]
fn model_file_path_traversal() {
    for path in [
        "../model.bin",
        "/etc/passwd",
        "a/../../b",
        "\\abs",
        "C:/x/model.bin",
        "",
        "a//b",
    ] {
        assert!(
            !manifest_problems(&with_file(|f| f.path = path.into()), false).is_empty(),
            "{path:?}"
        );
    }
}

#[test]
fn builtin_rejects_non_commercial_model() {
    let nc = with_model(|m| {
        m.license = "CC-BY-NC-4.0".into();
        m.commercial_use = false;
    });
    assert!(!manifest_problems(&nc, true).is_empty());
    // сторонний плагин может объявить NC-модель, если честно пометил commercial_use = false
    assert!(manifest_problems(&nc, false).is_empty());
}

#[test]
fn nc_license_cannot_claim_commercial_use() {
    let m = with_model(|m| m.license = "CC-BY-NC-4.0".into());
    assert!(!manifest_problems(&m, false).is_empty());
}

#[test]
fn builtin_rejects_unknown_license() {
    let m = with_model(|m| m.license = "LicenseRef-custom".into());
    assert!(!manifest_problems(&m, true).is_empty());
}

#[test]
fn cc_by_requires_attribution() {
    let m = with_model(|m| m.license = "CC-BY-4.0".into());
    assert!(!manifest_problems(&m, false).is_empty());
    let m = with_model(|m| {
        m.license = "CC-BY-4.0".into();
        m.attribution = Some("OPUS-MT, Helsinki-NLP, CC-BY-4.0".into());
    });
    assert!(manifest_problems(&m, true).is_empty());
}

#[test]
fn output_plugin_has_no_models() {
    let mut out = good();
    out.kind = PluginKind::Output;
    out.devices.clear();
    assert!(!manifest_problems(&out, false).is_empty());
    out.models.clear();
    assert!(manifest_problems(&out, false).is_empty());
}

#[test]
fn recognizer_needs_device() {
    let mut m = good();
    m.devices.clear();
    assert!(!manifest_problems(&m, false).is_empty());
}

#[test]
fn duplicate_model_ids() {
    let mut m = good();
    m.models.push(good_model());
    assert!(!manifest_problems(&m, false).is_empty());
}
