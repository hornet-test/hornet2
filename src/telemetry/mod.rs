mod config;
mod shutdown;

pub use config::TelemetryConfig;
pub use shutdown::TelemetryGuard;

use opentelemetry::KeyValue;
use opentelemetry::trace::TracerProvider as _;
use opentelemetry_otlp::{WithExportConfig, WithHttpConfig};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

pub fn init_telemetry() -> crate::Result<TelemetryGuard> {
    let config = TelemetryConfig::from_env();

    if config.enabled {
        // Try to initialize with OpenTelemetry, but fall back to stdout-only if it fails
        if let Err(e) = init_with_otel(&config) {
            eprintln!(
                "Failed to initialize OpenTelemetry: {}. Falling back to stdout-only logging.",
                e
            );
            init_stdout_only();
        }
    } else {
        init_stdout_only();
    }

    Ok(TelemetryGuard::new())
}

fn init_with_otel(config: &TelemetryConfig) -> crate::Result<()> {
    // Create HTTP client explicitly
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| {
            crate::HornetError::ValidationError(format!("Failed to build HTTP client: {}", e))
        })?;

    // Initialize OTLP exporter with HTTP client
    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_http()
        .with_http_client(client)
        .with_endpoint(&config.endpoint)
        .with_protocol(opentelemetry_otlp::Protocol::HttpBinary)
        .with_timeout(std::time::Duration::from_secs(5))
        .with_headers(config.headers.clone())
        .build()
        .map_err(|e| {
            eprintln!(
                "Failed to build OTLP exporter: {}. Falling back to stdout-only logging.",
                e
            );
            crate::HornetError::ValidationError(format!("Failed to build OTLP exporter: {}", e))
        })?;

    // Create resource with service metadata
    let resource = opentelemetry_sdk::Resource::builder_empty()
        .with_service_name(config.service_name.clone())
        .with_attributes([KeyValue::new("service.version", env!("CARGO_PKG_VERSION"))])
        .build();

    // Create tracer provider
    let provider = opentelemetry_sdk::trace::SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .with_resource(resource)
        .build();

    // Get tracer before setting as global provider
    let tracer = provider.tracer("hornet2");

    // Set as global provider
    opentelemetry::global::set_tracer_provider(provider);

    // Set up tracing layers
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("hornet2=info,tower_http=debug"));

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_thread_ids(true)
        .with_line_number(true);

    let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .with(otel_layer)
        .init();

    tracing::info!(
        "OpenTelemetry initialized with endpoint: {}",
        config.endpoint
    );
    Ok(())
}

fn init_stdout_only() {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("hornet2=info,tower_http=debug"));

    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_target(true)
        .with_thread_ids(true)
        .with_line_number(true)
        .init();

    tracing::info!("Tracing initialized (stdout only, OpenTelemetry disabled)");
}
