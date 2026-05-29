$ErrorActionPreference = "Stop"

$Root = Resolve-Path "$PSScriptRoot\.."
Set-Location $Root

Write-Host "== Rust format check =="
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check

Write-Host "== Rust tests =="
cargo test --manifest-path src-tauri/Cargo.toml

Write-Host "== Rust cargo check =="
cargo check --manifest-path src-tauri/Cargo.toml

Write-Host "Backend checks passed."
