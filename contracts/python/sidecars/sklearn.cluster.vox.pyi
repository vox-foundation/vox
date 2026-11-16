# Curated Tier 2 Python sidecar for `sklearn.cluster`
# vox-module: sklearn.cluster

fn kmeans_labels(x: list[float], k: int) -> list[int]
fn dbscan_labels(x: list[float], eps: float, min_samples: int) -> list[int]
