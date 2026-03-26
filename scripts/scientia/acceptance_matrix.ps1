#requires -Version 7.2
$ErrorActionPreference = 'Stop'
# SCIENTIA-focused acceptance slice (DB integration + publisher remote-status mapping unit tests).
$cargo = Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe'
if (-not (Test-Path -LiteralPath $cargo)) { $cargo = 'cargo' }

& $cargo test -p vox-db --test publication_flow_tests
& $cargo test -p vox-publisher --features scholarly-external-jobs scholarly_remote_status -- --nocapture
