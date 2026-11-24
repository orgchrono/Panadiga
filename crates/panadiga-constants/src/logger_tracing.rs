//! `tracing` + `tracing-subscriber` + `opentelemetry_sdk` initialization and sink dispatch layer.

use std::fmt;
use std::io;
use std::sync::OnceLock;
use std::sync::atomic::Ordering;

use opentelemetry::trace::TracerProvider as _;
use opentelemetry_sdk::trace::SdkTracerProvider;
use tracing::Subscriber;
use tracing_subscriber::layer::{Context, SubscriberExt};
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer, reload};

use super::{CURRENT_LEVEL, LogLevel, active_sink, get_level, is_enabled};

static RELOAD_HANDLE: OnceLock<reload::Handle<EnvFilter, tracing_subscriber::Registry>> =
    OnceLock::new();
static OTEL_PROVIDER: OnceLock<SdkTracerProvider> = OnceLock::new();

#[derive(Default)]
struct MessageVisitor {
    message: Option<String>,
}

impl tracing::field::Visit for MessageVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn fmt::Debug) {
        if field.name() == "message" {
            self.message = Some(format!("{value:?}"));
        }
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.message = Some(value.to_string());
        }
    }
}

struct CustomSinkLayer;

impl<S> Layer<S> for CustomSinkLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
        let Some(sink) = active_sink() else {
            return;
        };
        let level = match *event.metadata().level() {
            tracing::Level::ERROR => LogLevel::Error,
            tracing::Level::WARN => LogLevel::Warn,
            tracing::Level::INFO => LogLevel::Info,
            tracing::Level::DEBUG => LogLevel::Debug,
            tracing::Level::TRACE => LogLevel::Trace,
        };
        if !is_enabled(level) {
            return;
        }
        let mut visitor = MessageVisitor::default();
        event.record(&mut visitor);
        let msg = visitor
            .message
            .unwrap_or_else(|| event.metadata().name().to_string());
        sink(level, &msg);
    }
}

const LOPDF_SILENCE_DIRECTIVE: &str = "lopdf=error";

pub(crate) fn build_env_filter(level: LogLevel) -> EnvFilter {
    if level == LogLevel::Off {
        EnvFilter::new("off")
    } else {
        EnvFilter::new(format!(
            "{},{LOPDF_SILENCE_DIRECTIVE}",
            level.as_filter_directive()
        ))
    }
}

pub(crate) fn resolve_startup_filter(default_level: LogLevel) -> EnvFilter {
    let Ok(raw_env) = std::env::var("PANADIGA_LOG") else {
        return build_env_filter(default_level);
    };
    let Ok(mut filter) = EnvFilter::try_new(&raw_env) else {
        return build_env_filter(default_level);
    };
    if !raw_env.contains("lopdf")
        && let Ok(directive) = LOPDF_SILENCE_DIRECTIVE.parse()
    {
        filter = filter.add_directive(directive);
    }
    filter
}

/// Initializes the global `tracing` + `tracing-subscriber` + `opentelemetry_sdk` telemetry pipeline once.
pub fn init_telemetry(default_level: LogLevel) {
    if RELOAD_HANDLE.get().is_some() {
        set_level(default_level);
        return;
    }

    let filter = resolve_startup_filter(default_level);
    let (filter_layer, reload_handle) = reload::Layer::new(filter);

    let provider = SdkTracerProvider::builder().build();
    let tracer = provider.tracer("panadiga-core");
    let _ = OTEL_PROVIDER.set(provider);
    let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_writer(io::stderr)
        .with_target(false)
        .compact();

    let registry = tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .with(otel_layer)
        .with(CustomSinkLayer);

    let _ = registry.try_init();
    let _ = RELOAD_HANDLE.set(reload_handle);
    CURRENT_LEVEL.store(default_level as u8, Ordering::Release);
}

/// Ensures telemetry subscriber is initialized on first log emission.
#[inline]
pub fn ensure_telemetry_initialized() {
    if RELOAD_HANDLE.get().is_none() {
        init_telemetry(get_level());
    }
}

/// Dynamically updates the active telemetry log level across Rust `tracing` and custom sinks.
pub fn set_level(level: LogLevel) {
    CURRENT_LEVEL.store(level as u8, Ordering::Release);
    if let Some(handle) = RELOAD_HANDLE.get() {
        let _ = handle.reload(build_env_filter(level));
    }
}

/// Forwards a frontend Pino/OpenTelemetry log event into the canonical Rust `tracing` + OTel pipeline.
pub fn emit_frontend_telemetry(
    severity: &str,
    subsystem: &str,
    message: &str,
    details: Option<&str>,
) {
    ensure_telemetry_initialized();
    let span = tracing::info_span!("frontend_ui", subsystem = %subsystem);
    let _entered = span.enter();
    let detail_str = details.unwrap_or("");
    match LogLevel::parse_lossy(severity) {
        LogLevel::Error => {
            tracing::error!(subsystem = %subsystem, details = %detail_str, "{message}")
        }
        LogLevel::Warn => {
            tracing::warn!(subsystem = %subsystem, details = %detail_str, "{message}")
        }
        LogLevel::Debug => {
            tracing::debug!(subsystem = %subsystem, details = %detail_str, "{message}")
        }
        LogLevel::Trace => {
            tracing::trace!(subsystem = %subsystem, details = %detail_str, "{message}")
        }
        LogLevel::Info | LogLevel::Off => {
            tracing::info!(subsystem = %subsystem, details = %detail_str, "{message}");
        }
    }
}
