SYNTHETIC FIXTURE: not a measurement

### Model A

Resolved model ids: synthetic-none

| Arm | First pass | 95% CI | After repair | 95% CI | Fidelity (of valid) | Fidelity (of scored) | Attempts | Prompt tokens | Completion tokens | Cost (USD) |
|---|---|---|---|---|---|---|---|---|---|---|
| mermaid | 2/3 (66.7%) | [20.8, 93.9] | 2/3 (66.7%) | [20.8, 93.9] | 2/2 (100.0%) | 2/3 (66.7%) | 6 | 3600 | 480 | 0.0000 |
| voxdiagram | 2/3 (66.7%) | [20.8, 93.9] | 3/3 (100.0%) | [43.9, 100.0] | 3/3 (100.0%) | 3/3 (100.0%) | 5 | 3000 | 400 | 0.0000 |

### Model B

Resolved model ids: synthetic-none-b

| Arm | First pass | 95% CI | After repair | 95% CI | Fidelity (of valid) | Fidelity (of scored) | Attempts | Prompt tokens | Completion tokens | Cost (USD) |
|---|---|---|---|---|---|---|---|---|---|---|
| mermaid | 1/1 (100.0%) | [20.7, 100.0] | 1/1 (100.0%) | [20.7, 100.0] | 1/1 (100.0%) | 1/1 (100.0%) | 1 | 590 | 80 | 0.0000 |
| voxdiagram | 1/1 (100.0%) | [20.7, 100.0] | 1/1 (100.0%) | [20.7, 100.0] | 1/1 (100.0%) | 1/1 (100.0%) | 1 | 610 | 80 | 0.0000 |

### Paired difference (VoxDiagram minus Mermaid)

| Endpoint | Model | a | b | c | d | Δ (points) | Newcombe 95% CI | Verdict | McNemar p |
|---|---|---|---|---|---|---|---|---|---|
| first pass | A | 1 | 1 | 1 | 0 | +0.0 | [-64.0, +64.0] | inconclusive | 1.0000 |
| first pass | B | 1 | 0 | 0 | 0 | +0.0 | [-79.3, +79.3] | inconclusive | 1.0000 |
| after repair | A | 2 | 1 | 0 | 0 | +33.3 | [-29.1, +79.2] | inconclusive | 1.0000 |
| after repair | B | 1 | 0 | 0 | 0 | +0.0 | [-79.3, +79.3] | inconclusive | 1.0000 |

### Per kind

| Model | Kind | Arm | n | First pass | After repair |
|---|---|---|---|---|---|
| A | graph | mermaid | 2 | 1/2 (50.0%) | 1/2 (50.0%) |
| A | graph | voxdiagram | 2 | 2/2 (100.0%) | 2/2 (100.0%) |
| A | sequence | mermaid | 1 | 1/1 (100.0%) | 1/1 (100.0%) |
| A | sequence | voxdiagram | 1 | 0/1 (0.0%) | 1/1 (100.0%) |
| B | graph | mermaid | 1 | 1/1 (100.0%) | 1/1 (100.0%) |
| B | graph | voxdiagram | 1 | 1/1 (100.0%) | 1/1 (100.0%) |

Completeness, model A: incomplete (1 infra exclusions exceed 5.0% of 4 tasks); 3 scored, 1 excluded for transport errors (g003), 0 missing.
Completeness, model B: incomplete (3 of 4 tasks lack a first-pass attempt in an arm); 1 scored, 0 excluded for transport errors, 3 missing.

Primary endpoint verdict (model A, first pass): not determined
Verdict withheld: model A incomplete: 1 infra exclusions exceed 5.0% of 4 tasks.
