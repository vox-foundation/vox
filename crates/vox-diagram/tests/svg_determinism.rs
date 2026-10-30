//! REND-02: the same IR renders to the same bytes. Every `render` builds a fresh dagre graph
//! (fresh hash seeds inside the engine), so repeating in one process is a meaningful check;
//! cross-platform bytes are compared by the goldens on hosted CI.

mod support;

use std::collections::BTreeSet;
use std::fs;

use proptest::prelude::*;
use support::roundtrip::fixtures;
use support::{model_graph, noise, noisy_source};
use vox_diagram::check;
use vox_diagram::ir::Ir;
use vox_diagram::mermaid::{ImportOpts, import};
use vox_diagram::svg::{SvgOpts, render};

fn ir_of(src: &str) -> Ir {
    let Some(ir) = check(src).ir else {
        panic!("source does not check: {src:?}");
    };
    ir
}

fn read(rel: &str) -> String {
    let path = fixtures().join(rel);
    match fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => panic!("cannot read {}: {e}", path.display()),
    }
}

#[test]
fn twenty_fresh_renders() {
    let block =
        read("corpus/13-deep-research-waves-and-competitive-analysis-research-2026-l110.mmd");
    let Some(imported) = import(&block, ImportOpts::default()).source else {
        panic!("corpus block does not import");
    };
    let sources = [
        read("svg/cases/00-minimal.vdg"),
        read("svg/cases/10-edges.vdg"),
        read("svg/cases/21-nested.vdg"),
        imported,
    ];
    for src in &sources {
        let ir = ir_of(src);
        let outs: BTreeSet<String> = (0..20)
            .map(|_| match render(&ir, &SvgOpts::default()) {
                Ok(svg) => svg,
                Err(e) => panic!("render failed: {e}\n{src}"),
            })
            .collect();
        assert_eq!(outs.len(), 1, "{src}");
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// Two renders of one IR are equal, whether they succeed or fail.
    #[test]
    fn generated_graphs_render_identically(m in model_graph(), n in noise()) {
        let src = noisy_source(&m, &n);
        let Some(ir) = check(&src).ir else {
            return Err(TestCaseError::fail(format!("does not check:\n{src}")));
        };
        let opts = SvgOpts::default();
        prop_assert_eq!(render(&ir, &opts), render(&ir, &opts));
    }
}
