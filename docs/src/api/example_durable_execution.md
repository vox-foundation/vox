---
title: "Example: Durable Execution Example"
description: "Official documentation for Example: Durable Execution Example for the Vox language. Detailed technical reference, architecture guides, an"
category: "reference"
last_updated: 2026-03-29
training_eligible: true
---
# Example: Durable Execution Example

```vox
# Durable Execution Example
# Demonstrates activities, workflows, and the `with` expression syntax.

type OrderResult =
    | Ok(order_id: str)
    | Error(message: str)

type PaymentResult =
    | Ok(tx_id: str)
    | Error(message: str)

# Activities are units of work that may fail and be retried.
# They must return a Result type.
activity validate_order(order_data: str) to Result[str]:
    let validated = "validated-" + order_data
    ret Ok(validated)

activity charge_payment(amount: int, card_token: str) to Result[str]:
    let tx = "tx-" + card_token
    ret Ok(tx)

activity send_confirmation(recipient: str, order_id: str) to Result[str]:
    let msg = "Order " + order_id + " confirmed for " + recipient
    ret Ok(msg)

# Workflows orchestrate activities. Durable step replay today is scoped to the
# interpreted workflow runtime for linear workflows.
# Retry/backoff on that interpreted path currently matters for `mesh_*`
# activity execution; ordinary activities in examples like this still mostly
# show language shape and stable `activity_id` usage rather than full runtime parity.
workflow process_order(customer: str, order_data: str, amount: int) to Result[str]:
    # Validate with a short timeout and no retries
    let validated = validate_order(order_data) with { timeout: "5s" }

    # Charge payment with retries and backoff
    let payment = charge_payment(amount, "card-123") with { retries: 3, timeout: "30s", initial_backoff: "500ms" }

    # Send confirmation with basic retry
    let confirmation = send_confirmation(customer, "order-001") with { retries: 2, activity_id: "confirm-order-001" }

    ret confirmation
```
