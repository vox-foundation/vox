//! Library chunker fixtures (IDX-02, card half of IDX-03, IDX-04) and the real-corpus parity
//! guarantee: chunk identity is exactly Phase 1's `kb_ids` identity.
#![cfg(feature = "tantivy-lexical")]

mod common;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use vox_db::{KB_LIBRARY_CORPUS, VoxDb};
use vox_search::kb_ids;
use vox_search::library::chunk::{
    ChunkKind, ParsedDoc, SECTION_TOKEN_BUDGET, chunk_document, estimate_tokens,
};
use vox_search::library::corpus::walk_corpus;
use vox_search::library::{IndexDeps, index_library};

fn hasher(b: &[u8]) -> String {
    vox_crypto::hex_encode(&vox_crypto::secure_hash(b))
}

fn chunk(text: &str) -> ParsedDoc {
    chunk_document("docs/src/t.md", text, &hasher)
}

fn paths(doc: &ParsedDoc) -> Vec<&str> {
    doc.chunks.iter().map(|c| c.heading_path.as_str()).collect()
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn chunk_paths_match_kb_ids() {
    let root = repo_root();
    let files = walk_corpus(&root).0;
    assert!(!files.is_empty(), "no corpus under {}", root.display());
    for f in files {
        let raw = std::fs::read_to_string(&f.abs).unwrap_or_else(|e| panic!("{}: {e}", f.rel_path));
        let text = vox_bounded_fs::normalize_text(raw);
        let doc = chunk_document(&f.rel_path, &text, &hasher);
        let got: BTreeSet<String> = doc.chunks.iter().map(|c| c.heading_path.clone()).collect();
        let want = kb_ids::doc_heading_paths(&f.rel_path, &text);
        assert_eq!(
            got,
            want,
            "{}: only in chunks {:?}; only in kb_ids {:?}",
            f.rel_path,
            got.difference(&want).collect::<Vec<_>>(),
            want.difference(&got).collect::<Vec<_>>()
        );
        let hashes: BTreeSet<&str> = doc.chunks.iter().map(|c| c.hash.as_str()).collect();
        assert_eq!(
            hashes.len(),
            doc.chunks.len(),
            "{}: duplicate hash",
            f.rel_path
        );
    }
}

#[test]
fn chunk_fence_hash_no_split() {
    let doc = chunk(
        "# T\n## Real\nbefore\n```bash\n# not a heading\n## also not\n```\nafter\n~~~\n# tilde\n~~~\n",
    );
    assert!(paths(&doc).iter().all(|p| !p.ends_with("not a heading")
        && !p.ends_with("also not")
        && !p.ends_with("tilde")));
    let real = doc
        .chunks
        .iter()
        .find(|c| c.heading_path == "T › Real")
        .unwrap();
    assert!(real.body.contains("# not a heading") && real.body.contains("## also not"));
    assert!(real.body.contains("# tilde") && real.body.contains("after"));
}

#[test]
fn chunk_frontmatter_never_in_body() {
    let doc = chunk(
        "---\ntitle: T\ndescription: Some desc\ncategory: Guides\nstatus: current\n# a yaml comment\n---\nintro\n## A\ntext\n",
    );
    for c in &doc.chunks {
        assert!(!c.body.contains("# a yaml comment"), "{}", c.body);
        assert!(!c.body.contains("title:"), "{}", c.body);
        if c.kind == ChunkKind::Section {
            assert!(!c.body.contains("description:"), "{}", c.body);
            assert!(!c.body.contains("category:"), "{}", c.body);
        }
    }
}

#[test]
fn chunk_heading_path_h3_to_h6() {
    let doc = chunk(
        "---\ntitle: T\n---\n## A\na\n### B\nb\n#### C\nc\n##### D\nd\n###### E\ne\n## F\nf\n# Whatever\nw\n",
    );
    let got = paths(&doc);
    for want in [
        "T › A",
        "T › A › B",
        "T › A › B › C",
        "T › A › B › C › D",
        "T › A › B › C › D › E",
        "T › F",
        "T",
    ] {
        assert!(got.contains(&want), "missing {want:?} in {got:?}");
    }
    // The H1 between sections resets the stack to the root, and is not a segment.
    assert!(got.iter().all(|p| !p.contains("Whatever")));
}

#[test]
fn chunk_h1_not_duplicated() {
    let doc = chunk("---\ntitle: Guide\n---\n# Guide\ntext\n## Install\nx\n");
    let set: BTreeSet<&str> = paths(&doc).into_iter().collect();
    assert_eq!(set, BTreeSet::from(["Guide", "Guide › Install"]));
    assert!(!set.contains("Guide › Guide"));

    let doc = chunk("# Beta Notes\nintro\n## One\nx\n");
    assert_eq!(doc.title, "Beta Notes");
    assert!(paths(&doc).contains(&"Beta Notes"));
    assert!(paths(&doc).contains(&"Beta Notes › One"));
}

#[test]
fn chunk_crlf_bom_same_hashes() {
    let lf = "---\ntitle: T\ndescription: d\n---\nintro\n## A\nline one\nline two\n```\n# x\n```\n## B\nz\n";
    let crlf = format!("\u{feff}{}", lf.replace('\n', "\r\n"));
    let a = chunk(lf);
    let b = chunk(&crlf);
    let ha: Vec<&str> = a.chunks.iter().map(|c| c.hash.as_str()).collect();
    let hb: Vec<&str> = b.chunks.iter().map(|c| c.hash.as_str()).collect();
    assert!(ha.len() >= 3);
    assert_eq!(ha, hb);
}

#[test]
fn chunk_anchor_github_slugger() {
    let doc = chunk("# T\nintro\n## Install `vox kb` (macOS)!\nx\n## Usage\na\n## Usage\nb\n");
    let anchors: Vec<(&str, Option<&str>)> = doc
        .chunks
        .iter()
        .map(|c| (c.heading_path.as_str(), c.anchor.as_deref()))
        .collect();
    assert!(
        anchors.contains(&("T › Install vox kb (macOS)!", Some("install-vox-kb-macos"))),
        "{anchors:?}"
    );
    let usage: Vec<Option<&str>> = anchors
        .iter()
        .filter(|(p, _)| *p == "T › Usage")
        .map(|(_, a)| *a)
        .collect();
    assert_eq!(usage, [Some("usage"), Some("usage-1")]);
    let card = &doc.chunks[0];
    assert_eq!((card.kind, card.anchor.as_deref()), (ChunkKind::Card, None));
}

#[test]
fn chunk_card_first() {
    let doc = chunk(
        "---\ntitle: Guide\ndescription: How to install it.\ncategory: How-To\nstatus: current\n---\n# Guide\n\nFirst paragraph here\nspanning two lines.\n\nSecond paragraph.\n\n## Install\nx\n",
    );
    let cards: Vec<_> = doc
        .chunks
        .iter()
        .filter(|c| c.kind == ChunkKind::Card)
        .collect();
    assert_eq!(cards.len(), 1);
    assert_eq!(doc.chunks[0].kind, ChunkKind::Card);
    let card = &doc.chunks[0];
    assert_eq!(
        card.heading_path,
        kb_ids::doc_title("docs/src/t.md", "---\ntitle: Guide\n---\n")
    );
    for want in [
        "Guide",
        "How to install it.",
        "First paragraph here\nspanning two lines.",
        "category: How-To",
        "status: current",
    ] {
        assert!(card.body.contains(want), "{want:?} not in {:?}", card.body);
    }
    assert!(!card.body.contains("Second paragraph"));
    assert!(!card.body.contains("# Guide"));
    // A doc with no frontmatter still gets exactly one card.
    let bare = chunk("# Bare\ntext\n");
    assert_eq!(
        bare.chunks
            .iter()
            .filter(|c| c.kind == ChunkKind::Card)
            .count(),
        1
    );
    assert_eq!(bare.chunks[0].heading_path, "Bare");
}

#[tokio::test]
async fn kb_chunking_end_to_end() {
    let dir = tempfile::tempdir().unwrap();
    let docs = [
        (
            "docs/src/a.md",
            "---\ntitle: Alpha\ndescription: About alpha.\n---\nintro\n## Usage\nx\n## Usage\ny\n",
        ),
        ("docs/src/sub/b.md", "# Beta\npara\n### Deep\nz\n"),
    ];
    for (rel, text) in docs {
        let p = dir.path().join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }
    let db = VoxDb::open_memory().await.unwrap();
    let deps = IndexDeps {
        hasher: &hasher,
        corpus_commit: None,
        trust_tier: "trusted",
    };
    index_library(&db, dir.path(), &dir.path().join("lex"), &deps)
        .await
        .unwrap();
    let rows = db.library_chunk_rows(KB_LIBRARY_CORPUS).await.unwrap();
    let mut by_doc: BTreeMap<&str, Vec<_>> = BTreeMap::new();
    for r in &rows {
        by_doc.entry(r.doc_path.as_str()).or_default().push(r);
    }
    for (rel, text) in docs {
        let want = chunk_document(rel, text, &hasher);
        let got = &by_doc[rel];
        assert_eq!(got.len(), want.chunks.len(), "{rel}");
        assert_eq!(got[0].chunk_kind, "card", "{rel}");
        assert_eq!(
            got.iter().filter(|r| r.chunk_kind == "card").count(),
            1,
            "{rel}"
        );
        for (r, c) in got.iter().zip(&want.chunks) {
            assert_eq!(
                (
                    r.chunk_hash.as_str(),
                    r.heading_path.as_str(),
                    r.anchor.as_deref(),
                    r.chunk_kind.as_str()
                ),
                (
                    c.hash.as_str(),
                    c.heading_path.as_str(),
                    c.anchor.as_deref(),
                    c.kind.as_str()
                ),
                "{rel}"
            );
        }
    }
}

fn big_doc() -> String {
    let prose = "The indexer walks every heading section and keeps each one whole when it fits. ";
    let mut s = String::from("---\ntitle: T\n---\n## Big\n");
    for i in 0..60 {
        s.push_str(&format!("Paragraph {i}. {prose}{prose}\n\n"));
    }
    s.push_str("```rust\nfn main() {\n    // # not a heading\n    println!(\"hi\");\n}\n```\n\n");
    for i in 60..80 {
        s.push_str(&format!("Paragraph {i}. {prose}\n\n"));
    }
    s
}

#[test]
fn chunk_oversize_split_within_budget() {
    let text = big_doc();
    assert!(text.len() > 9_000);
    let doc = chunk(&text);
    let parts: Vec<_> = doc
        .chunks
        .iter()
        .filter(|c| c.kind == ChunkKind::Section && c.heading_path == "T › Big")
        .collect();
    assert!(parts.len() >= 2, "{} parts", parts.len());
    for p in &parts {
        assert!(
            estimate_tokens(&p.body) <= SECTION_TOKEN_BUDGET,
            "{}",
            p.token_estimate
        );
        assert_eq!(p.token_estimate, estimate_tokens(&p.body));
        // Every part keeps the section's anchor; no part cuts inside the fence.
        assert_eq!(p.anchor.as_deref(), Some("big"));
        assert_eq!(
            p.body.matches("```").count() % 2,
            0,
            "fence cut: {}",
            p.body
        );
    }
    assert_eq!(
        parts
            .iter()
            .filter(|p| p.body.contains("fn main()"))
            .count(),
        1
    );
    let hashes: BTreeSet<&str> = parts.iter().map(|p| p.hash.as_str()).collect();
    assert_eq!(hashes.len(), parts.len());
    // Splitting never invents a heading_path.
    let got: BTreeSet<&str> = paths(&doc).into_iter().collect();
    assert_eq!(got, BTreeSet::from(["T", "T › Big"]));
}

#[test]
fn chunk_small_section_not_split() {
    let text = "---\ntitle: T\n---\n## Small\n".to_string() + &"word ".repeat(400);
    assert!(estimate_tokens(&text) <= SECTION_TOKEN_BUDGET);
    let doc = chunk(&text);
    let small = doc
        .chunks
        .iter()
        .filter(|c| c.heading_path == "T › Small")
        .count();
    assert_eq!(small, 1);
}

#[test]
fn chunk_split_is_deterministic() {
    let text = big_doc();
    let key = |d: &ParsedDoc| -> Vec<(String, String)> {
        d.chunks
            .iter()
            .map(|c| (c.heading_path.clone(), c.hash.clone()))
            .collect()
    };
    assert_eq!(key(&chunk(&text)), key(&chunk(&text)));
}

fn write_file(root: &Path, rel: &str, text: &str) {
    let p = root.join(rel);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, text).unwrap();
}

fn walked(root: &Path) -> Vec<String> {
    walk_corpus(root)
        .0
        .into_iter()
        .map(|f| f.rel_path)
        .collect()
}

#[tokio::test]
async fn kb_archive_excluded() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    for rel in [
        "docs/src/archive/old.md",
        "docs/src/research/archive/older.md",
        "docs/src/notes.mdx",
        "docs/src/ok.md",
        "docs/src/library/page.md",
        "outside.md",
    ] {
        write_file(root, rel, "# T\nbody\n");
    }
    #[cfg(unix)]
    std::os::unix::fs::symlink(root.join("outside.md"), root.join("docs/src/link.md")).unwrap();

    assert_eq!(walked(root), ["docs/src/library/page.md", "docs/src/ok.md"]);

    let db = VoxDb::open_memory().await.unwrap();
    let deps = IndexDeps {
        hasher: &hasher,
        corpus_commit: None,
        trust_tier: "trusted",
    };
    index_library(&db, root, &root.join("lex"), &deps)
        .await
        .unwrap();
    let rows = db.library_chunk_rows(KB_LIBRARY_CORPUS).await.unwrap();
    assert!(!rows.is_empty());
    assert!(rows.iter().all(|r| !r.doc_path.contains("/archive/")));
    let docs: BTreeSet<&str> = rows.iter().map(|r| r.doc_path.as_str()).collect();
    assert_eq!(
        docs,
        BTreeSet::from(["docs/src/library/page.md", "docs/src/ok.md"])
    );
}

#[test]
fn walker_matches_kb_filter() {
    let dir = tempfile::tempdir().unwrap();
    for rel in [
        "docs/src/a.md",
        "docs/src/archived-notes.md",
        "docs/src/library/archive-policy.md",
        "docs/src/archive/x.md",
        "docs/src/c#-notes.md",
        "docs/src/n.txt",
    ] {
        write_file(dir.path(), rel, "# T\n");
    }
    let got = walked(dir.path());
    assert_eq!(
        got,
        [
            "docs/src/a.md",
            "docs/src/archived-notes.md",
            "docs/src/library/archive-policy.md"
        ]
    );
    // Same predicate on the real tree.
    for f in walk_corpus(&repo_root())
        .0
        .iter()
        .chain(&walk_corpus(dir.path()).0)
    {
        assert!(kb_ids::is_kb_corpus_path(&f.rel_path), "{}", f.rel_path);
    }
    // Component match, not substring.
    assert!(kb_ids::is_kb_corpus_path("docs/src/archived-notes.md"));
    assert!(kb_ids::is_kb_corpus_path(
        "docs/src/library/archive-policy.md"
    ));
    assert!(!kb_ids::is_kb_corpus_path("docs/src/library/archive/x.md"));
}
