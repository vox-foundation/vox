@echo off
setlocal
call "C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvars64.bat"
if %errorlevel% neq 0 (
    echo MSVC init failed
    exit /b 1
)
set VOX_TRAIN_SKIP_CORPUS_MIX=1
cargo run --release -p vox-cli --features gpu,populi-candle-cuda -- populi train --backend qlora --tokenizer hf --preset qwen_4080_16g --model Qwen/Qwen2.5-Coder-3B-Instruct --data-dir C:\Users\Owner\vox\target\dogfood --output-dir C:\Users\Owner\vox\populi\runs\qwen25_qlora --min-rating 0
endlocal
