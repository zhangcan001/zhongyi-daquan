cd "$PSScriptRoot\.."

if (!(Test-Path ".git")) {
  git init
}

git add .
git commit -m "docs: initialize zhongyi codex ready package"

Write-Host "完成：Git 已初始化并提交文档。"
Write-Host "下一步：code ."
