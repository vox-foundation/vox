#!/usr/bin/env python3
"""
Post-build SEO enrichment pass for the Vox mdBook output.

What this does (in order, per HTML file):
  1. Derives the canonical URL from the output file path (uses .html extension correctly).
  2. Overwrites the JS-placeholder <link rel="canonical"> with the real static value
     so that non-JS crawlers and Google's HTML-only pass get the right canonical.
  3. Reads the <meta name="description"> that mdbook-metadata injected and deduplicates
     it (removes the global fallback if a per-page one is present).
  4. Reads the page-level <meta name="schema_type"> injected from frontmatter and emits
     a contextual JSON-LD <script> block (HowTo, FAQPage, TechArticle, or SoftwareSourceCode).

Run from the repo root after `mdbook build docs`:
    python3 docs/scripts/seo_postprocess.py
"""

import os
import re
import sys
from pathlib import Path

BOOK_DIR = Path("docs/book")
BASE_URL = "https://vox-lang.org"

SITE_DESCRIPTION = (
    "Vox is an AI-native, null-free programming language that compiles to Rust and "
    "TypeScript. One file covers schema, server, UI, and AI agent tools."
)

HOWTO_SCHEMA_TMPL = """\
<script type="application/ld+json">
{{
  "@context": "https://schema.org",
  "@type": "HowTo",
  "name": "{title}",
  "description": "{description}",
  "url": "{url}",
  "publisher": {{
    "@type": "Organization",
    "name": "Vox Foundation",
    "url": "https://vox-lang.org"
  }}
}}
</script>"""

FAQPAGE_SCHEMA_TMPL = """\
<script type="application/ld+json">
{{
  "@context": "https://schema.org",
  "@type": "FAQPage",
  "name": "{title}",
  "url": "{url}"
}}
</script>"""

TECH_ARTICLE_SCHEMA_TMPL = """\
<script type="application/ld+json">
{{
  "@context": "https://schema.org",
  "@type": "TechArticle",
  "headline": "{title}",
  "description": "{description}",
  "url": "{url}",
  "dateModified": "{date_modified}",
  "publisher": {{
    "@type": "Organization",
    "name": "Vox Foundation",
    "url": "https://vox-lang.org"
  }}
}}
</script>"""

SOFTWARE_SCHEMA = """\
<script type="application/ld+json">
{
  "@context": "https://schema.org",
  "@type": "SoftwareSourceCode",
  "name": "Vox Programming Language",
  "description": "An AI-native, full-stack programming language that compiles to Rust and TypeScript.",
  "codeRepository": "https://github.com/vox-foundation/vox",
  "programmingLanguage": "Vox",
  "runtimePlatform": "Rust, Tokio, Axum",
  "license": "https://opensource.org/licenses/Apache-2.0",
  "author": {
    "@type": "Person",
    "name": "Bert Brainerd"
  }
}
</script>"""


def get_meta(html: str, name: str) -> str:
    """Extract content of <meta name="name"> or <meta property="name">."""
    pattern = (
        rf'<meta\s+(?:name|property)=["\']?{re.escape(name)}["\']?\s+content=["\']([^"\']*)["\']'
        rf'|<meta\s+content=["\']([^"\']*)["\']?\s+(?:name|property)=["\']?{re.escape(name)}["\']?'
    )
    m = re.search(pattern, html, re.IGNORECASE)
    if not m:
        return ""
    return (m.group(1) or m.group(2) or "").strip()


def path_to_url(rel_path: Path) -> str:
    """Convert a relative HTML path within docs/book/ to its canonical URL."""
    parts = rel_path.parts
    return BASE_URL + "/" + "/".join(parts)


def choose_schema(schema_type: str, title: str, description: str, url: str, date_modified: str) -> str:
    st = schema_type.lower()
    description = description or SITE_DESCRIPTION
    title = title or "Vox Programming Language"
    if st in ("howto", "how-to", "tutorial", "getting-started"):
        return HOWTO_SCHEMA_TMPL.format(title=title, description=description, url=url)
    elif st == "faqpage":
        return FAQPAGE_SCHEMA_TMPL.format(title=title, url=url)
    elif st in ("techarticle", "explanation", "reference", "architecture", "adr"):
        return TECH_ARTICLE_SCHEMA_TMPL.format(
            title=title, description=description, url=url,
            date_modified=date_modified or ""
        )
    else:
        return SOFTWARE_SCHEMA


def process_file(html_path: Path):
    rel = html_path.relative_to(BOOK_DIR)
    canonical_url = path_to_url(rel)

    text = html_path.read_text(encoding="utf-8")

    # 1. Overwrite the JS-placeholder canonical with the real static value
    text = re.sub(
        r'<link\s+id="canonical-url"\s+rel="canonical"\s+href="[^"]*">',
        f'<link id="canonical-url" rel="canonical" href="{canonical_url}">',
        text
    )

    # 2. Deduplicate <meta name="description">: if mdbook-metadata injected one,
    #    remove the global fallback (the one whose content equals SITE_DESCRIPTION).
    desc_tags = re.findall(r'<meta\s+name=["\']description["\'][^>]*>', text, re.IGNORECASE)
    if len(desc_tags) > 1:
        # Remove the global fallback (first occurrence)
        text = re.sub(
            r'<!-- Global meta description fallback[^>]*-->\s*<meta name="description"[^>]*>\n?',
            '', text, count=1
        )

    # 3. Read per-page SEO metadata for JSON-LD
    page_description = get_meta(text, "description") or SITE_DESCRIPTION
    schema_type = get_meta(text, "schema_type") or ""
    date_modified = get_meta(text, "last_updated") or ""
    title_match = re.search(r'<title>([^<]+)</title>', text)
    title = title_match.group(1).replace(" - Vox: The AI-Native Programming Language", "").strip() if title_match else ""

    # 4. Remove existing JSON-LD SoftwareSourceCode block from head.hbs (always present)
    text = re.sub(
        r'<!-- Structured Data: SoftwareSourceCode \(JSON-LD\) -->\s*<script type="application/ld\+json">.*?</script>',
        '', text, flags=re.DOTALL
    )

    # 5. Inject contextual JSON-LD before </head>
    schema_block = choose_schema(schema_type, title, page_description, canonical_url, date_modified)
    text = text.replace("</head>", f"\n{schema_block}\n</head>", 1)

    html_path.write_text(text, encoding="utf-8")


def main():
    if not BOOK_DIR.exists():
        print(f"ERROR: {BOOK_DIR} not found. Run `mdbook build docs` first.", file=sys.stderr)
        sys.exit(1)

    processed = 0
    for html_file in BOOK_DIR.rglob("*.html"):
        # Skip the print.html omnibus page
        if html_file.name == "print.html":
            continue
        process_file(html_file)
        processed += 1

    print(f"seo_postprocess.py: processed {processed} HTML pages.")


if __name__ == "__main__":
    main()
