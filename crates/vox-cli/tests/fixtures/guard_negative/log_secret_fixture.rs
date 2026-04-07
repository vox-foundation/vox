fn bad_log(access_token: &str) {
    tracing::info!("access_token={}", access_token);
}
