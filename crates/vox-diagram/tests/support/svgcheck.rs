//! A quick-xml allowlist checker for the renderer's SVG, independent of the writer: one pass over
//! the events that rejects anything outside the contract (elements, attributes, classes, style
//! properties, entities, ids, references, raw text) and reads the drawn geometry and decoded
//! text back out. Numbers are parsed into half-pixel integers (`12.5` is 25).
#![allow(dead_code)]

use quick_xml::events::{BytesStart, Event};
use quick_xml::{Reader, XmlVersion};

/// One `text` element: `owner` indexes the shape it labels (`nodes`, `edge_labels` or
/// `containers` by `class`), `lines` are its decoded `tspan`s.
#[derive(Debug, Clone)]
pub struct Label {
    pub class: String,
    pub owner: usize,
    pub anchor_x: i64,
    pub first_baseline: i64,
    pub lines: Vec<String>,
}

/// What the checker read back.
#[derive(Debug, Default)]
pub struct Svg {
    pub width: i64,
    pub height: i64,
    pub nodes: Vec<Shape>,
    pub containers: Vec<Shape>,
    pub edge_points: Vec<(i64, i64)>,
    pub edge_labels: Vec<Shape>,
    pub labels: Vec<Label>,
    /// `(class, decoded text)`: `title`, `desc`, and each `text` element by class (lines joined
    /// with `\n`).
    pub texts: Vec<(String, String)>,
    pub attr_values: Vec<String>,
    pub ids: Vec<String>,
    pub root_children: Vec<String>,
    /// Every `property:value` of every `style`.
    pub paints: Vec<(String, String)>,
}

const ELEMENTS: [&str; 11] = [
    "svg", "title", "desc", "defs", "marker", "path", "rect", "polygon", "text", "tspan", "g",
];
const ATTRS: [&str; 23] = [
    "xmlns",
    "viewBox",
    "role",
    "aria-labelledby",
    "class",
    "style",
    "id",
    "refX",
    "refY",
    "markerWidth",
    "markerHeight",
    "orient",
    "d",
    "marker-end",
    "x",
    "y",
    "width",
    "height",
    "rx",
    "ry",
    "points",
    "text-anchor",
    "dy",
];
const CLASSES: [&str; 14] = [
    "vd-diagram",
    "vd-containers",
    "vd-container",
    "vd-container-label",
    "vd-edges",
    "vd-edge",
    "vd-nodes",
    "vd-node",
    "vd-node-shape",
    "vd-node-rim",
    "vd-label",
    "vd-edge-labels",
    "vd-edge-label-bg",
    "vd-edge-label",
];
const STYLE_PROPS: [&str; 5] = ["fill", "stroke", "stroke-width", "font-family", "font-size"];
const ENTITIES: [&str; 5] = ["amp", "lt", "gt", "quot", "#39"];

pub use super::svgnum::Shape;
use super::svgnum::{R, bad, half, path_points, polygon_box};

/// Entity names in a raw (undecoded) fragment; each must be on the allowlist.
fn check_entities(raw: &str) -> R<()> {
    let mut rest = raw;
    while let Some(i) = rest.find('&') {
        let tail = &rest[i + 1..];
        let Some(end) = tail.find(';') else {
            return bad(format!("unterminated entity in {raw:?}"));
        };
        if !ENTITIES.contains(&&tail[..end]) {
            return bad(format!("entity &{};", &tail[..end]));
        }
        rest = &tail[end + 1..];
    }
    Ok(())
}

fn check_raw_text(t: &str) -> R<()> {
    if t.contains('>') {
        return bad(format!("raw > in text {t:?}"));
    }
    if let Some(c) = t.chars().find(|c| {
        (c.is_control() && !matches!(c, '\t' | '\n' | '\r')) || matches!(c, '\u{fffe}' | '\u{ffff}')
    }) {
        return bad(format!("control char U+{:04X} in text {t:?}", u32::from(c)));
    }
    Ok(())
}

struct TextCtx {
    class: String,
    owner: usize,
    x: i64,
    y: i64,
    lines: Vec<String>,
}

#[derive(Default)]
struct Walk {
    out: Svg,
    prefix: String,
    stack: Vec<String>,
    root_done: bool,
    refs: Vec<String>,
    /// Inside a `g.vd-node` that has not drawn its outline yet.
    node_open: bool,
    text: Option<TextCtx>,
    buf: String,
}

fn name_of(b: &BytesStart<'_>) -> R<String> {
    std::str::from_utf8(b.name().as_ref())
        .map(str::to_owned)
        .map_err(|e| e.to_string())
}

fn style_ok(style: &str, out: &mut Vec<(String, String)>) -> R<()> {
    for decl in style.split(';') {
        let Some((prop, value)) = decl.split_once(':') else {
            return bad(format!("style declaration {decl:?}"));
        };
        if !STYLE_PROPS.contains(&prop) {
            return bad(format!("style property {prop:?}"));
        }
        if value.contains("url(") {
            return bad(format!("url( in style {style:?}"));
        }
        out.push((prop.to_owned(), value.to_owned()));
    }
    Ok(())
}

impl Walk {
    fn attrs(&mut self, e: &BytesStart<'_>) -> R<Vec<(String, String)>> {
        let mut got = Vec::new();
        for a in e.attributes() {
            let a = a.map_err(|e| format!("attribute: {e}"))?;
            let key = std::str::from_utf8(a.key.as_ref()).map_err(|e| e.to_string())?;
            if !ATTRS.contains(&key) {
                return bad(format!("attribute {key:?} not allowed"));
            }
            let raw = std::str::from_utf8(&a.value).map_err(|e| e.to_string())?;
            check_entities(raw)?;
            let val = a
                .normalized_value(XmlVersion::Implicit1_0)
                .map_err(|e| format!("attribute {key}: {e}"))?
                .into_owned();
            self.attr_rules(key, &val)?;
            self.out.attr_values.push(val.clone());
            got.push((key.to_owned(), val));
        }
        Ok(got)
    }

    fn attr_rules(&mut self, key: &str, val: &str) -> R<()> {
        match key {
            "style" => style_ok(val, &mut self.out.paints)?,
            "class" if !CLASSES.contains(&val) => return bad(format!("class {val:?} not allowed")),
            "id" => {
                if !val.starts_with(&format!("{}-", self.prefix)) {
                    return bad(format!("id {val:?} lacks prefix {:?}", self.prefix));
                }
                if self.out.ids.iter().any(|i| i == val) {
                    return bad(format!("duplicate id {val:?}"));
                }
                self.out.ids.push(val.to_owned());
            }
            "aria-labelledby" => self.refs.extend(val.split_whitespace().map(str::to_owned)),
            "marker-end" => match val.strip_prefix("url(#").and_then(|v| v.strip_suffix(')')) {
                Some(id) => self.refs.push(id.to_owned()),
                None => return bad(format!("marker-end {val:?}")),
            },
            _ if val.contains("url(") => return bad(format!("url( in {key}")),
            _ => {}
        }
        Ok(())
    }

    fn open(&mut self, e: &BytesStart<'_>, empty: bool) -> R<()> {
        let name = name_of(e)?;
        if !ELEMENTS.contains(&name.as_str()) {
            return bad(format!("element <{name}> not allowed"));
        }
        if self.stack.is_empty() {
            if self.root_done {
                return bad(format!("second root element <{name}>"));
            }
            if name != "svg" {
                return bad(format!("root is <{name}>"));
            }
        } else if self.stack.len() == 1 {
            self.out.root_children.push(name.clone());
        }
        let attrs = self.attrs(e)?;
        let get = |k: &str| attrs.iter().find(|(n, _)| n == k).map(|(_, v)| v.as_str());
        self.geometry(&name, &get)?;
        self.stack.push(name.clone());
        if empty {
            self.close(&name)?;
        }
        Ok(())
    }

    fn geometry<'a>(&mut self, name: &str, get: &impl Fn(&str) -> Option<&'a str>) -> R<()> {
        let num = |k: &str| half(get(k).unwrap_or(""));
        let boxed = || -> R<Shape> {
            Ok(Shape {
                x: num("x")?,
                y: num("y")?,
                w: num("width")?,
                h: num("height")?,
            })
        };
        match (name, get("class")) {
            ("svg", _) => {
                let vb: Vec<&str> = get("viewBox").unwrap_or("").split(' ').collect();
                if vb.len() != 4 || vb[0] != "0" || vb[1] != "0" {
                    return bad("viewBox must be 0 0 W H");
                }
                self.out.width = half(vb[2])?;
                self.out.height = half(vb[3])?;
            }
            ("g", Some("vd-node")) => self.node_open = true,
            ("rect" | "polygon", Some("vd-node-shape")) => {
                if !self.node_open {
                    return bad("node outline outside a vd-node or drawn twice");
                }
                self.node_open = false;
                let s = if name == "rect" {
                    boxed()?
                } else {
                    polygon_box(get("points").unwrap_or(""))?
                };
                self.out.nodes.push(s);
            }
            ("rect", Some("vd-container")) => self.out.containers.push(boxed()?),
            ("rect", Some("vd-edge-label-bg")) => self.out.edge_labels.push(boxed()?),
            ("path", Some("vd-edge")) => {
                let pts = path_points(get("d").unwrap_or(""))?;
                self.out.edge_points.extend(pts);
            }
            ("text", Some(class)) => {
                let owner = match class {
                    "vd-label" => self.out.nodes.len(),
                    "vd-edge-label" => self.out.edge_labels.len(),
                    "vd-container-label" => self.out.containers.len(),
                    _ => return bad(format!("text class {class:?}")),
                }
                .saturating_sub(1);
                self.text = Some(TextCtx {
                    class: class.to_owned(),
                    owner,
                    x: num("x")?,
                    y: num("y")?,
                    lines: Vec::new(),
                });
            }
            _ => {}
        }
        Ok(())
    }

    fn close(&mut self, name: &str) -> R<()> {
        self.stack.pop();
        match name {
            "title" | "desc" => {
                let t = std::mem::take(&mut self.buf);
                self.out.texts.push((name.to_owned(), t));
            }
            "tspan" => {
                let t = std::mem::take(&mut self.buf);
                match self.text.as_mut() {
                    Some(c) => c.lines.push(t),
                    None => return bad("tspan outside text"),
                }
            }
            "text" => {
                let Some(c) = self.text.take() else {
                    return bad("text without class");
                };
                self.out.texts.push((c.class.clone(), c.lines.join("\n")));
                self.out.labels.push(Label {
                    class: c.class,
                    owner: c.owner,
                    anchor_x: c.x,
                    first_baseline: c.y,
                    lines: c.lines,
                });
            }
            "g" if self.node_open => return bad("vd-node without an outline"),
            _ => {}
        }
        if self.stack.is_empty() {
            self.root_done = true;
        }
        Ok(())
    }

    fn in_text_leaf(&self) -> bool {
        matches!(
            self.stack.last().map(String::as_str),
            Some("title" | "desc" | "tspan")
        )
    }

    fn text_event(&mut self, t: &str) -> R<()> {
        check_raw_text(t)?;
        if self.in_text_leaf() {
            self.buf.push_str(t);
        } else if !t.trim().is_empty() {
            return bad(format!("text outside title/desc/tspan: {t:?}"));
        }
        Ok(())
    }

    fn reference(&mut self, name: &str) -> R<()> {
        let ch = match name {
            "amp" => '&',
            "lt" => '<',
            "gt" => '>',
            "quot" => '"',
            "#39" => '\'',
            _ => return bad(format!("entity &{name};")),
        };
        if !self.in_text_leaf() {
            return bad(format!("entity &{name}; outside title/desc/tspan"));
        }
        self.buf.push(ch);
        Ok(())
    }
}

fn run(svg: &str, prefix: &str) -> R<Svg> {
    let mut w = Walk {
        prefix: prefix.to_owned(),
        ..Walk::default()
    };
    let mut rd = Reader::from_str(svg);
    loop {
        match rd.read_event().map_err(|e| format!("xml: {e}"))? {
            Event::Start(e) => w.open(&e, false)?,
            Event::Empty(e) => w.open(&e, true)?,
            Event::End(e) => {
                let name = std::str::from_utf8(e.name().as_ref())
                    .map_err(|e| e.to_string())?
                    .to_owned();
                if w.stack.last() != Some(&name) {
                    return bad(format!("mismatched end tag </{name}>"));
                }
                w.close(&name)?;
            }
            Event::Text(t) => w.text_event(&t.decode().map_err(|e| e.to_string())?)?,
            Event::GeneralRef(r) => w.reference(&r.decode().map_err(|e| e.to_string())?)?,
            Event::Comment(_) => return bad("comment not allowed"),
            Event::CData(_) => return bad("CDATA not allowed"),
            Event::PI(_) | Event::Decl(_) => return bad("processing instruction not allowed"),
            Event::DocType(_) => return bad("DOCTYPE not allowed"),
            Event::Eof => break,
        }
    }
    if !w.stack.is_empty() || !w.root_done {
        return bad("document ends with open elements");
    }
    for r in &w.refs {
        if !w.out.ids.contains(r) {
            return bad(format!("unresolved reference #{r}"));
        }
    }
    Ok(w.out)
}

/// Accept `svg` only when it satisfies the whole allowlist contract with ids prefixed `prefix-`.
pub fn check(svg: &str, prefix: &str) -> Result<Svg, String> {
    run(svg, prefix)
}
