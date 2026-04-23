---
title: "Standard Library Built-ins"
description: "Core execution environment capabilities exposed in Vox (std.* and built-ins)."
category: "reference"
status: "current"
last_updated: "2026-04-06"
training_eligible: true

schema_type: "TechArticle"
---

# Reference: Standard Library Built-ins

Vox includes a minimal, highly optimized standard library focused exclusively on system I/O, core conversions, and process lifecycle capabilities inherently trusted by the compiler orchestrator.

## Global Built-ins

These core functions are evaluated globally across any lexical space in the application without module imports.

| Signature | Description |
|-----------|-------------|
| `fn len(collection: T) -> int` | Returns the number of elements in a sequence, string, list, or mapping dictionary structure. |
| `fn str(val: T) -> str` | Explicitly coerces arbitrary object types and scalar values strictly into UTF-8 strings. |
| `fn assert(condition: bool) -> Unit` | Halts execution contexts raising terminal logic failures safely. |
| `fn print(message: str) -> Unit` | Synchronous STDOUT writer. |

## Process and Execution IO (`std.fs.*`)

File system operations interact securely via WASI/os permission mappings. Error cascades explicitly require `Result`.

| Signature | Description |
|-----------|-------------|
| `fn read(path: str) -> Result[str]` | Reads file at `path` as UTF-8 text. Returns `Error(msg)` if not found or unreadable. |
| `fn write(path: str, content: str) -> Result[Unit]` | Creates or completely overwrites the target file with the string content. |
| `fn exists(path: str) -> bool` | Evaluates whether a file or directory exists at the given path. |
| `fn is_file(path: str) -> bool` | Returns true if the path is a file. |
| `fn is_dir(path: str) -> bool` | Returns true if the path is a directory. |
| `fn canonicalize(path: str) -> Result[str]` | Returns the canonical, absolute form of the path. |
| `fn list_dir(path: str) -> Result[list[str]]` | Returns a list of filenames in the directory. |
| `fn glob(pattern: str) -> Result[list[str]]` | Returns a list of paths matching the glob pattern. |
| `fn remove(path: str) -> Result[Unit]` | Removes the file at the given path. |
| `fn read_bytes(path: str) -> Result[str]` | Reads raw bytes as a string representation. |
| `fn mkdir(path: str) -> Result[Unit]` | Creates a single directory at the given path. |
| `fn copy(src: str, dst: str) -> Result[Unit]` | Copies a file from source to destination. |
| `fn remove_dir_all(path: str) -> Result[Unit]` | Recursively removes a directory and all of its contents. |

## Path Manipulation (`std.path.*`)

| Signature | Description |
|-----------|-------------|
| `fn join(a: str, b: str) -> str` | Joins two path parts. |
| `fn join_many(parts: list[str]) -> str` | Joins a list of path parts. |
| `fn basename(p: str) -> str` | Extracts the base name from a path. |
| `fn dirname(p: str) -> str` | Extracts the directory name from a path. |
| `fn extension(p: str) -> str` | Extracts the file extension. |

## Environment (`std.env.*`)

| Signature | Description |
|-----------|-------------|
| `fn get(key: str) -> Option[str]` | Retrieves an environment variable. |

## Process Execution (`std.process.*`)

| Signature | Description |
|-----------|-------------|
| `fn which(cmd: str) -> Option[str]` | Finds a command in the PATH. |
| `fn run(cmd: str, args: list[str]) -> Result[int]` | Runs a command and returns the exit code. |
| `fn run_ex(cmd: str, args: list[str], cwd: str, env: map[str, str]) -> Result[int]` | Runs a command with specific cwd and environment. |
| `fn run_capture(cmd: str, args: list[str]) -> Result[{exit: int, stdout: str, stderr: str}]` | Runs a command and captures its output. |
| `fn exit(code: int) -> never` | Terminates the process with the given exit code. |

## JSON Processing (`std.json.*`)

| Signature | Description |
|-----------|-------------|
| `fn read_str(json: str, path: str) -> Result[str]` | Extracts a string from a JSON document at the given path. |
| `fn read_f64(json: str, path: str) -> Result[float]` | Extracts a float from JSON. |
| `fn quote(s: str) -> str` | Properly escapes a string for inclusion in JSON. |

## Cryptography (`std.crypto.*`)

| Signature | Description |
|-----------|-------------|
| `fn hash_fast(s: str) -> str` | Fast, non-cryptographic hash. |
| `fn hash_secure(s: str) -> str` | Secure cryptographic hash (SHA-256). |
| `fn uuid() -> str` | Generates a UUID v4 string. |

## Time (`std.time.*`)

| Signature | Description |
|-----------|-------------|
| `fn now_ms() -> int` | Returns current UNIX timestamp in milliseconds. |

## Logging (`std.log.*`)

| Signature | Description |
|-----------|-------------|
| `fn debug(msg: str) -> Unit` | Logs a debug message. |
| `fn info(msg: str) -> Unit` | Logs an info message. |
| `fn warn(msg: str) -> Unit` | Logs a warning message. |
| `fn error(msg: str) -> Unit` | Logs an error message. |

## OpenClaw Invocation (`OpenClaw.*`)

| Signature | Description |
|-----------|-------------|
| `fn list_skills() -> Result[str]` | Lists available OpenClaw skills. |
| `fn call(skill: str, args: str) -> Result[str]` | Invokes an OpenClaw skill. |
| `fn subscribe(topic: str) -> Result[str]` | Subscribes to an OpenClaw topic. |
| `fn unsubscribe(topic: str) -> Result[str]` | Unsubscribes from an OpenClaw topic. |
| `fn notify(topic: str, msg: str) -> Result[str]` | Notifies an OpenClaw topic. |

## CDP System Automation (`Browser.*`)

*Note: These are native-script only (not available when compiled to WASM).*

| Signature | Description |
|-----------|-------------|
| `fn open() -> Result[Unit]` | Opens the default automation browser. |
| `fn close() -> Result[Unit]` | Closes the automation browser. |
| `fn goto(url: str) -> Result[Unit]` | Navigates to a specific URL. |
| `fn click(selector: str) -> Result[Unit]` | Clicks on the DOM element matched by selector. |
| `fn fill(selector: str, value: str) -> Result[Unit]` | Fills a DOM element with a text value. |
| `fn wait_for(selector: str) -> Result[Unit]` | Waits for a selector to appear on the page. |
| `fn text(selector: str) -> Result[str]` | Returns the inner text of an element. |
| `fn html(selector: str) -> Result[str]` | Returns the inner HTML of an element. |
| `fn screenshot(path: str) -> Result[Unit]` | Takes a screenshot and saves it to the path. |

## Network (`std.http.*`)

| Signature | Description |
|-----------|-------------|
| `fn get_text(url: str) -> Result[str]` | Submits an HTTP GET request to the target URL and returns the response body as text. |
| `fn post_json(url: str, body: str) -> Result[str]` | Submits an HTTP POST request to the target URL with the provided JSON body string. |

---

**Related Topics**:
- [Reference: Database Query Surface](ref-db-surface.md)
- [Explanation: The Runtime](../explanation/expl-runtime.md)
