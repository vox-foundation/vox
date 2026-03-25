---
id: "{{id}}"
title: "{{title}}"
author: "{{author}}"
published_at: "{{published_at}}"
tags: ["research", "agentic-ai", "vox"]
syndication:
  github:
    repo: "{{default_github_repo}}"
    post_type: "Release"
    release_tag: "news-{{id}}"
    draft: true
  twitter:
    short_text: "New research: {{title}}. See the abstract in the thread."
    thread: true
  open_collective:
    collective_slug: "{{default_collective_slug}}"
    is_private: false
  rss: true
  dry_run: true
---
# {{title}}
*By {{author}}*

## Abstract
{{abstract_text}}

## Key Findings
- [Agent to insert detailed bullet points]
