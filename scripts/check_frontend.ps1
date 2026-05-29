$ErrorActionPreference = "Stop"

$Root = Resolve-Path "$PSScriptRoot\.."
Set-Location $Root

Write-Host "== Frontend type check =="
npm --prefix frontend run check

Write-Host "== Frontend build =="
npm --prefix frontend run build

Write-Host "Frontend checks passed."
