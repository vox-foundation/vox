---
title: "Journey: Reliable Background Workflows"
description: "How to escape brittle external job queues using Vox's native Durable Execution for microservice reliability."
category: "getting-started"
sort_order: 2
---

# Journey: Reliable Background Workflows

## The Brittle Reality of Job Queues

When a user submits an order, your system might need to charge a credit card, reserve inventory, and send an email out. What happens when the server crashes midway between reserving the inventory and sending the email? 

Microservice developers typically reach for complex infrastructure like Celery, Sidekiq, Temporal, AWS Step Functions, or Kafka. You write convoluted compensation logic, manual retry loops, and separate out small chunks of code across different services just to ensure task reliability. It fragments your business logic.

## The Vox Paradigm: Native Durable Execution

Vox gives you **Durable Execution** out of the box using two keywords: `@workflow` and `activity`.

You write a single function that looks like linear, synchronous code. Behind the scenes, Vox records the result of each `activity` in a persistent journal or VoxDB. If your server is killed midway through a workflow, upon restart Vox rapidly replays the workflow state, skips the already-completed steps natively (without re-running them), and resumes execution at the exact line of code where it left off.

## Core Snippet: Surviving a Server Crash

```vox
// vox:skip
// Activities are wrapped by the workflow runtime. 
activity charge_payment(amount: int, token: str) -> Result[str] {
    let result = std.http.post_json("https://api.stripe.com/v1/charges", {
        amount: amount,
        source: token
    })
    return Ok(result.json().id)
}

activity send_email(user: str, message: str) -> Result[Unit] {
    std.http.post_json("https://api.sendgrid.com/v3/mail/send", {
        to: user,
        text: message
    })
    return Ok(())
}

workflow process_order(customer: str, amount: int, card_tok: str) -> Result[str] {
    // 1. Charge via retryable activity. 
    let payment_id = charge_payment(amount, card_tok)
        with { retries: 3, timeout: "30s", initial_backoff: "500ms" }

    // 2. Send email
    let _ = send_email(customer, "Receipt for " + payment_id)

    return Ok(payment_id)
}
```

## Running the Process

1. Save the snippet into your project.
2. The orchestrator runtime requires a local state store to persist workflow states. Running:
   ```bash
   vox run server.vox
   ```
   Will automatically start the journal layer mapped to your local storage.

## Deep Dives

To learn more about the theoretical constraints and architectural layout of Vox's durable workflows:

- **[Tutorial: Workflow Durability](../tutorials/tut-workflow-durability.md)**: A step-by-step walkthough of the recovery mechanism.
- **[Explanation: Durable Execution](../explanation/expl-durable-execution.md)**: Deep dive into how Vox tracks replay safety and ensures side-effect idempotency.
- **[Durable Workflow Journal Contract v1](../adr/019-durable-workflow-journal-contract-v1.md)**: The ADR dictating the storage format and constraints placed on compiled state machines.
