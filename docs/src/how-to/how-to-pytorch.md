---
title: "@py.import – Python Library Integration (`torch`, `numpy`, etc.)"
description: "Official documentation for @py.import – Python Library Integration (`torch`, `numpy`, etc.) for the Vox language."
category: "how-to"
last_updated: 2026-03-24
training_eligible: true
---
# @py.import – Python Library Integration (`torch`, `numpy`, etc.)

> **2026 stance:** **`vox container init` is retired** (hard error — use Rust/PM flows). **`@py.import` / uv-backed setup is not a supported product path.** Native ML stacks live under **`vox mens`** / Candle; treat the material below as historical reference only.
> For integration with external libraries via FFI going forward, see [Rust FFI & Migration Guide](../architecture/rust-ecosystem-support-ssot.md).

Vox historically documented importing Python libraries from `.vox` via `@py.import` with **[uv](https://docs.astral.sh/uv/)** for wheels. That workflow is **not** maintained as a supported package-management lane.

## Quick Start

```vox
// vox:skip
@py.import torch
@py.import torch.nn as nn

fn run_inference(input: list[float]) -> list[float] {
    let t = torch.tensor(input)
    let model = nn.Linear(4, 1)
    return model.forward(t).tolist()
}
```

Legacy documentation previously recommended:

```bash
vox container init --file src/main.vox
```

That command now **fails** with a migration message — do not rely on it for new work.

## Syntax

```
@py.import <module>                   # binds to last segment (torch → torch)
@py.import <module> as <alias>        # custom binding (torch.nn → nn)
```

Both dotted module paths (`torch.nn.functional`) and simple names (`torch`) are supported.

## How It Worked (historical)

The retired `vox container init` flow used **uv** as follows:

1. Detects your environment (uv, Python version, GPU/CUDA).
2. Runs `uv python install 3.12` — idempotent, skips if already installed.
3. Generates a `pyproject.toml` with the correct PyTorch wheel source (CPU or CUDA).
4. Runs `uv sync` — creates `.venv` in your project directory.

At runtime, the `vox-py` bridge auto-detects the `.venv` and injects its `site-packages` into Python's `sys.path`. **No `PYTHONPATH` or shell activation is needed.**

### venv discovery order

The runtime looks for the venv in this order:

| Priority | Source |
|---|---|
| 1 | `UV_PROJECT_ENVIRONMENT` env var (set by `uv run`) |
| 2 | `VIRTUAL_ENV` env var (manual activation) |
| 3 | `.venv` in the current working directory |
| 4 | Subprocess query: `uv run python -c "import sys; print(sys.prefix)"` |

### Type conversions

Inputs are automatically converted from Vox types to Python types:

| Vox type      | Python type    |
|---------------|----------------|
| `int`         | `int`          |
| `float`       | `float`        |
| `str`         | `str`          |
| `bool`        | `bool`         |
| `list[T]`     | `list`         |
| `dict`        | `dict`         |

Return values come back as their string representation. Use helper utilities like `PY_RT.tensor_to_vec_f64()` to convert tensors to Vox-native lists, or `PY_RT.to_json()` for structured results.

## PyTorch Example

```vox
// vox:skip
@py.import torch
@py.import torch.nn as nn
@py.import torch.nn.functional as F

fn mlp_forward(x: list[float]) -> list[float] {
    let t       = torch.tensor(x)
    let linear1 = nn.Linear(4, 8)
    let linear2 = nn.Linear(8, 2)
    let h       = F.relu(linear1.forward(t))
    let out     = linear2.forward(h)
    return out.tolist()
}

fn main() {
    let result = mlp_forward([1.0, 2.0, 3.0, 4.0])
    println(result)
}
```

## NumPy Example

```vox
// vox:skip
@py.import numpy as np

fn moving_average(data: list[float], window: int) -> list[float] {
    let arr = np.array(data)
    let weights = np.ones(window) / window
    return np.convolve(arr, weights, "valid").tolist()
}
```

## Runtime Environment *(historical)*

**`vox container init` is retired** (hard error). It no longer provisions Python, **uv**, or a project venv. The snippet below is only for readers maintaining trees that still have a pre-existing `.venv` from before that cutover:

```bash
# Retired — fails today with an explicit migration message.
vox container init --file src/main.vox

# Historical follow-up only: rebuild a binary against an already-materialized venv layout.
cargo build && ./target/debug/my-app
```

### Docker / CI *(historical)*

The **`vox container init` + `uv sync` lane is retired.** The snippets below are retained only for readers maintaining old trees.

When the venv lives at a non-standard path (e.g. inside a Docker image), set `VOX_VENV_PATH` to override auto-detection:

```dockerfile
# Historical — prefer the repo-root Rust `Dockerfile` for new work.
FROM python:3.12-slim
RUN pip install uv

WORKDIR /app
COPY . .
RUN uv sync

# VOX_VENV_PATH tells the compiled binary exactly where packages live
ENV VOX_VENV_PATH=/app/.venv
CMD ["./target/release/my-app"]
```

Or in a CI step:

```yaml
# Historical uv-based CI — not a supported Vox PM path.
- run: |
    uv sync
    cargo build --release
    VOX_VENV_PATH=$(pwd)/.venv ./target/release/my-app
```

> [!TIP]
> For GPU workloads on the **historical** `@py.import` + CUDA wheel path, you needed an NVIDIA GPU so auto-detection could pick PyTorch wheels. **New work:** prefer **`vox mens`** / Candle — see [Mens training](../reference/mens-training.md).

> [!NOTE]
> The `vox-py` Cargo feature is disabled by default to keep compile times short. Enable it by adding `vox-py` as a dependency to your project's `Cargo.toml`.

> [!IMPORTANT]
> **Do not set `PYTHONPATH` manually.** The `vox-py` runtime discovers the uv-managed `.venv` automatically. Setting `PYTHONPATH` to a different environment will override this detection and may cause import errors.

## CUDA Configuration

Vox auto-selects the right PyTorch wheel source based on your detected GPU:

| Detected CUDA | PyTorch index |
|---|---|
| 13.x | `cu130` |
| 12.4–12.6 | `cu124` |
| 12.1–12.3 | `cu121` |
| 11.8 | `cu118` |
| None / CPU | `cpu` |

## Available Bridge Methods

| Method | Description |
|--------|-------------|
| `PY_RT.call_method(alias, method, args)` | Positional args |
| `PY_RT.call_method_kwargs(alias, method, args, kwargs)` | Positional + keyword args |
| `PY_RT.call_method0(alias, method)` | Zero-arg call |
| `PY_RT.get_attr(alias, attr_path)` | Get attribute value as string |
| `PY_RT.tensor_to_vec_f64(alias, repr)` | Extract tensor → `Vec<f64>` |
| `PY_RT.to_json(alias, expr)` | Extract any Python value → JSON |
| `PY_RT.eval(alias, expression)` | Evaluate arbitrary Python expression |


## See Also

- [`@py.import` decorator reference](../reference/ref-decorators.md)
- [Candle inference serving](../../../crates/vox-populi/src/mens/tensor/candle_inference_serve.rs)
- [NumPy integration patterns](how-to-ai-agents.md)

---

## The Future: Native Vox ML (`vox-tensor`)

While Python integration historically provided utility for `@py.import` experiments, it inherently conflicts with deeply-held Vox principles: **Zero dependency drift**, **One Binary deployment**, and **Complete cross-platform compilation**.

To address this, we have implemented **`vox-tensor`** — a native ML layer built on the [Burn](https://burn.dev) framework, providing 95% of PyTorch's capabilities without Python.

### Current API (implemented)

```rust
// Tensor creation
Tensor::zeros_1d(len)               // 1D zero tensor
Tensor::zeros_2d(rows, cols)        // 2D zero tensor
Tensor::ones_1d(len)                // 1D ones tensor
Tensor::ones_2d(rows, cols)         // 2D ones tensor
Tensor::from_vec_1d(data)           // 1D from Vec<f32>
Tensor::from_vec_2d(data, rows, cols) // 2D from Vec<f32>
Tensor::randn_1d(len)               // 1D random normal
Tensor::randn_2d(rows, cols)        // 2D random normal

// Operations
tensor.add(&other)           // element-wise add
tensor.sub(&other)           // element-wise subtract
tensor.mul(&other)           // element-wise multiply
tensor.mul_scalar(f64)       // scalar multiply
tensor.add_scalar(f64)       // scalar add
tensor.matmul(&other)        // matrix multiply (2D only)
tensor.transpose()           // transpose (2D only)
tensor.relu()                // ReLU activation
tensor.sigmoid()             // sigmoid activation
tensor.sum()                 // sum all elements
tensor.mean()                // mean all elements
tensor.to_vec()              // extract to Vec<f32>
tensor.shape()               // TensorShape
tensor.numel()               // total element count
```

### Neural Network Layers

```rust
// Layers
nn::Module::linear(in, out, bias)   // Dense layer
nn::Module::dropout(prob)           // Dropout
nn::Module::batch_norm1d(features)  // BatchNorm1d
nn::Module::conv2d(in_ch, out_ch, kernel) // Conv2d

// Composition
nn::Sequential::new(vec![
    Module::linear(4, 8, true),
    Module::linear(8, 2, true),
])
.forward(input_tensor)
```

### Example: MLP inference without Python

```vox
// vox:skip
import tensor as t
import nn

fn infer_mlp() -> list[float] {
    let model = nn.Sequential([
        nn.Module::linear(4, 8, true),
        nn.Module::linear(8, 2, true),
    ])

    let input = t.Tensor::from_vec_2d([1.0, 2.0, 3.0, 4.0], 1, 4)
    let out = model.forward(input)
    return out.to_vec()
}
```

This ensures **Low K-Complexity** (no shell dependencies), native type-checked operations, and deployment via the built-in HTTP server — all in a single, self-contained binary.

> [!NOTE]
> `vox-tensor` uses **NdArray** (CPU) as the default backend with **Autodiff** for gradient tracking.
> GPU acceleration (WGPU) is available via the `wgpu` feature flag in `vox-tensor/Cargo.toml`.
