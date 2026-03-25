---
id: "{{id}}"
title: "{{title}}"
author: "{{author}}"
published_at: "{{published_at}}"
tags: ["security", "advisory", "vox"]
syndication:
  github:
    repo: "{{default_github_repo}}"
    post_type: "Discussion"
    discussion_category: "{{discussion_category}}"
  twitter:
    short_text: "{{tweet_summary}}"
  open_collective:
    collective_slug: "{{default_collective_slug}}"
    is_private: false
  rss: true
  dry_run: true
---
# {{title}}

{{body_markdown}}
