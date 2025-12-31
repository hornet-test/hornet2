pub struct TelemetryGuard {
    _marker: (),
}

impl Default for TelemetryGuard {
    fn default() -> Self {
        Self::new()
    }
}

impl TelemetryGuard {
    pub fn new() -> Self {
        Self { _marker: () }
    }
}

impl Drop for TelemetryGuard {
    fn drop(&mut self) {
        tracing::info!("Shutting down telemetry...");

        // In OpenTelemetry 0.31+, shutdown is handled automatically
        // when the TracerProvider is dropped. We just need to allow
        // time for the final flush to complete.
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
}
