#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum AudioError {
    #[error("WASAPI: {context}: {source}")]
    Wasapi {
        context: &'static str,
        #[source]
        source: wasapi::WasapiError,
    },
    #[error("Windows API: {context}: {source}")]
    Windows {
        context: &'static str,
        #[source]
        source: windows::core::Error,
    },
    #[error("audio device not found: {0}")]
    DeviceNotFound(String),
    #[error("no audio session found for process {0:?}")]
    ProcessNotFound(String),
    #[error("failed to start capture thread: {0}")]
    Spawn(#[source] std::io::Error),
    #[error("capture thread exited before initialization")]
    ThreadExited,
    #[error("capture thread panicked")]
    ThreadPanicked,
}

/// Добавляет к ошибке WinAPI/WASAPI контекст: какой вызов не удался.
pub(crate) trait Context<T> {
    fn ctx(self, context: &'static str) -> Result<T, AudioError>;
}

impl<T> Context<T> for Result<T, wasapi::WasapiError> {
    fn ctx(self, context: &'static str) -> Result<T, AudioError> {
        self.map_err(|source| AudioError::Wasapi { context, source })
    }
}

impl<T> Context<T> for windows::core::Result<T> {
    fn ctx(self, context: &'static str) -> Result<T, AudioError> {
        self.map_err(|source| AudioError::Windows { context, source })
    }
}
