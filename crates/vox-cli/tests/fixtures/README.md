# CLI test fixtures

- **`command_catalog_paths_baseline.txt`** — Sorted slash-separated paths of every `vox …` command discovered from clap (`command_catalog::build_catalog`). When the surface changes on purpose, refresh with:

  ```bash
  UPDATE_CLI_CATALOG_BASELINE=1 cargo test -p vox-cli --test command_catalog_paths_baseline
  ```

  (`cmd.exe` / PowerShell: `set UPDATE_CLI_CATALOG_BASELINE=1` then the same `cargo test` line.)
