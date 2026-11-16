use std::error::Error as StdError;

/// Ошибка, которую плагин возвращает ядру.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum PluginError {
    #[error("model not available: {0}")]
    ModelNotFound(String),
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("invalid config: {0}")]
    InvalidConfig(String),
    #[error("unsupported: {0}")]
    Unsupported(String),
    #[error("engine error: {0}")]
    Engine(String),
    #[error("timeout")]
    Timeout,
    #[error(transparent)]
    Other(Box<dyn StdError + Send + Sync>),
}

/// Манифест плагина невалиден. Содержит все найденные проблемы, а не только первую.
#[derive(Debug, Clone, thiserror::Error)]
#[error("invalid manifest for plugin {plugin_id:?}: {}", problems.join("; "))]
pub struct ManifestError {
    pub plugin_id: String,
    pub problems: Vec<String>,
}
