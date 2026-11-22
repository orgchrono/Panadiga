//! Structured Telemetry & Tracing Subsystem for Panadiga
//!
//! Powered by standard Rust `tracing`, `tracing-subscriber`, `opentelemetry`,
//! `opentelemetry_sdk`, and `tracing-opentelemetry`:
//! - All diagnostic events stream cleanly to `stderr` (never corrupting CLI `--json` or MCP stdio JSON-RPC).
//! - Dynamic runtime log-level reloading via `tracing_subscriber::reload::Handle`.
//! - OpenTelemetry trace provider capturing spans and frontend Pino UI events in a single unified Rust pipeline.

use std::cell::{Cell, RefCell};
use std::fmt;
use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{Arc, Mutex};

#[path = "logger_tracing.rs"]
mod logger_tracing;

pub use logger_tracing::{
    emit_frontend_telemetry, ensure_telemetry_initialized, init_telemetry, set_level,
};

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

thread_local! {
    static THREAD_LEVEL_OVERRIDE: Cell<Option<u8>> = const { Cell::new(None) };
    static THREAD_SINK_OVERRIDE: RefCell<Option<LogSink>> = const { RefCell::new(None) };
}

fn with_custom_sink<R>(f: impl FnOnce(&mut Option<LogSink>) -> R) -> R {
    let mut guard = CUSTOM_SINK.lock().unwrap_or_else(|e| e.into_inner());
    f(&mut guard)
}

#[inline]
fn effective_level_raw() -> u8 {
    THREAD_LEVEL_OVERRIDE
        .with(Cell::get)
        .unwrap_or_else(|| CURRENT_LEVEL.load(Ordering::Acquire))
}

fn active_sink() -> Option<LogSink> {
    THREAD_SINK_OVERRIDE
        .with(|slot| slot.borrow().clone())
        .or_else(|| with_custom_sink(|slot| slot.clone()))
}

/// RAII guard overriding the telemetry level exclusively on the calling thread.
pub struct ThreadLogLevelGuard {
    previous: Option<u8>,
}

impl Drop for ThreadLogLevelGuard {
    fn drop(&mut self) {
        let prev = self.previous;
        THREAD_LEVEL_OVERRIDE.with(|slot| slot.set(prev));
    }
}

/// Overrides the telemetry [`LogLevel`] for the current thread until the returned guard drops.
#[must_use]
pub fn with_thread_log_level(level: LogLevel) -> ThreadLogLevelGuard {
    let previous = THREAD_LEVEL_OVERRIDE.with(|slot| {
        let prev = slot.get();
        slot.set(Some(level as u8));
        prev
    });
    ThreadLogLevelGuard { previous }
}

/// RAII guard overriding the custom log sink exclusively on the calling thread.
pub struct ThreadLogSinkGuard {
    previous: Option<LogSink>,
}

impl Drop for ThreadLogSinkGuard {
    fn drop(&mut self) {
        let prev = self.previous.take();
        THREAD_SINK_OVERRIDE.with(|slot| {
            *slot.borrow_mut() = prev;
        });
    }
}

/// Installs a thread-local log sink for deterministic isolated telemetry verification.
#[must_use]
pub fn with_thread_sink<F>(sink: F) -> ThreadLogSinkGuard
where
    F: Fn(LogLevel, &str) + Send + Sync + 'static,
{
    let wrapped: LogSink = Arc::new(sink);
    let previous = THREAD_SINK_OVERRIDE.with(|slot| slot.borrow_mut().replace(wrapped));
    ThreadLogSinkGuard { previous }
}

/// Retrieves the current active telemetry log level (respecting thread-local scoped overrides).
pub fn get_level() -> LogLevel {
    match effective_level_raw() {
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
    level as u8 <= effective_level_raw()
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
    if let Some(sink) = active_sink() {
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
#[path = "logger_tests.rs"]
mod tests;
