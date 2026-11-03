//! The allowlist tables of `svgcheck`: every element, attribute, class, style property and entity
//! the renderer may emit. Anything else is rejected.

pub(super) const ELEMENTS: [&str; 11] = [
    "svg", "title", "desc", "defs", "marker", "path", "rect", "polygon", "text", "tspan", "g",
];
pub(super) const ATTRS: [&str; 23] = [
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
pub(super) const CLASSES: [&str; 19] = [
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
    "vd-lifelines",
    "vd-lifeline",
    "vd-notes",
    "vd-note",
    "vd-note-label",
];
pub(super) const STYLE_PROPS: [&str; 6] = [
    "fill",
    "stroke",
    "stroke-width",
    "stroke-dasharray",
    "font-family",
    "font-size",
];
pub(super) const ENTITIES: [&str; 5] = ["amp", "lt", "gt", "quot", "#39"];
