use super::*;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

#[test]
fn test_logger_levels_and_filtering() {
    let _warn_guard = with_thread_log_level(LogLevel::Warn);
    assert!(!is_enabled(LogLevel::Info));
    assert!(is_enabled(LogLevel::Warn));
    assert!(is_enabled(LogLevel::Error));
    assert_eq!(get_level(), LogLevel::Warn);

    {
        let _info_guard = with_thread_log_level(LogLevel::Info);
        assert!(is_enabled(LogLevel::Info));
        assert_eq!(get_level(), LogLevel::Info);
    }

    assert!(!is_enabled(LogLevel::Info));
}

#[test]
fn test_logger_custom_sink_and_tracing_bridge() {
    ensure_telemetry_initialized();
    let _level_guard = with_thread_log_level(LogLevel::Info);
    let captured = Arc::new(AtomicBool::new(false));
    let cap_clone = Arc::clone(&captured);

    let _sink_guard = with_thread_sink(move |_level, msg| {
        if msg.contains("PANADIGA_SINK_TEST") {
            cap_clone.store(true, Ordering::Release);
        }
    });

    raw(format_args!("PANADIGA_SINK_TEST_MESSAGE"));
    assert!(captured.load(Ordering::Acquire));
}

#[test]
fn test_env_filter_silences_lopdf_glyph_warnings() {
    let info_filter = logger_tracing::build_env_filter(LogLevel::Info).to_string();
    assert!(
        info_filter.contains("lopdf=error"),
        "expected lopdf=error directive in {info_filter}"
    );
    assert!(info_filter.contains("info"));

    let off_filter = logger_tracing::build_env_filter(LogLevel::Off).to_string();
    assert_eq!(off_filter, "off");

    let resolved = logger_tracing::resolve_startup_filter(LogLevel::Warn).to_string();
    assert!(!resolved.is_empty());
}
