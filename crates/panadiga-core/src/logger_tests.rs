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
