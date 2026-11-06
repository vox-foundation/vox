---
title: "How-To: System I/O and Capabilities"
description: "How to safely interact with filesystems and network endpoints."
category: "How-To Guides"
status: "current"
training_eligible: true

schema_type: "HowTo"
---

# How-To: System I/O

Vox code natively compiles into isolated WASI execution bounded containers or strict actor channels. System IO (disk reading/writing, network fetching) runs under the `std.fs` and `std.http` global contexts.

> [!IMPORTANT]
> Aspirational `@task` sandboxes or untrusted LLM code generated at runtime may have explicit prohibitions against invoking arbitrary `std.fs` or `std.http` targets. See [Explanation: Capabilities](../explanation/expl-capabilities.md).

## Reading and Writing Files

The `std.fs` package treats operations as inherently failable (returning `Result`).

```vox
import std.fs

fn process_log() to Result[Unit] {
    let contents = fs.read("/var/logs/app.log")?
    
    if len(contents) > 1000 {
        fs.write("/var/logs/app-archive.log", contents)?
        fs.write("/var/logs/app.log", "")?
    }
    
    return Ok(Unit)
}
```

## External Network Requests

Vox uses `std.http` to generate outbound JSON API requests, translating directly to `reqwest` instances under the hood.

```vox
fn query_weather(city: str) to Result[str] {
    let endpoint = "https://api.weather.com/v1/" + city
    let response = std.http.get_text(endpoint)?
    return Ok(response)
}
```

If you are posting complex ADT models, serialize them safely across the JSON integration boundary.

```vox
fn publish_event(topic: str, payload: str) to Result[Unit] {
    let body = "{\"topic\":\"" + topic + "\",\"message\":\"" + payload + "\"}"
    let res = std.http.post_json("https://webhook.site/abc", body)?
    assert(res is "200 OK")
    return Ok(Unit)
}
```

## Handling Errors Gracefully

Always surface the `Result` type rather than attempting to `unwrap()` or panic inside production web routes, to allow the framework to map the error to a correct HTTP 500 equivalent.
