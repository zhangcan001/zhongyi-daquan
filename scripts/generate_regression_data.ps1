$ErrorActionPreference = "Stop"

$Root = Resolve-Path "$PSScriptRoot\.."
Set-Location $Root

$DbPath = if ($args.Length -gt 0) { $args[0] } else { "local-data/database/thread_h_regression.db" }

cargo run --manifest-path src-tauri/Cargo.toml --example generate_regression_data -- --db $DbPath --check-performance
