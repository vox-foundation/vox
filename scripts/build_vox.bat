@echo off
call "C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvars64.bat"
cargo build --release -p vox-cli --features gpu,populi-candle-cuda,populi-dei
exit /b %errorlevel%
