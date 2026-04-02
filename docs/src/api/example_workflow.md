---
title: "Example: Vox Workflow Example"
description: "Official documentation for Example: Vox Workflow Example for the Vox language. Detailed technical reference, architecture guides, and imp"
category: "reference"
last_updated: 2026-03-24
training_eligible: true
---
# Example: Vox Workflow Example

```vox
# Vox Workflow Example
# Demonstrates workflow syntax: steps, `with` options, and activity calls.
#
# Intent: long-running orchestration. Today, durable step replay is the
# interpreted `vox mens workflow ...` path with a stable run id and
# `activity_id`; generated Rust is not yet a full durable state machine.
# See expl-actors-workflows.md. Interpreted durable runs now honor retry/
# backoff for `mesh_*` activity execution; ordinary activities in examples
# like this still mainly show language shape and stable `activity_id` usage.

# An activity that fetches data from an external API.
# Activities are the only place for side effects in workflows.
activity fetch_user_data(user_id: str) to Result[str]:
    # This would call an external API in production
    ret Ok("User data for " + user_id)

# An activity that sends a notification email.
# The `with` expression configures retry and timeout behavior.
activity send_notification(email: str, body: str) to Result[bool]:
    # External email service call
    ret Ok(true)

# A multi-step onboarding workflow.
#
# With the interpreted runtime + same run id + stable activity ids, completed
# steps can be skipped on restart; idempotent activities still matter.
workflow onboard_user(user_id: str, email: str) to Result[str]:
    # Step 1: Fetch user profile
    let profile = fetch_user_data(user_id) with { retries: 3, timeout: "30s" }

    # Step 2: Send welcome email
    let _ = send_notification(email, "Welcome! " + profile) with { retries: 5, timeout: "60s" }

    # Step 3: Return success
    ret Ok("Onboarding complete for " + user_id)

# Entry point
fn main():
    let result = onboard_user("usr_123", "alice@example.com")
    ret result
```
