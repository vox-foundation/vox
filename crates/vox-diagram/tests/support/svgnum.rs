//! Number and shape parsing for `svgcheck`: half-pixel decimals, path data and polygon boxes.
#![allow(dead_code)]

/// Axis-aligned box in half-pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Shape {
    pub x: i64,
    pub y: i64,
    pub w: i64,
    pub h: i64,
}

pub type R<T> = Result<T, String>;

pub fn bad<T>(m: impl Into<String>) -> R<T> {
    Err(m.into())
}

/// `12` -> 24, `12.5` -> 25, `-3.5` -> -7; anything else is an error.
pub fn half(s: &str) -> R<i64> {
    let (neg, body) = s.strip_prefix('-').map_or((false, s), |b| (true, b));
    let (int, frac) = body
        .split_once('.')
        .map_or((body, None), |(i, f)| (i, Some(f)));
    if int.is_empty() || !int.bytes().all(|b| b.is_ascii_digit()) {
        return bad(format!("bad number {s:?}"));
    }
    let mut v = int.parse::<i64>().map_err(|e| format!("{s:?}: {e}"))? * 2;
    match frac {
        None => {}
        Some("5") => v += 1,
        Some(_) => return bad(format!("not a half-pixel number {s:?}")),
    }
    Ok(if neg { -v } else { v })
}

pub fn pairs(nums: &[i64]) -> R<Vec<(i64, i64)>> {
    if nums.is_empty() || !nums.len().is_multiple_of(2) {
        return bad(format!("odd coordinate count {}", nums.len()));
    }
    Ok(nums.chunks(2).map(|c| (c[0], c[1])).collect())
}

/// Path data `M/L/Q` followed by space-separated numbers, as points.
pub fn path_points(d: &str) -> R<Vec<(i64, i64)>> {
    let mut nums = Vec::new();
    for chunk in d.split(|c: char| c.is_ascii_alphabetic()) {
        for tok in chunk.split_whitespace() {
            nums.push(half(tok)?);
        }
    }
    if let Some(c) = d
        .chars()
        .find(|c| c.is_ascii_alphabetic() && !"MLQ".contains(*c))
    {
        return bad(format!("path command {c:?}"));
    }
    pairs(&nums)
}

pub fn polygon_box(points: &str) -> R<Shape> {
    let mut nums = Vec::new();
    for tok in points.split(|c: char| c == ',' || c.is_whitespace()) {
        if !tok.is_empty() {
            nums.push(half(tok)?);
        }
    }
    let pts = pairs(&nums)?;
    let (x0, x1) = (
        pts.iter().map(|p| p.0).min().unwrap_or(0),
        pts.iter().map(|p| p.0).max().unwrap_or(0),
    );
    let (y0, y1) = (
        pts.iter().map(|p| p.1).min().unwrap_or(0),
        pts.iter().map(|p| p.1).max().unwrap_or(0),
    );
    Ok(Shape {
        x: x0,
        y: y0,
        w: x1 - x0,
        h: y1 - y0,
    })
}
