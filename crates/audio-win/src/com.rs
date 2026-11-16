//! Инициализация COM на текущем потоке (RAII).

/// Держит COM (MTA) инициализированным, пока жив. Если поток уже был в STA, COM всё равно
/// пригоден к использованию, но парный `CoUninitialize` в этом случае не вызывается.
#[derive(Debug)]
pub(crate) struct ComGuard {
    owned: bool,
}

impl ComGuard {
    pub(crate) fn init_mta() -> Self {
        // S_OK и S_FALSE (уже инициализирован) оба требуют парного CoUninitialize.
        // RPC_E_CHANGED_MODE (поток в STA) — не наш, не трогаем.
        Self {
            owned: wasapi::initialize_mta().is_ok(),
        }
    }
}

impl Drop for ComGuard {
    fn drop(&mut self) {
        if self.owned {
            wasapi::deinitialize();
        }
    }
}
