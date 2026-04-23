fn main() {
    let features = [
        ("gpu", std::env::var("CARGO_FEATURE_GPU").is_ok()),
        ("oratio", std::env::var("CARGO_FEATURE_ORATIO").is_ok()),
        (
            "oratio-mic",
            std::env::var("CARGO_FEATURE_ORATIO_MIC").is_ok(),
        ),
        (
            "script-execution",
            std::env::var("CARGO_FEATURE_SCRIPT_EXECUTION").is_ok(),
        ),
        (
            "mens-candle-cuda",
            std::env::var("CARGO_FEATURE_MENS_CANDLE_CUDA").is_ok(),
        ),
        ("cloud", std::env::var("CARGO_FEATURE_CLOUD").is_ok()),
        (
            "execution-api",
            std::env::var("CARGO_FEATURE_EXECUTION_API").is_ok(),
        ),
        (
            "stub-check",
            std::env::var("CARGO_FEATURE_STUB_CHECK").is_ok(),
        ),
        ("populi", std::env::var("CARGO_FEATURE_POPULI").is_ok()),
    ];
    let json = serde_json::to_string(
        &features
            .iter()
            .filter(|(_, on)| *on)
            .map(|(n, _)| *n)
            .collect::<Vec<_>>(),
    )
    .unwrap();
    println!("cargo:rustc-env=VOX_BUILD_FEATURES={json}");
}
