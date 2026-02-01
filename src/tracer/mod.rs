//! eBPF/OpenTelemetry trace collection and processing module.
//!
//! This module provides:
//! - OTLP receiver for collecting traces from OBI (OpenTelemetry eBPF Instrumentation)
//! - Span processing and classification (incoming vs outgoing)
//! - Persistent storage for trace sessions
//! - Semantic conventions compliance
//! - OpenAPI inference from trace data
//! - Arazzo workflow generation
//! - Stub generation for dependency APIs

pub mod config;
pub mod generators;
pub mod inference;
pub mod otlp_receiver;
pub mod semantic_conventions;
pub mod span_classifier;
pub mod store;
pub mod types;

pub use config::TracerConfig;
pub use generators::{ArazzoGenerator, StubFormat, StubGenerator};
pub use inference::{OpenApiGenerator, PathAnalyzer, SchemaInferrer};
pub use otlp_receiver::OtlpReceiver;
pub use span_classifier::SpanClassifier;
pub use store::TraceStore;
pub use types::*;
