$ErrorActionPreference = "Stop"

$ProjectRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location $ProjectRoot

Write-Host "项目目录: $ProjectRoot"

if (-not (Test-Path ".git")) {
  Write-Host "当前目录不是 Git 仓库，正在执行 git init..."
  git init
}

$hasCommit = $true
git rev-parse --verify HEAD *> $null
if ($LASTEXITCODE -ne 0) {
  $hasCommit = $false
}

if (-not $hasCommit) {
  Write-Host "当前仓库还没有任何提交。请先提交主项目骨架，再重新运行本脚本。"
  exit 1
}

$items = @(
  @{ Branch = "codex/thread-db-schema"; Path = "..\zy-thread-db" },
  @{ Branch = "codex/thread-entry-ui"; Path = "..\zy-thread-entry" },
  @{ Branch = "codex/thread-import-clean"; Path = "..\zy-thread-import" },
  @{ Branch = "codex/thread-search-performance"; Path = "..\zy-thread-search" },
  @{ Branch = "codex/thread-dedup-relation"; Path = "..\zy-thread-relation" },
  @{ Branch = "codex/thread-backup-jobs"; Path = "..\zy-thread-backup" },
  @{ Branch = "codex/thread-ai-ready"; Path = "..\zy-thread-ai" },
  @{ Branch = "codex/thread-qa-regression"; Path = "..\zy-thread-qa" }
)

$createdBranches = @()
$skippedBranches = @()
$createdWorktrees = @()
$skippedWorktrees = @()

foreach ($item in $items) {
  $branch = $item.Branch
  $relativePath = $item.Path
  $fullPath = [System.IO.Path]::GetFullPath((Join-Path $ProjectRoot $relativePath))

  git show-ref --verify --quiet "refs/heads/$branch"
  if ($LASTEXITCODE -eq 0) {
    $skippedBranches += $branch
    Write-Host "分支已存在，跳过: $branch"
  } else {
    git branch $branch
    if ($LASTEXITCODE -ne 0) {
      Write-Host "创建分支失败: $branch"
      exit 1
    }
    $createdBranches += $branch
    Write-Host "已创建分支: $branch"
  }

  if (Test-Path $fullPath) {
    $skippedWorktrees += $fullPath
    Write-Host "worktree 目录已存在，跳过: $fullPath"
  } else {
    git worktree add $fullPath $branch
    if ($LASTEXITCODE -ne 0) {
      Write-Host "创建 worktree 失败: $fullPath -> $branch"
      exit 1
    }
    $createdWorktrees += $fullPath
    Write-Host "已创建 worktree: $fullPath"
  }
}

Write-Host ""
Write-Host "多线程 worktree 检查完成。"
Write-Host "新建分支数量: $($createdBranches.Count)"
Write-Host "跳过分支数量: $($skippedBranches.Count)"
Write-Host "新建 worktree 数量: $($createdWorktrees.Count)"
Write-Host "跳过 worktree 数量: $($skippedWorktrees.Count)"
Write-Host ""
Write-Host "下一步建议先打开第一批线程 A / D / G:"
Write-Host "powershell -ExecutionPolicy Bypass -File .\scripts\03_打开第一批线程_ADG.ps1"
