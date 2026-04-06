---
title: "Standard Library Reference"
description: "Core execution environment capabilities exposed in Vox (std.* and built-ins)."
category: "reference"
status: "current"
last_updated: "2026-04-06"
training_eligible: true
---

# Reference: Standard Library

Vox includes a small but highly optimized standard library focused exclusively on system I/O, core conversions, and process lifecycle capabilities inherently trusted by the compiler orchestrator logic. 

## Global Built-ins 

These core functions form the functional spine evaluated globally across any lexical space in the application without module imports. 

- `len(collection) to int`  
  *Returns the number of elements found in a generic sequence, string, list, or mapping dictionary structure.*
- `str(val) to str`  
  *Explicitly coerces arbitrary object types and scalar values strictly into UTF-8 strings. Equivalent to standard Display/ToString Rust conversions.*
- `assert(condition: bool) to Unit`  
  *Halts execution contexts raising terminal logic failures safely during `vox test` verification routines. Replaces previous testing validation frameworks.*
- `print(message: str) to Unit`  
  *Synchronous STDOUT writer equivalent to `std.log.info(message)`. Does not perform distributed tracking propagation across DEI bounds.*

## Process and Execution IO (`std.fs.*`)

File system operations interact securely via WASI/os permission mappings. Error cascades explicitly require `Result` extraction logic via `match` or `?`. 

- `std.fs.read(path: str) to Result[str]`  
  *Inhales the target text file synchronously loading UTF-8 formats safely.*
- `std.fs.write(path: str, content: str) to Result[Unit]`  
  *Creates or targets and injects fully replaced sequences over the target file map.*
- `std.fs.exists(path: str) to bool`  
  *Fast evaluation to verify file path locations without trapping result structures.*

## Network Context Handlers 

- `std.http.get(url: str) to Result[str]`
- `std.http.post_json(url: str, body: json) to Result[str]`  
  *(Currently executed asynchronously beneath Rust reqwest implementations and handled intrinsically).*

## Actor Execution Lifecycle Context 

Actors function transparently but require explicit interaction over state persistence markers. 

- `state_load(key: str) to T`  
  *Deserializes stored actor persistence states linked identically onto its parent Actor runtime definition key namespace.*
- `state_save(key: str, val: T) to Unit`  
  *Marshals the data object dynamically onto the core database storage linked exactly across its current sequence id runtime tracking.*

---

**Related Topics**:
- [Reference: Database Query Surface](ref-db-surface.md)
- [Explanation: The Runtime](../explanation/expl-runtime.md)
