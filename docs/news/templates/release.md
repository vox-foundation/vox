---
id: "{{id}}"
title: "{{title}}"
author: "{{author}}"
published_at: "{{published_at}}"
tags: ["release", "vox"]
syndication:
  github:
    repo: "{{default_github_repo}}"
    post_type: "Release"
    release_tag: "{{release_tag}}"
    draft: true
  twitter:
    short_text: "{{tweet_summary}}"
  rss: true
  dry_run: true
---
# {{title}}

{{body_markdown}}
