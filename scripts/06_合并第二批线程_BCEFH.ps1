$ErrorActionPreference = "Stop"

$ProjectRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$CurrentRoot = git rev-parse --show-toplevel 2>$null

if (-not $CurrentRoot) {
  Write-Host "当前目录不是 Git 仓库。请在主项目目录运行本脚本。"
  exit 1
}

if ([System.IO.Path]::GetFullPath($CurrentRoot) -ne [System.IO.Path]::GetFullPath($ProjectRoot)) {
  Write-Host "请在主项目目录运行本脚本: $ProjectRoot"
  exit 1
}

Set-Location $ProjectRoot

$dirty = git status --porcelain
if ($dirty) {
  Write-Host "当前主项目目录有未提交修改，请先提交后再合并。"
  git status --short
  exit 1
}

$branches = @(
  "codex/thread-entry-ui",
  "codex/thread-import-clean",
  "codex/thread-dedup-relation",
  "codex/thread-backup-jobs",
  "codex/thread-qa-regression"
)

foreach ($branch in $branches) {
  git show-ref --verify --quiet "refs/heads/$branch"
  if ($LASTEXITCODE -ne 0) {
    Write-Host "分支不存在，跳过: $branch"
    continue
  }

  Write-Host "正在合并: $branch"
  git merge --no-ff $branch
  if ($LASTEXITCODE -ne 0) {
    Write-Host "合并失败，请解决冲突后继续: $branch"
    exit 1
  }
}

Write-Host "第二批线程 B / C / E / F / H 合并完成。"
