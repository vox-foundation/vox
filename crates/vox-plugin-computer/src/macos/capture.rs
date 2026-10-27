//! Per-window and display capture with ScreenCaptureKit
//! (`SCScreenshotManager`), which reads occluded and background windows
//! without raising them.

use std::ptr::NonNull;
use std::sync::mpsc;
use std::time::Duration;

use block2::RcBlock;
use objc2::AllocAnyThread;
use objc2::rc::Retained;
use objc2_core_foundation::{CFRetained, CGPoint, CGRect, CGSize};
use objc2_core_graphics::{
    CGDataProvider, CGDisplayBounds, CGDisplayCopyDisplayMode, CGDisplayMode, CGImage,
    CGImageByteOrderInfo, CGMainDisplayID, CGPreflightScreenCaptureAccess,
};
use objc2_foundation::{NSArray, NSError};
use objc2_screen_capture_kit::{
    SCContentFilter, SCScreenshotManager, SCShareableContent, SCStreamConfiguration,
};

use crate::computer::Capture;
use crate::model::{Frame, Rect, encode_png, fit_size};

const TIMEOUT: Duration = Duration::from_secs(5);

/// Moves an Objective-C result out of a completion handler. The receiving
/// thread is the only user once sent.
struct Sendable<T>(T);
// SAFETY: values are handed off exactly once and never shared.
unsafe impl<T> Send for Sendable<T> {}

fn describe(err: *mut NSError) -> String {
    // SAFETY: completion handlers pass either null or a valid NSError.
    match unsafe { err.as_ref() } {
        Some(e) => e.localizedDescription().to_string(),
        None => "no result".into(),
    }
}

pub fn require_permission() -> Result<(), String> {
    if CGPreflightScreenCaptureAccess() {
        Ok(())
    } else {
        Err("screen_recording_denied: grant Screen Recording to the host app (terminal, IDE, \
             or vox daemon) in System Settings > Privacy & Security > Screen & System Audio Recording"
            .into())
    }
}

fn shareable() -> Result<Retained<SCShareableContent>, String> {
    // ScreenCaptureKit aborts the process (CGS_REQUIRE_INIT) unless a CoreGraphics
    // session exists; querying the display mode establishes one.
    let _ = native_scale();
    let (tx, rx) = mpsc::channel();
    let block = RcBlock::new(move |content: *mut SCShareableContent, err: *mut NSError| {
        // SAFETY: the handler receives an autoreleased object; retain it.
        let res = unsafe { Retained::retain(content) }
            .map(Sendable)
            .ok_or_else(|| describe(err));
        let _ = tx.send(res);
    });
    unsafe {
        SCShareableContent::getShareableContentExcludingDesktopWindows_onScreenWindowsOnly_completionHandler(
            true, true, &block,
        );
    }
    match rx.recv_timeout(TIMEOUT) {
        Ok(Ok(content)) => Ok(content.0),
        Ok(Err(e)) => Err(format!("backend_error: shareable content: {e}")),
        Err(_) => Err("backend_error: ScreenCaptureKit did not answer".into()),
    }
}

/// Pixels per point of the main display (2.0 on Retina).
pub fn native_scale() -> f64 {
    let mode: Option<CFRetained<CGDisplayMode>> = CGDisplayCopyDisplayMode(CGMainDisplayID());
    let mode = mode.as_deref();
    let points = CGDisplayMode::width(mode);
    if points == 0 {
        1.0
    } else {
        CGDisplayMode::pixel_width(mode) as f64 / points as f64
    }
}

fn capture_image(
    filter: &SCContentFilter,
    config: &SCStreamConfiguration,
) -> Result<CFRetained<CGImage>, String> {
    let (tx, rx) = mpsc::channel();
    let block = RcBlock::new(move |image: *mut CGImage, err: *mut NSError| {
        // SAFETY: the handler's image is borrowed for the call; retain it.
        let res = NonNull::new(image)
            .map(|p| Sendable(unsafe { CFRetained::retain(p) }))
            .ok_or_else(|| describe(err));
        let _ = tx.send(res);
    });
    unsafe {
        SCScreenshotManager::captureImageWithFilter_configuration_completionHandler(
            filter,
            config,
            Some(&block),
        );
    }
    match rx.recv_timeout(TIMEOUT) {
        Ok(Ok(img)) => Ok(img.0),
        Ok(Err(e)) => Err(format!("backend_error: capture: {e}")),
        Err(_) => Err("backend_error: capture timed out".into()),
    }
}

fn to_png(image: &CGImage) -> Result<(Vec<u8>, u32, u32), String> {
    let img = Some(image);
    if CGImage::bits_per_pixel(img) != 32 {
        return Err("backend_error: unexpected pixel format".into());
    }
    let (w, h) = (CGImage::width(img) as u32, CGImage::height(img) as u32);
    let provider: Option<CFRetained<CGDataProvider>> = CGImage::data_provider(img);
    let data = CGDataProvider::data(provider.as_deref()).ok_or("backend_error: no pixel data")?;
    let bgra = CGImage::byte_order_info(img) == CGImageByteOrderInfo::Order32Little;
    let png = encode_png(&data.to_vec(), w, h, CGImage::bytes_per_row(img), bgra)?;
    Ok((png, w, h))
}

/// Capture `window_id` (global frame `window`) or the main display; `region`
/// is in global points.
pub fn screenshot(
    window_id: Option<u32>,
    region: Option<Rect>,
    max_long_edge: u32,
) -> Result<Capture, String> {
    require_permission()?;
    let content = shareable()?;
    let (filter, base) = match window_id {
        Some(id) => {
            let windows = unsafe { content.windows() };
            let window = windows
                .iter()
                .find(|w| unsafe { w.windowID() } == id)
                .ok_or("backend_error: window is no longer capturable")?;
            let frame = unsafe { window.frame() };
            let filter = unsafe {
                SCContentFilter::initWithDesktopIndependentWindow(SCContentFilter::alloc(), &window)
            };
            (filter, frame)
        }
        None => {
            let main = CGMainDisplayID();
            let displays = unsafe { content.displays() };
            let display = displays
                .iter()
                .find(|d| unsafe { d.displayID() } == main)
                .ok_or("backend_error: main display not capturable")?;
            let filter = unsafe {
                SCContentFilter::initWithDisplay_excludingWindows(
                    SCContentFilter::alloc(),
                    &display,
                    &NSArray::new(),
                )
            };
            (filter, CGDisplayBounds(main))
        }
    };
    let src = region.unwrap_or(Rect {
        x: base.origin.x,
        y: base.origin.y,
        w: base.size.width,
        h: base.size.height,
    });
    let (w, h, scale) = fit_size(src.w, src.h, native_scale(), max_long_edge);
    let config = unsafe { SCStreamConfiguration::new() };
    unsafe {
        config.setWidth(w as usize);
        config.setHeight(h as usize);
        config.setShowsCursor(false);
        config.setIgnoreShadowsSingleWindow(true);
        if region.is_some() {
            config.setSourceRect(CGRect {
                origin: CGPoint {
                    x: src.x - base.origin.x,
                    y: src.y - base.origin.y,
                },
                size: CGSize {
                    width: src.w,
                    height: src.h,
                },
            });
        }
    }
    let image = capture_image(&filter, &config)?;
    let (png, width, height) = to_png(&image)?;
    let scale = if src.w > 0.0 {
        f64::from(width) / src.w
    } else {
        scale
    };
    Ok(Capture {
        png,
        frame: Frame {
            origin_x: src.x,
            origin_y: src.y,
            scale,
            width,
            height,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Preflight never prompts; a denial must name the permission and where to grant it.
    #[test]
    fn require_permission_is_ok_or_names_the_missing_grant() {
        if let Err(e) = require_permission() {
            assert!(e.starts_with("screen_recording_denied:"), "{e}");
            assert!(e.contains("Screen Recording"), "{e}");
        }
    }
}
