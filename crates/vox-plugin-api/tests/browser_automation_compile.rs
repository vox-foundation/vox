use abi_stable::{erased_types::TD_Opaque, std_types::*};
use vox_plugin_api::extensions::browser_automation::{BrowserAutomation, BrowserAutomation_TO};

struct DummyBrowser;

impl BrowserAutomation for DummyBrowser {
    fn open(&self, _url: RStr<'_>, _headless: bool) -> RResult<RString, RBoxError> {
        RResult::ROk(RString::from("page-dummy"))
    }
    fn goto(&self, _page_id: RStr<'_>, _url: RStr<'_>) -> RResult<(), RBoxError> {
        RResult::ROk(())
    }
    fn click(&self, _page_id: RStr<'_>, _target: RStr<'_>) -> RResult<(), RBoxError> {
        RResult::ROk(())
    }
    fn fill(
        &self,
        _page_id: RStr<'_>,
        _target: RStr<'_>,
        _value: RStr<'_>,
    ) -> RResult<(), RBoxError> {
        RResult::ROk(())
    }
    fn wait_for(
        &self,
        _page_id: RStr<'_>,
        _target: RStr<'_>,
        _timeout_secs: u64,
    ) -> RResult<(), RBoxError> {
        RResult::ROk(())
    }
    fn text(&self, _page_id: RStr<'_>, _target: RStr<'_>) -> RResult<RString, RBoxError> {
        RResult::ROk(RString::from(""))
    }
    fn html(&self, _page_id: RStr<'_>, _target: RStr<'_>) -> RResult<RString, RBoxError> {
        RResult::ROk(RString::from(""))
    }
    fn screenshot_bytes(&self, _page_id: RStr<'_>) -> RResult<RVec<u8>, RBoxError> {
        RResult::ROk(RVec::new())
    }
    fn screenshot(&self, _page_id: RStr<'_>, path: RStr<'_>) -> RResult<RString, RBoxError> {
        RResult::ROk(RString::from(path.as_str()))
    }
    fn visible_text_summary(
        &self,
        _page_id: RStr<'_>,
        _max_chars: u64,
    ) -> RResult<RString, RBoxError> {
        RResult::ROk(RString::from(""))
    }
    fn ax_tree(&self, _page_id: RStr<'_>) -> RResult<RString, RBoxError> {
        RResult::ROk(RString::from("[]"))
    }
    fn close(&self, _page_id: RStr<'_>) -> RResult<(), RBoxError> {
        RResult::ROk(())
    }
}

#[test]
fn dummy_browser_constructs() {
    let _: BrowserAutomation_TO<'static, RBox<()>> =
        BrowserAutomation_TO::from_value(DummyBrowser, TD_Opaque);
}
