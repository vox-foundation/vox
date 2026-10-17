//! Container finishing. dagre sizes a container from its children only, so the seam pads the
//! engine's rect, adds a title band on top and re-wraps the title to the final width.

use super::text::{WRAP_CELLS, measure, wrap_at};
use super::{Half, LContainer, PlacedContainer, Rect};

/// Padding left, right and below the children of a container: 8 px.
const CONTAINER_PAD: Half = 16;

/// Space kept between a title and each side of its container: 10 px.
const TITLE_INSET: Half = 20;

/// Line pitch of a container title, px.
pub(crate) const TITLE_STEP: Half = 16;

/// Height of the title band above the children for `lines` title lines: 10 px plus 16 px a line.
pub(crate) fn title_band(lines: usize) -> Half {
    20 + 32 * Half::try_from(lines).unwrap_or(0)
}

fn widest(lines: &[String]) -> Half {
    lines.iter().map(|l| measure(l)).max().unwrap_or(0)
}

/// `lines` if they fit `inner` half-pixels, else the same text re-wrapped at the widest cell limit
/// that fits (a single cell per line at worst).
fn fit_title(lines: &[String], inner: Half) -> Vec<String> {
    if widest(lines) <= inner {
        return lines.to_vec();
    }
    let text = lines.join("\n");
    let mut best = lines.to_vec();
    for cells in (1..WRAP_CELLS).rev() {
        best = wrap_at(&text, cells);
        if widest(&best) <= inner {
            break;
        }
    }
    best
}

/// The final frame of container `c` whose engine rect is `engine`: padded 8 px on the left, right
/// and bottom, with the title band on top and the title fitted to the final inner width.
pub(crate) fn frame(c: &LContainer, engine: Rect) -> PlacedContainer {
    let w = engine.w + 2 * CONTAINER_PAD;
    let title_lines = fit_title(&c.title_lines, w - 2 * TITLE_INSET);
    let band = title_band(title_lines.len());
    PlacedContainer {
        rect: Rect {
            x: engine.x - CONTAINER_PAD,
            y: engine.y - band,
            w,
            h: engine.h + CONTAINER_PAD + band,
        },
        title_lines,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn container(title: &[&str]) -> LContainer {
        LContainer {
            parent: None,
            depth: 1,
            title_lines: title.iter().map(|s| (*s).to_owned()).collect(),
            strut_w: 40,
        }
    }

    const ENGINE: Rect = Rect {
        x: 100,
        y: 200,
        w: 400,
        h: 300,
    };

    #[test]
    fn the_band_is_10px_plus_16px_per_line() {
        assert_eq!(title_band(0), 20);
        assert_eq!(title_band(1), 52);
        assert_eq!(title_band(3), 116);
    }

    #[test]
    fn the_engine_rect_is_padded_on_three_sides_and_grown_upwards_by_the_band() {
        let f = frame(&container(&["Title"]), ENGINE);
        assert_eq!(
            f.rect,
            Rect {
                x: 84,
                y: 148,
                w: 432,
                h: 300 + 16 + 52
            }
        );
        assert_eq!(f.title_lines, vec!["Title".to_owned()]);
    }

    #[test]
    fn a_title_that_fits_is_kept_as_wrapped() {
        let t = ["Backend services", "second line"];
        let f = frame(&container(&t), ENGINE);
        assert_eq!(f.title_lines, t.map(str::to_owned).to_vec());
        assert_eq!(f.rect.y, 200 - title_band(2));
    }

    #[test]
    fn a_title_wider_than_the_frame_is_rewrapped_to_fit_and_the_band_grows() {
        let narrow = Rect { w: 200, ..ENGINE };
        let one_line = ["WWWW iiii WWWW iiii WWWW"];
        let f = frame(&container(&one_line), narrow);
        let inner = f.rect.w - 40;
        assert!(f.title_lines.len() > 1, "{:?}", f.title_lines);
        assert!(f.title_lines.iter().all(|l| measure(l) <= inner));
        assert_eq!(f.rect.y, narrow.y - title_band(f.title_lines.len()));
    }

    #[test]
    fn a_title_that_never_fits_ends_at_one_cell_per_line() {
        let tiny = Rect { w: 0, ..ENGINE };
        let f = frame(&container(&["WWW"]), tiny);
        assert!(f.title_lines.len() >= 3, "{:?}", f.title_lines);
    }
}
