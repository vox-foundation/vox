//! Test egress guard (Task 9c), pulled into every vox-search integration test
//! binary with `mod common;`.
//!
//! With `web-scrape` feature-unified on (any workspace-wide build),
//! `WebSearchDispatcher` scrapes every kept hit's URL. Several tests keep hits
//! whose URLs the provider clients hard-code to real hosts (Wikipedia always
//! yields `https://en.wikipedia.org/wiki/…`, arXiv `https://arxiv.org/abs/…`),
//! so no wiremock body can make them local. This ctor routes every non-loopback
//! request through a dead local proxy, so such a fetch fails fast on the
//! machine (scraping then keeps the engine snippet) and no real host is ever
//! contacted, not even by DNS. Loopback (the wiremock servers) is exempt.
#[ctor::ctor(unsafe)]
#[allow(unsafe_code)] // SAFETY: ctors run pre-main, single-threaded, before any test thread.
fn block_non_loopback_egress() {
    unsafe {
        for key in [
            "HTTP_PROXY",
            "HTTPS_PROXY",
            "ALL_PROXY",
            "http_proxy",
            "https_proxy",
            "all_proxy",
        ] {
            std::env::set_var(key, "http://127.0.0.1:9");
        }
        for key in ["NO_PROXY", "no_proxy"] {
            std::env::set_var(key, "127.0.0.1,localhost,::1");
        }
    }
}
