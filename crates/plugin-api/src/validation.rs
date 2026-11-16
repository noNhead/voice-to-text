//! Проверка манифестов плагинов и лицензионной политики моделей (`docs/PLUGINS.md`).

use url::Url;

use crate::error::ManifestError;
use crate::types::{ApiRange, ApiVersion, ModelFile, ModelSpec, PluginKind, PluginManifest};

pub const PLUGIN_API_VERSION: ApiVersion = ApiVersion { major: 1, minor: 0 };

/// Лицензии, разрешающие коммерческое использование. Только они допустимы во встроенном каталоге.
pub const COMMERCIAL_LICENSE_ALLOWLIST: &[&str] = &[
    "MIT",
    "Apache-2.0",
    "BSD-2-Clause",
    "BSD-3-Clause",
    "CC0-1.0",
    "CC-BY-4.0",
    "MPL-2.0",
];

/// Лицензии, которые точно запрещают коммерческое использование (защита от ошибки в манифесте).
pub const NON_COMMERCIAL_LICENSES: &[&str] = &[
    "CC-BY-NC-4.0",
    "CC-BY-NC-SA-4.0",
    "CC-BY-NC-ND-4.0",
    "CC-BY-NC-3.0",
    "CC-BY-NC-SA-3.0",
];

const ATTRIBUTION_REQUIRED: &[&str] = &["CC-BY-4.0"];

/// Точные хосты или суффиксы (начинаются с точки).
const MODEL_HOST_ALLOWLIST: &[&str] = &[
    "huggingface.co",
    ".hf.co",
    "github.com",
    "objects.githubusercontent.com",
];

/// `^[a-z0-9][a-z0-9_.-]{1,63}$`
#[must_use]
pub fn is_valid_plugin_id(id: &str) -> bool {
    let mut chars = id.chars();
    (2..=64).contains(&id.len())
        && chars
            .next()
            .is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || "_.-".contains(c))
}

fn is_valid_model_id(id: &str) -> bool {
    let mut chars = id.chars();
    (1..=128).contains(&id.len())
        && chars.next().is_some_and(|c| c.is_ascii_alphanumeric())
        && chars.all(|c| c.is_ascii_alphanumeric() || "_.-".contains(c))
}

fn is_sha256(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn host_allowed(host: &str) -> bool {
    MODEL_HOST_ALLOWLIST
        .iter()
        .any(|allowed| match allowed.strip_prefix('.') {
            Some(base) => host.len() > allowed.len() && host.ends_with(allowed) || host == base,
            None => host == *allowed,
        })
}

fn is_safe_relative_path(path: &str) -> bool {
    !path.is_empty()
        && !path.starts_with(['/', '\\'])
        && !path.contains(':')
        && path
            .split(['/', '\\'])
            .all(|part| part != ".." && !part.is_empty())
}

fn file_problems(f: &ModelFile, place: &str) -> Vec<String> {
    let mut p = Vec::new();
    match Url::parse(&f.url) {
        Ok(url) if url.scheme() != "https" => {
            p.push(format!("{place}: {}: url must be https", f.path));
        }
        Ok(url) if !url.username().is_empty() || url.password().is_some() => {
            p.push(format!(
                "{place}: {}: url must not contain credentials",
                f.path
            ));
        }
        Ok(url) => {
            let host = url.host_str().unwrap_or_default();
            if !host_allowed(host) {
                p.push(format!("{place}: {}: host {host:?} is not allowed", f.path));
            }
        }
        Err(e) => p.push(format!("{place}: {}: bad url: {e}", f.path)),
    }
    if !is_sha256(&f.sha256) {
        p.push(format!(
            "{place}: {}: sha256 must be 64 lowercase hex chars",
            f.path
        ));
    }
    if f.size == 0 {
        p.push(format!("{place}: {}: size must be positive", f.path));
    }
    if !is_safe_relative_path(&f.path) {
        p.push(format!(
            "{place}: {:?}: path must be relative and stay inside model dir",
            f.path
        ));
    }
    p
}

/// Проблемы описания модели. Пустой список — модель валидна.
#[must_use]
pub fn model_problems(model: &ModelSpec, builtin: bool) -> Vec<String> {
    let place = format!("model {:?}", model.id);
    let mut p = Vec::new();
    if !is_valid_model_id(&model.id) {
        p.push(format!("{place}: bad id"));
    }
    if model.files.is_empty() {
        p.push(format!("{place}: no files"));
    }
    for f in &model.files {
        p.extend(file_problems(f, &place));
    }
    let license = model.license.as_str();
    if NON_COMMERCIAL_LICENSES.contains(&license) && model.commercial_use {
        p.push(format!("{place}: license {license} forbids commercial use"));
    }
    if ATTRIBUTION_REQUIRED.contains(&license) && model.attribution.is_none() {
        p.push(format!(
            "{place}: license {license} requires attribution text"
        ));
    }
    if builtin {
        if !COMMERCIAL_LICENSE_ALLOWLIST.contains(&license) {
            p.push(format!(
                "{place}: license {license:?} is not in the commercial allowlist"
            ));
        }
        if !model.commercial_use {
            p.push(format!(
                "{place}: built-in models must allow commercial use"
            ));
        }
    }
    p
}

/// Все проблемы манифеста. Пустой список — манифест валиден.
#[must_use]
pub fn manifest_problems(manifest: &PluginManifest, builtin: bool) -> Vec<String> {
    let mut p = Vec::new();
    if !is_valid_plugin_id(&manifest.id) {
        p.push("bad plugin id (expected ^[a-z0-9][a-z0-9_.-]{1,63}$)".to_owned());
    }
    match manifest.api.parse::<ApiRange>() {
        Ok(range) if !range.matches(PLUGIN_API_VERSION) => p.push(format!(
            "plugin API {:?} is incompatible with {PLUGIN_API_VERSION}",
            manifest.api
        )),
        Ok(_) => {}
        Err(e) => p.push(e.to_string()),
    }
    match manifest.kind {
        PluginKind::Output if !manifest.models.is_empty() => {
            p.push("output plugins must not declare models".to_owned());
        }
        PluginKind::Recognizer | PluginKind::Translator if manifest.devices.is_empty() => {
            p.push("recognizer/translator must declare at least one device".to_owned());
        }
        _ => {}
    }
    let mut seen = std::collections::BTreeSet::new();
    for model in &manifest.models {
        if !seen.insert(model.id.as_str()) {
            p.push(format!("duplicate model id {:?}", model.id));
        }
        p.extend(model_problems(model, builtin));
    }
    p
}

pub fn validate_manifest(manifest: &PluginManifest, builtin: bool) -> Result<(), ManifestError> {
    let problems = manifest_problems(manifest, builtin);
    if problems.is_empty() {
        Ok(())
    } else {
        Err(ManifestError {
            plugin_id: manifest.id.clone(),
            problems,
        })
    }
}
