#!/usr/bin/env python3
import json
import os
import sys
import re
from pathlib import Path

def main():
    report_path = Path("docs/lychee_report.json")
    book_dir = Path("docs/book")

    if not report_path.exists():
        print("lychee_report.json not found. Skipping link icon injection.")
        return

    if not book_dir.exists():
        print("docs/book not found. Run mdbook build first.")
        return

    try:
        with open(report_path, "r", encoding="utf-8-sig") as f:
            data = json.load(f)
    except Exception as e:
        print(f"Failed to load lychee JSON: {e}")
        return

    def normalize_url(url):
        return url.rstrip('/')

    success_urls = set()
    error_urls = set()
    excluded_urls = set()

    for file_map in data.get("success_map", {}).values():
        for item in file_map:
            url = item.get("url", "")
            if url.startswith("http"):
                success_urls.add(normalize_url(url))

    for file_map in data.get("error_map", {}).values():
        for item in file_map:
            url = item.get("url", "")
            if url.startswith("http"):
                error_urls.add(normalize_url(url))

    for file_map in data.get("excluded_map", {}).values():
        for item in file_map:
            url = item.get("url", "")
            if url.startswith("http"):
                excluded_urls.add(normalize_url(url))

    def replace_link(match):
        full_match = match.group(0)
        href = match.group(1)
        link_content = match.group(2)
        
        if not href.startswith("http"):
            return full_match

        norm_href = normalize_url(href)
        icon = ""
        if norm_href in error_urls:
            icon = ' <span class="lychee-status error" title="Broken link">🔴</span>'
        elif norm_href in excluded_urls:
            icon = ' <span class="lychee-status excluded" title="Ignored link">⚪</span>'
        elif norm_href in success_urls:
            icon = ' <span class="lychee-status success" title="Verified link">🟢</span>'

        return f'<a href="{href}"{match.group(3)}>{link_content}{icon}</a>'

    # Regex matches: <a href="href" [other_attrs]>link_content</a>
    pattern = re.compile(r'<a\s+href="([^"]+)"([^>]*)>(.*?)</a>', re.IGNORECASE | re.DOTALL)

    processed = 0
    for html_file in book_dir.rglob("*.html"):
        text = html_file.read_text(encoding="utf-8-sig")
        new_text = pattern.sub(replace_link, text)
        if text != new_text:
            html_file.write_text(new_text, encoding="utf-8")
            processed += 1

    print(f"Injected lychee icons into {processed} HTML files.")

if __name__ == "__main__":
    main()
