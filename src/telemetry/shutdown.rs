use opentelemetry::global;

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

        // Flush pending spans
        global::shutdown_tracer_provider();

        // Allow time for final flush
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
}
