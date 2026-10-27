### Model A

Resolved model ids: deepseek/deepseek-v4.1-flash-1017

| Arm | First pass | 95% CI | After repair | 95% CI | Fidelity (of valid) | Fidelity (of scored) | Attempts | Prompt tokens | Completion tokens | Cost (USD) |
|---|---|---|---|---|---|---|---|---|---|---|
| mermaid | 192/200 (96.0%) | [92.3, 98.0] | 198/200 (99.0%) | [96.4, 99.7] | 172/198 (86.9%) | 172/200 (86.0%) | 212 | 168814 | 141002 | 0.0906 |
| voxdiagram | 196/200 (98.0%) | [95.0, 99.2] | 200/200 (100.0%) | [98.1, 100.0] | 193/200 (96.5%) | 193/200 (96.5%) | 204 | 148423 | 89615 | 0.0564 |

### Model B

Resolved model ids: xiaomi/mimo-v2.6-pro

| Arm | First pass | 95% CI | After repair | 95% CI | Fidelity (of valid) | Fidelity (of scored) | Attempts | Prompt tokens | Completion tokens | Cost (USD) |
|---|---|---|---|---|---|---|---|---|---|---|
| mermaid | 195/199 (98.0%) | [94.9, 99.2] | 198/199 (99.5%) | [97.2, 99.9] | 140/198 (70.7%) | 140/199 (70.4%) | 206 | 157055 | 69452 | 0.0730 |
| voxdiagram | 194/199 (97.5%) | [94.3, 98.9] | 199/199 (100.0%) | [98.1, 100.0] | 182/199 (91.5%) | 182/199 (91.5%) | 205 | 149235 | 48422 | 0.0649 |

### Paired difference (VoxDiagram minus Mermaid)

| Endpoint | Model | a | b | c | d | Δ (points) | Newcombe 95% CI | Verdict | McNemar p |
|---|---|---|---|---|---|---|---|---|---|
| first pass | A | 188 | 8 | 4 | 0 | +2.0 | [-1.7, +5.9] | non-inferior | 0.3877 |
| first pass | B | 190 | 4 | 5 | 0 | -0.5 | [-4.0, +2.9] | non-inferior | 1.0000 |
| after repair | A | 198 | 2 | 0 | 0 | +1.0 | [-1.0, +3.6] | non-inferior | 0.5000 |
| after repair | B | 198 | 1 | 0 | 0 | +0.5 | [-1.4, +2.8] | non-inferior | 1.0000 |

### Per kind

| Model | Kind | Arm | n | First pass | After repair |
|---|---|---|---|---|---|
| A | graph | mermaid | 184 | 176/184 (95.7%) | 182/184 (98.9%) |
| A | graph | voxdiagram | 184 | 180/184 (97.8%) | 184/184 (100.0%) |
| A | sequence | mermaid | 16 | 16/16 (100.0%) | 16/16 (100.0%) |
| A | sequence | voxdiagram | 16 | 16/16 (100.0%) | 16/16 (100.0%) |
| B | graph | mermaid | 183 | 179/183 (97.8%) | 182/183 (99.5%) |
| B | graph | voxdiagram | 183 | 178/183 (97.3%) | 183/183 (100.0%) |
| B | sequence | mermaid | 16 | 16/16 (100.0%) | 16/16 (100.0%) |
| B | sequence | voxdiagram | 16 | 16/16 (100.0%) | 16/16 (100.0%) |

Completeness, model A: complete; 200 scored, 0 excluded for transport errors, 0 missing.
Completeness, model B: complete; 199 scored, 1 excluded for transport errors (g056), 0 missing.

Primary endpoint verdict (model A, first pass): non-inferior
