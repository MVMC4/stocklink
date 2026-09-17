//! Structured logging plus optional OTLP trace export. `init_tracing` is the
//! one thing every service's `main.rs` calls before doing anything else — a
//! panic or error on line one should still be logged, not lost because
//! tracing wasn't wired up yet.
//!
//! OTLP export is opt-in (`OTEL_ENABLED`, off by default): most local dev
//! and CI runs have nowhere to send spans, and initializing the exporter
//! against an unreachable collector shouldn't be a startup requirement.

use opentelemetry::trace::TracerProvider as _;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::trace::SdkTracerProvider;
use opentelemetry_sdk::Resource;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer};

use crate::config::ObservabilityConfig;

/// Holds the tracer provider alive for the process lifetime and flushes it
/// on drop — `main.rs` keeps this bound in a variable it doesn't otherwise
/// use, so spans generated right up to shutdown still get exported.
pub struct TracingGuard {
    provider: Option<SdkTracerProvider>,
}

impl Drop for TracingGuard {
    fn drop(&mut self) {
        if let Some(provider) = &self.provider {
            if let Err(err) = provider.shutdown() {
                eprintln!("tracer provider shutdown failed: {err}");
            }
        }
    }
}

pub fn init_tracing(config: &ObservabilityConfig) -> TracingGuard {
    let env_filter =
        EnvFilter::try_new(&config.log_filter).unwrap_or_else(|_| EnvFilter::new("info"));

    let fmt_layer = if config.json {
        tracing_subscriber::fmt::layer()
            .json()
            .with_target(true)
            .boxed()
    } else {
        tracing_subscriber::fmt::layer().with_target(true).boxed()
    };

    let provider = if config.otel.enabled {
        build_tracer_provider(config).ok()
    } else {
        None
    };

    let otel_layer = provider.as_ref().map(|provider| {
        let tracer = provider.tracer(config.otel.service_name.clone());
        tracing_opentelemetry::layer().with_tracer(tracer)
    });

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .with(otel_layer)
        .init();

    TracingGuard { provider }
}

fn build_tracer_provider(
    config: &ObservabilityConfig,
) -> Result<SdkTracerProvider, opentelemetry_otlp::ExporterBuildError> {
    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(config.otel.endpoint.clone())
        .build()?;

    let resource = Resource::builder()
        .with_service_name(config.otel.service_name.clone())
        .build();

    Ok(SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .with_resource(resource)
        .build())
}
