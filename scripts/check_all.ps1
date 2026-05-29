$ErrorActionPreference = "Stop"

$Root = Resolve-Path "$PSScriptRoot\.."
Set-Location $Root

& "$PSScriptRoot\check_frontend.ps1"
& "$PSScriptRoot\check_backend.ps1"

Write-Host "All checks passed."
