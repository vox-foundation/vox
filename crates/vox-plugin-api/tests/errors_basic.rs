use vox_plugin_api::errors::LogLevel;

#[test]
fn log_levels_round_trip_through_serde() {
    let levels = [
        LogLevel::Trace,
        LogLevel::Debug,
        LogLevel::Info,
        LogLevel::Warn,
        LogLevel::Error,
    ];
    for l in levels {
        let json = serde_json::to_string(&l).unwrap();
        let back: LogLevel = serde_json::from_str(&json).unwrap();
        assert_eq!(l, back);
    }
}
