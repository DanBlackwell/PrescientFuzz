# Tinyinst example
This is a fuzzer example to show how libafl_tinyinst works

## How to build
1. Build the harness with `cl test\test.cpp -o test.exe`
2. Build the fuzzer with `cargo build --release`. The fuzzer is `target\release\tinyinst_simple.exe`

## Run with cargo-make
Or, you can simple run it using cargo-make
1. Open up developer powershell so that you have access to cl (Windows Default Compiler)
2. Run `cargo make run` to run the fuzzer