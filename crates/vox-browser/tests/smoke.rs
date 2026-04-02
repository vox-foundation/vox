//! Manual / CI opt-in: `cargo test -p vox-browser open_about_blank -- --ignored`

#[tokio::test]
#[ignore = "Requires Chromium on PATH or VOX_CHROME_EXECUTABLE"]
async fn open_about_blank_and_close() {
    let eng = vox_browser::global_engine();
    let page_id = eng
        .open("about:blank", true)
        .await
        .expect("Browser::open (install Chrome/Chromium or set VOX_CHROME_EXECUTABLE)");
    eng.close(&page_id).await.expect("Browser::close");
}
