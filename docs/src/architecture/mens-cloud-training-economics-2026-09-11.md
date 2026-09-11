---
title: "MENS Cloud Training Economics (surveyed 2026-09-11)"
description: "When renting a GPU beats the 128 GB local machine for a 27-32B QLoRA, what it costs across twelve providers with per-row provenance, and the gotchas that dominate the bill."
category: "Architecture SSOTs"
status: "research"
training_eligible: false
---

## 1. The default is local

The measured baseline for MENS fine-tuning on this machine (Apple M5 Max, 128 GB
unified memory) is: **41.825 GB peak** against a **107.52 GiB** Metal working
set, **38.96 tok/s**, **~2 h 10 m** wall clock, val loss **2.974 → 0.403**,
**$0**, **zero OOM**. That is 39% utilization of the working set with no
crashes on a run that already completed successfully.

Cloud GPU rental is a **gated contingency**, not a peer option to local
training. It is justified only when one of four trigger conditions fires:

1. **A capacity wall** — dense ~70B+ parameters at 8-bit, a full (non-LoRA)
   fine-tune above ~13B, or any model above ~100B.
2. **Four or more concurrent runs** — parallel sweeps. Cloud's real product
   here is **width**, not depth: renting six single-GPU instances to run six
   hyperparameter arms in parallel for ~15 minutes each at ~$3 total beats
   serializing the same six arms locally over 13 hours.
3. **A single run projected to exceed 12 hours.**
4. **Sequence-length pressure** against the measured 80.64 GiB single-buffer
   cap on this machine.

State plainly: **the cloud's product here is width, not depth.** One run
finishing 2x faster on a rented GPU does not beat a free run that already
finishes. Six arms in parallel for $3 does beat 13 hours serialized on one
machine — that is the only shape of win cloud offers at this job size.

## 2. The shape of the decision

A 27–32B QLoRA fine-tune trains on **one GPU**. The only all-reduce in scope
is over LoRA adapter parameters, which are small enough that multi-GPU
data/model parallelism buys nothing at this scale. Every offering that only
sells 8-GPU nodes is therefore **structurally wrong** for this job — not
merely expensive, but incapable of being used as anything but one paid-for
GPU with seven idle siblings.

This is the rule Task 1 encodes as `UnsuitableReason::MultiGpuNode`.

## 3. The survey table

Every row carries provider, SKU, on-demand $/hr, spot $/hr, billing
granularity, and a confidence flag:

- **V** — provider's own pricing page or official API.
- **S** — third-party/secondary source.
- **U** — unverified (page unreachable, or the figure was not published).

All figures fetched **2026-09-11**.

### Single-GPU-capable, VRAM >= 80 GB, sorted by on-demand rate

| Provider | SKU (VRAM) | On-demand $/hr | Spot $/hr | Billing | Conf. |
|---|---|---|---|---|---|
| Vast.ai | RTX PRO 6000 Max-Q (96 GB) | from $0.92, median **$1.55** | not published (**U**) | per-second | V |
| Vast.ai | RTX PRO 6000 WS (96 GB) | from $0.96, median **$1.39** | not published (**U**) | per-second | V |
| RunPod | RTX PRO 6000 | Community **$1.69** / Secure $2.09 | not published (**U**) | per-second | V |
| Vast.ai | H100 SXM (80 GB) | from $1.73, median **$2.09** | not published (**U**) | per-second | V |
| Hyperstack | RTX Pro 6000 SE (96 GB) | $1.85 | **$1.48** | per-minute | V |
| Vast.ai | H200 (141 GB) | from $1.97, median $4.61 | not published (**U**) | per-second | V |
| Voltage Park | H100 (Ethernet) | from **$1.99** | not offered | hourly | V |
| Vast.ai | A100 SXM4 (80 GB) | from $0.34, median $0.93 | not published (**U**) | per-second | V |
| Lambda | GH200 (96 GB) | $2.29 | none offered | per-minute | V |
| Prime Intellect | H100 (80 GB) | $2.43 | $0.94 | unverified | V (see caveat) |
| RunPod | H100 SXM (80 GB) | Community **$2.69** / Secure $3.49 | not published (**U**) | per-second | V |
| Hyperstack | H100 SXM | $3.20 | not offered on SXM | per-minute | V |
| Verda (ex-DataCrunch) | H100 SXM5 (80 GB) | $3.25 | **$1.63** | unverified | V |
| Nebius | H100 | $3.85 | **$2.15** | unverified | V |
| Together AI | HGX H100 | $3.99 | **preemptible $1.99** | hourly | V |
| Lambda | H100 SXM (80 GB) | $4.29 | none offered | per-minute | V |
| Google Cloud | a2-ultragpu-1g (1x A100 80 GB) | $5.0688 | $2.9279 | unverified | V |
| AWS | **p5.4xlarge (1x H100 80 GB)** | $6.88 (us-east-1) | $2.63 | unverified | V / **S** (spot) |

### Node-granular only — listed so the arithmetic is on the record, not as candidates

| Provider | SKU | $/hr (node) | $/hr per GPU | Single GPU? | Conf. |
|---|---|---|---|---|---|
| CoreWeave | HGX H100 (8x) | $49.24 | $6.16 | **No** | V |
| Google Cloud | a3-highgpu-8g (8x H100) | $88.49 (spot $50.46) | $11.06 | **No** | V |
| Azure | ND96isr H100 v5 (8x) | $98.32 (spot $18.17) | $12.29 | **No** | V |

Renting an Azure ND96isr to run one QLoRA is **~$236 for a 13-hour run at the
*spot* price** and ~$1,278 on demand, against ~$18–27 for the same
silicon-class on a single-GPU provider. GCP's a3-highgpu-8g spot is worse
still (~$656). These are not close calls.

## 4. Throughput — what is measured and what is inferred

| Device | Memory bandwidth | Ratio vs M5 Max | Conf. |
|---|---|---|---|
| M5 Max 128 GB (40-core) | **614 GB/s** | 1.00x | **VERIFIED** (Apple tech specs) |
| RTX PRO 6000 Blackwell | **1,792 GB/s** | 2.92x | VERIFIED |
| H100 SXM | **3.35 TB/s** | 5.46x | VERIFIED |
| H200 | **4.8 TB/s** | 7.82x | VERIFIED |

One clean measured LoRA fine-tuning datapoint exists: **Llama-3.1-8B on 1x
RTX PRO 6000 Server Edition = 4,962 tok/s** (Exxact, 2026-06-04, **VERIFIED**).
Scaling to ~27B by parameter count gives ~1,500 tok/s against the measured
38–56 tok/s local — **~27–39x, but EXTRAPOLATED across a different model
family, framework, and quantization.**

**State the honest range as 10–30x, inferred. No direct 27B-class benchmark
exists on either side of the comparison.** A bandwidth ratio is not a
throughput ratio, and published fine-tuning throughput numbers are unusually
dirty — one widely-cited 46,000 tok/s headline figure turned out to have
**zero gradient flow**.

## 5. Gotchas, ranked by likelihood of biting a 2–6h single-GPU LoRA run

This section, not the $/hr table, is where the money actually goes.

1. **Egress can dominate the bill, and it is not in the sort key.** Vast.ai
   egress is **host-set per GB**, rates ranging $0.00–$0.10/GB. A published
   itemized vast.ai bill: GPU 0.92h x $0.27 = **$0.25**; download 106 GB x
   $0.04/GB = **$4.14** — **16x the compute cost**. Listings sort by $/hr,
   which excludes the dominant term. (**VERIFIED**, vast.ai billing
   documentation / itemized invoice, fetched 2026-09-11.)
2. **Storage bills until you DELETE, not STOP.** RunPod's volume rate
   *doubles* when a pod is stopped: $0.10 -> **$0.20/GB/mo**. (**VERIFIED**,
   RunPod pricing page, 2026-09-11.)
3. **A stopped RunPod pod frequently cannot restart** — the GPU gets rented to
   someone else while you are stopped. RunPod maintains a dedicated
   troubleshooting page for this failure mode. (**VERIFIED**, RunPod docs,
   2026-09-11.)
4. **No usable grace period on reclaim.** Vast.ai documents none. RunPod's
   "5-second SIGTERM" is **REPORTED** only (third-party blog; not in current
   RunPod primary docs). Five seconds cannot flush a 27B checkpoint.
   Checkpoint on a **fixed iteration cadence**, not on a termination signal.
5. **Data posture — the constraint that decides the provider.** Vast.ai's own
   security FAQ states *"Provider security varies significantly"* and
   recommends restricting to certified providers (**VERIFIED**, vast.ai
   security FAQ, 2026-09-11). Container isolation defends against other
   renters; it does **not** defend against a host with physical root. **The
   training corpus is the user's proprietary codebase.** That is the deciding
   fact, not the $/hr column.

## 6. Uptime SLAs — three providers publish one, contrary to the earlier draft

| Provider | Numeric SLA | Credits | Conf. |
|---|---|---|---|
| **Voltage Park** | **>=99.5% per VM** | 10% (95–99.5%), 25% (90–95%), **100% (<90%)**, 30-day claim window | **VERIFIED** — the most concrete self-serve GPU SLA found |
| **Nebius** | **99.50%** Compute Cloud | graduated 10–30% compensation | **VERIFIED** |
| **Hyperstack** | published but **self-contradictory** — the same document states 99.5% in one section and 100.0% in another | — | **VERIFIED as unreliable** — do not budget against it |
| **vast.ai** | **NONE, explicitly disclaimed.** ToS: *"Company cannot guarantee the Website and that Company Services will be always available."* No credits, no uptime number, no tier that adds one; liability capped at 3 months of fees. "Secure Cloud" on vast.ai is a host-selection **filter**, not a contractual product. | none | **VERIFIED** |
| **RunPod** | **not published for self-serve.** ToS carves out explicitly: *"Runpod does not make any specific uptime warranties with respect to the Community Cloud Offerings."* The 99.99% figure appears only on the enterprise sales page with no linked SLA document. | none self-serve | **VERIFIED** |
| **Lambda** | **none** — ToS is AS-IS. Third-party "99.9%" claims are contradicted by it. | none | **VERIFIED** |
| **Prime Intellect** | **none**, stated outright | none | **VERIFIED** |
| **CoreWeave** | object storage only; **not compute** | n/a | **VERIFIED** |

All fetched 2026-09-11 from each provider's own ToS / SLA / pricing pages.

## 7. Recommendation

- **Default: do not rent.** Run locally. 41.8 GB peak on a 107.52 GiB working
  set at $0 is not a problem looking for a purchase.
- **When a trigger fires: RunPod Secure Cloud, on-demand.** Not vast.ai, not
  spot. You are buying a **trust boundary and predictable egress**, not the
  lowest $/hr — and at single-digit dollars per run, the marketplace discount
  is noise against a proprietary-corpus exposure. RTX PRO 6000 at
  **$2.09/hr** Secure, or H100 SXM at $3.49.
- **Always `terminate`, never `stop`** (gotchas 2 and 3). Checkpoint on a
  fixed iteration cadence regardless of lane (gotcha 4).
- **If the marketplace is acceptable for a specific run** (public corpus
  only): Vast.ai 1x RTX PRO 6000 Blackwell 96 GB, median $1.39–1.55/hr,
  per-second billing — but **price the egress before you sort by $/hr**
  (gotcha 1).
- **If H100 specifically is required from a first-party operator: Voltage
  Park at $1.99/hr** — cheapest verified single-H100, and the only one in
  this survey with a real credit ladder behind a numeric SLA.
- **Budget $20–35 per run** in the sane tier; with egress and one restart in
  three, plan **~$45/run all-in**.
- **Spot is not worth it at this job size.** The discount is ~$10–15 on a
  ~$30 run, against a bid market with no published grace period and a
  documented restart failure mode.

## 8. Rejected, with the reason

- **GCP A3, Azure ND H100 v5, CoreWeave** — no shape smaller than 8 GPUs.
  Structurally wrong; the arithmetic is in §3.
- **AWS p5.4xlarge** — a genuine single-H100 shape, but $6.88/hr on demand is
  3–5x the neoclouds. Its spot price ($2.63, **SECONDARY**) is the only
  hyperscaler number in range.
- **GCP a2-ultragpu-1g** — the one GCP single-GPU escape hatch. Workable, but
  Ampere-slow and right at the VRAM floor at $5.07/hr.
- **RTX 5090 (32 GB) and RTX 4090 (24 GB)** — cheapest per hour on the board
  and **cannot hold the working set**. They appear only so the filter's
  `InsufficientVram` rejection has something to reject.

## 9. Stated gaps — do not budget against these

- **Vast.ai interruptible $/hr: UNVERIFIED.** Per-host bid market, no
  published rate; several `docs.vast.ai` interruptible pages 404'd.
- **Vast.ai storage and bandwidth $/GB: UNVERIFIED as a single number** —
  host-set, $0.00–$0.10/GB observed. Part of the bill cannot be priced before
  choosing a host. See gotcha 1.
- **RunPod spot rate: UNVERIFIED.** No figure published anywhere reached.
- **Billing granularity** for GCP, AWS, Azure, Nebius, CoreWeave, Verda,
  Voltage Park, and Prime Intellect was not confirmed from a fetched page.
- **Prime Intellect** — the homepage marketplace widget repeats `$3.14/HR`
  across mismatched GPU/VRAM labels. Treat everything except its H100 row as
  UNVERIFIED.
- **GCP region** — figures are the page's default region as rendered; the
  region selector could not be read back.
- **No 27B-class fine-tuning throughput benchmark exists** on either the
  local or the rented side. The 10–30x figure in §4 is inferred, not
  measured.
- **This whole table is a snapshot dated 2026-09-11.** The survey date is in
  the filename and the title precisely so a stale row is visible rather than
  silently trusted. The code holds no prices; `cloud-estimate` reads live
  rates from the provider APIs.

## 10. Link back to the code

The suitability rules in §2 and §8 are executable at
`crates/vox-populi/src/mens/cloud/offer_filter.rs`, not prose. The command
that prints the live version of this table — local row first — is
`vox mens cloud-estimate`.
