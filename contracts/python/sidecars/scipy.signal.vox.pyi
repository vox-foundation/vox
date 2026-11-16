# Curated Tier 2 Python sidecar for `scipy.signal`
# vox-module: scipy.signal

fn welch(x: list[float], nperseg: int) -> list[float]
fn periodogram(x: list[float]) -> list[float]
fn savgol_filter(x: list[float], window_length: int, polyorder: int) -> list[float]
fn find_peaks(x: list[float], height: float) -> list[int]
