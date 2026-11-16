//! Pure `std.plot` chart plans. Building a plan stores frame, chart kind, labels, and does not
//! call the Python worker.

use std::sync::atomic::{AtomicU32, Ordering};

use crate::eval::frame_plan::FramePlan;

static PLOT_WORKER_CALLS: AtomicU32 = AtomicU32::new(0);

#[must_use]
pub fn plot_worker_calls() -> u32 {
    PLOT_WORKER_CALLS.load(Ordering::SeqCst)
}

/// Pure `std.plot` figure specification rendered inside the Python worker on `.save(png)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlotPlan {
    chart_kind: &'static str,
    frame: FramePlan,
    x_col: Option<String>,
    y_col: Option<String>,
    title_text: Option<String>,
    legend_enabled: Option<bool>,
    worker_calls: u32,
}

impl PlotPlan {
    fn new(chart_kind: &'static str, frame: FramePlan) -> Self {
        Self {
            chart_kind,
            frame,
            x_col: None,
            y_col: None,
            title_text: None,
            legend_enabled: None,
            worker_calls: plot_worker_calls(),
        }
    }

    #[must_use]
    pub fn line(frame: FramePlan) -> Self {
        Self::new("line", frame)
    }

    #[must_use]
    pub fn scatter(frame: FramePlan) -> Self {
        Self::new("scatter", frame)
    }

    #[must_use]
    pub fn bar(frame: FramePlan) -> Self {
        Self::new("bar", frame)
    }

    #[must_use]
    pub fn hist(frame: FramePlan) -> Self {
        Self::new("hist", frame)
    }

    #[must_use]
    pub fn x(&self, col: &str) -> Self {
        let mut next = self.clone();
        next.x_col = Some(col.to_string());
        next
    }

    #[must_use]
    pub fn y(&self, col: &str) -> Self {
        let mut next = self.clone();
        next.y_col = Some(col.to_string());
        next
    }

    #[must_use]
    pub fn title(&self, text: &str) -> Self {
        let mut next = self.clone();
        next.title_text = Some(text.to_string());
        next
    }

    #[must_use]
    pub fn legend(&self, enabled: bool) -> Self {
        let mut next = self.clone();
        next.legend_enabled = Some(enabled);
        next
    }

    #[must_use]
    pub fn chart_kind(&self) -> &'static str {
        self.chart_kind
    }

    #[must_use]
    pub fn frame(&self) -> &FramePlan {
        &self.frame
    }

    #[must_use]
    pub fn x_col(&self) -> Option<&str> {
        self.x_col.as_deref()
    }

    #[must_use]
    pub fn y_col(&self) -> Option<&str> {
        self.y_col.as_deref()
    }

    #[must_use]
    pub fn title_text(&self) -> Option<&str> {
        self.title_text.as_deref()
    }

    #[must_use]
    pub fn legend_enabled(&self) -> Option<bool> {
        self.legend_enabled
    }

    #[must_use]
    pub fn worker_calls(&self) -> u32 {
        self.worker_calls
    }
}
