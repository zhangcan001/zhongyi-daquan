$ErrorActionPreference = "Stop"

$ProjectRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location $ProjectRoot

$paths = @(
  "..\zy-thread-db",
  "..\zy-thread-search",
  "..\zy-thread-ai"
)

foreach ($path in $paths) {
  $fullPath = [System.IO.Path]::GetFullPath((Join-Path $ProjectRoot $path))
  if (Test-Path $fullPath) {
    Write-Host "打开: $fullPath"
    code $fullPath
  } else {
    Write-Host "未找到 worktree，跳过: $fullPath"
  }
}
