//! Structured Telemetry & Tracing Subsystem for Panadiga
//!
//! Powered by standard Rust `tracing`, `tracing-subscriber`, `opentelemetry`,
//! `opentelemetry_sdk`, and `tracing-opentelemetry`:
//! - All diagnostic events stream cleanly to `stderr` (never corrupting CLI `--json` or MCP stdio JSON-RPC).
//! - Dynamic runtime log-level reloading via `tracing_subscriber::reload::Handle`.
//! - OpenTelemetry trace provider capturing spans and frontend Pino UI events in a single unified Rust pipeline.

use std::fmt;
use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use opentelemetry::trace::TracerProvider as _;
use opentelemetry_sdk::trace::SdkTracerProvider;
use tracing::Subscriber;
use tracing_subscriber::layer::{Context, SubscriberExt};
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer, reload};

/// Log severity levels ordered by verbosity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum LogLevel {
    Off = 0,
    Error = 1,
    Warn = 2,
    Info = 3,
    Debug = 4,
    Trace = 5,
}

impl LogLevel {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Off => "OFF",
            Self::Error => "ERROR",
            Self::Warn => "WARN",
            Self::Info => "INFO",
            Self::Debug => "DEBUG",
            Self::Trace => "TRACE",
        }
    }

    pub const fn as_filter_directive(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Error => "error",
            Self::Warn => "warn",
            Self::Info => "info",
            Self::Debug => "debug",
            Self::Trace => "trace",
        }
    }

    pub fn parse_lossy(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "off" | "silent" => Self::Off,
            "error" => Self::Error,
            "warn" | "warning" => Self::Warn,
            "debug" => Self::Debug,
            "trace" => Self::Trace,
            _ => Self::Info,
        }
    }
}

static CURRENT_LEVEL: AtomicU8 = AtomicU8::new(LogLevel::Info as u8);
static RAW_STDOUT_ENABLED: AtomicBool = AtomicBool::new(true);

type LogSink = Arc<dyn Fn(LogLevel, &str) + Send + Sync + 'static>;
static CUSTOM_SINK: Mutex<Option<LogSink>> = Mutex::new(None);
static RELOAD_HANDLE: OnceLock<reload::Handle<EnvFilter, tracing_subscriber::Registry>> =
    OnceLock::new();
static OTEL_PROVIDER: OnceLock<SdkTracerProvider> = OnceLock::new();

fn with_custom_sink<R>(f: impl FnOnce(&mut Option<LogSink>) -> R) -> R {
    let mut guard = CUSTOM_SINK.lock().unwrap_or_else(|e| e.into_inner());
    f(&mut guard)
}

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
        let Some(sink) = with_custom_sink(|slot| slot.clone()) else {
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

/// Initializes the global `tracing` + `tracing-subscriber` + `opentelemetry_sdk` telemetry pipeline once.
pub fn init_telemetry(default_level: LogLevel) {
    if RELOAD_HANDLE.get().is_some() {
        set_level(default_level);
        return;
    }

    let filter = EnvFilter::try_from_env("PANADIGA_LOG")
        .unwrap_or_else(|_| EnvFilter::new(default_level.as_filter_directive()));
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
        let _ = handle.reload(EnvFilter::new(level.as_filter_directive()));
    }
}

/// Retrieves the current active telemetry log level.
pub fn get_level() -> LogLevel {
    match CURRENT_LEVEL.load(Ordering::Acquire) {
        0 => LogLevel::Off,
        1 => LogLevel::Error,
        2 => LogLevel::Warn,
        3 => LogLevel::Info,
        4 => LogLevel::Debug,
        _ => LogLevel::Trace,
    }
}

/// Enables or suppresses unprefixed CLI table/banner emission to stdout (disabled automatically for MCP / `--json -`).
pub fn set_raw_output_enabled(enabled: bool) {
    RAW_STDOUT_ENABLED.store(enabled, Ordering::Release);
}

/// Returns true if messages of the specified level should be emitted.
#[inline]
pub fn is_enabled(level: LogLevel) -> bool {
    level as u8 <= CURRENT_LEVEL.load(Ordering::Relaxed)
}

/// Sets a custom callback sink for intercepting `tracing` events.
pub fn set_sink<F>(sink: F)
where
    F: Fn(LogLevel, &str) + Send + Sync + 'static,
{
    let wrapped: LogSink = Arc::new(sink);
    with_custom_sink(|slot| {
        *slot = Some(wrapped);
    });
}

/// Clears any registered custom sink.
pub fn reset_sink() {
    with_custom_sink(|slot| {
        *slot = None;
    });
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

/// Emits a formatted diagnostic message through Rust `tracing`.
pub fn log(level: LogLevel, args: fmt::Arguments) {
    ensure_telemetry_initialized();
    if !is_enabled(level) {
        return;
    }
    match level {
        LogLevel::Error => tracing::error!("{args}"),
        LogLevel::Warn => tracing::warn!("{args}"),
        LogLevel::Info => tracing::info!("{args}"),
        LogLevel::Debug => tracing::debug!("{args}"),
        LogLevel::Trace => tracing::trace!("{args}"),
        LogLevel::Off => {}
    }
}

/// Emits raw CLI presentation output (or routes through custom sink / stderr when stdout is reserved).
pub fn raw(args: fmt::Arguments) {
    if !is_enabled(LogLevel::Info) {
        return;
    }
    let text = format!("{args}");
    if let Some(sink) = with_custom_sink(|slot| slot.clone()) {
        sink(LogLevel::Info, &text);
        return;
    }
    if !RAW_STDOUT_ENABLED.load(Ordering::Acquire) {
        tracing::debug!(target: "panadiga_raw", "{text}");
        return;
    }

    let stdout = io::stdout();
    let mut lock = stdout.lock();
    let _ = writeln!(lock, "{text}");
}

#[macro_export]
macro_rules! __panadiga_log_dispatch {
    ($level:expr, $($arg:tt)*) => {
        $crate::logger::log($level, format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => {
        $crate::__panadiga_log_dispatch!($crate::logger::LogLevel::Info, $($arg)*)
    };
}

#[macro_export]
macro_rules! log_warn {
    ($($arg:tt)*) => {
        $crate::__panadiga_log_dispatch!($crate::logger::LogLevel::Warn, $($arg)*)
    };
}

#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => {
        $crate::__panadiga_log_dispatch!($crate::logger::LogLevel::Error, $($arg)*)
    };
}

#[macro_export]
macro_rules! log_debug {
    ($($arg:tt)*) => {
        $crate::__panadiga_log_dispatch!($crate::logger::LogLevel::Debug, $($arg)*)
    };
}

#[macro_export]
macro_rules! log_trace {
    ($($arg:tt)*) => {
        $crate::__panadiga_log_dispatch!($crate::logger::LogLevel::Trace, $($arg)*)
    };
}

#[macro_export]
macro_rules! log_println {
    () => {
        $crate::logger::raw(format_args!(""))
    };
    ($($arg:tt)*) => {
        $crate::logger::raw(format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_raw {
    ($($arg:tt)*) => {
        $crate::logger::raw(format_args!($($arg)*))
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;

    #[test]
    fn test_logger_levels_and_filtering() {
        init_telemetry(LogLevel::Info);
        set_level(LogLevel::Warn);
        assert!(!is_enabled(LogLevel::Info));
        assert!(is_enabled(LogLevel::Warn));
        assert!(is_enabled(LogLevel::Error));

        set_level(LogLevel::Info);
        assert!(is_enabled(LogLevel::Info));
    }

    #[test]
    fn test_logger_custom_sink_and_tracing_bridge() {
        init_telemetry(LogLevel::Info);
        let captured = Arc::new(AtomicBool::new(false));
        let cap_clone = Arc::clone(&captured);

        set_sink(move |_level, msg| {
            if msg.contains("PANADIGA_SINK_TEST") {
                cap_clone.store(true, Ordering::Release);
            }
        });

        raw(format_args!("PANADIGA_SINK_TEST_MESSAGE"));
        assert!(captured.load(Ordering::Acquire));

        reset_sink();
    }
}
