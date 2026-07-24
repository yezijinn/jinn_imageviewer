# =============================================================================
# Jinn 图片查看器 - 全自动构建脚本
# =============================================================================
# 功能：自动检测环境、构建 release 版本、复制为友好文件名
# 作者：叶子Jinn
# =============================================================================

[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"

# 设置控制台编码为 UTF-8
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
chcp 65001 | Out-Null

# 颜色输出函数
function Write-Success { param($msg) Write-Host "✓ $msg" -ForegroundColor Green }
function Write-Info { param($msg) Write-Host "ℹ $msg" -ForegroundColor Cyan }
function Write-Warning { param($msg) Write-Host "⚠ $msg" -ForegroundColor Yellow }
function Write-Error { param($msg) Write-Host "✗ $msg" -ForegroundColor Red }
function Write-Step { param($msg) Write-Host "`n▶ $msg" -ForegroundColor Magenta }

# 获取脚本所在目录
$ProjectRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $ProjectRoot

Write-Host @"

═══════════════════════════════════════════════════════
   Jinn 图片查看器 - 自动构建脚本
═══════════════════════════════════════════════════════

"@ -ForegroundColor Cyan

# =============================================================================
# 步骤 1: 环境检测
# =============================================================================
Write-Step "步骤 1/5: 检测构建环境"

# 检查 Rust 工具链
Write-Info "检查 Rust 工具链..."
try {
    $rustcVersion = rustc --version 2>&1 | Out-String
    $cargoVersion = cargo --version 2>&1 | Out-String
    Write-Success "Rust 已安装"
    Write-Host "  rustc: $($rustcVersion.Trim())" -ForegroundColor Gray
    Write-Host "  cargo: $($cargoVersion.Trim())" -ForegroundColor Gray
} catch {
    Write-Error "未找到 Rust 工具链"
    Write-Host "请访问 https://rustup.rs/ 安装 Rust" -ForegroundColor Yellow
    exit 1
}

# 检查 MSVC 工具链（可选警告）
Write-Info "检查 MSVC 工具链..."
$clFound = $false
try {
    $null = Get-Command cl.exe -ErrorAction SilentlyContinue
    $clFound = $true
    Write-Success "MSVC 编译器已在 PATH 中"
} catch {
    Write-Warning "cl.exe 未在 PATH 中（cargo 可能会自动处理）"
}

# 检查项目文件
Write-Info "检查项目文件..."
$requiredFiles = @("Cargo.toml", "src\main.rs", "build.rs", "app_icon.ico")
$missingFiles = @()
foreach ($file in $requiredFiles) {
    if (-not (Test-Path $file)) {
        $missingFiles += $file
    }
}

if ($missingFiles.Count -gt 0) {
    Write-Error "缺少必需文件："
    $missingFiles | ForEach-Object { Write-Host "  - $_" -ForegroundColor Red }
    exit 1
}
Write-Success "项目文件完整"

# =============================================================================
# 步骤 2: 清理旧构建
# =============================================================================
Write-Step "步骤 2/5: 清理旧构建产物"

if (Test-Path "target\release\jinn-imageviewer.exe") {
    Write-Info "清理旧的 release 构建..."
    Remove-Item "target\release\jinn-imageviewer.exe" -Force -ErrorAction SilentlyContinue
    Write-Success "清理完成"
} else {
    Write-Info "无旧构建产物，跳过清理"
}

# =============================================================================
# 步骤 3: 运行测试
# =============================================================================
Write-Step "步骤 3/5: 运行单元测试"

Write-Info "执行 cargo test..."
$testOutput = cargo test --quiet 2>&1
$testExitCode = $LASTEXITCODE

if ($testExitCode -eq 0) {
    Write-Success "所有测试通过"
    # 解析测试结果
    $testResult = $testOutput | Select-String "test result:" | Select-Object -Last 1
    if ($testResult) {
        Write-Host "  $($testResult.Line.Trim())" -ForegroundColor Gray
    }
} else {
    Write-Error "测试失败"
    Write-Host $testOutput -ForegroundColor Red
    exit 1
}

# =============================================================================
# 步骤 4: Release 构建
# =============================================================================
Write-Step "步骤 4/5: 构建 Release 版本"

Write-Info "执行 cargo build --release..."
Write-Host "  (首次构建可能需要 5-10 分钟，请耐心等待...)" -ForegroundColor Yellow

$buildStartTime = Get-Date
$buildOutput = cargo build --release 2>&1
$buildExitCode = $LASTEXITCODE
$buildDuration = (Get-Date) - $buildStartTime

if ($buildExitCode -ne 0) {
    Write-Error "构建失败"
    Write-Host $buildOutput -ForegroundColor Red
    exit 1
}

Write-Success "构建成功 (耗时: $([math]::Round($buildDuration.TotalSeconds, 1)) 秒)"

# =============================================================================
# 步骤 5: 复制并重命名 exe
# =============================================================================
Write-Step "步骤 5/5: 生成最终可执行文件"

$sourceExe = "target\release\jinn-imageviewer.exe"
$targetExe = "Jinn图片查看器.exe"

if (-not (Test-Path $sourceExe)) {
    Write-Error "未找到构建产物: $sourceExe"
    exit 1
}

# 获取文件信息
$fileInfo = Get-Item $sourceExe
$fileSizeMB = [math]::Round($fileInfo.Length / 1MB, 2)

Write-Info "复制到项目根目录..."
Copy-Item $sourceExe $targetExe -Force

if (Test-Path $targetExe) {
    Write-Success "生成完成"
    Write-Host ""
    Write-Host "═══════════════════════════════════════════════════════" -ForegroundColor Green
    Write-Host "  ✓ 构建成功！" -ForegroundColor Green
    Write-Host "═══════════════════════════════════════════════════════" -ForegroundColor Green
    Write-Host ""
    Write-Host "  文件路径: " -NoNewline -ForegroundColor Cyan
    Write-Host (Resolve-Path $targetExe) -ForegroundColor White
    Write-Host "  文件大小: " -NoNewline -ForegroundColor Cyan
    Write-Host "$fileSizeMB MB" -ForegroundColor White
    Write-Host "  修改时间: " -NoNewline -ForegroundColor Cyan
    Write-Host $fileInfo.LastWriteTime -ForegroundColor White
    Write-Host ""
    Write-Host "  双击 $targetExe 即可运行！" -ForegroundColor Yellow
    Write-Host ""
} else {
    Write-Error "复制失败"
    exit 1
}

# =============================================================================
# 可选: 运行 clippy 检查
# =============================================================================
Write-Step "额外: 代码质量检查 (clippy)"

Write-Info "执行 cargo clippy..."
$clippyOutput = cargo clippy --quiet 2>&1
$clippyExitCode = $LASTEXITCODE

if ($clippyExitCode -eq 0) {
    Write-Success "Clippy 检查通过，无警告"
} else {
    Write-Warning "Clippy 发现了一些建议（不影响构建）"
    # 显示 clippy 输出（仅 warning 和 error）
    $clippyOutput | Select-String "warning:|error:" | ForEach-Object {
        Write-Host "  $_" -ForegroundColor Yellow
    }
}

Write-Host ""
Write-Host "═══════════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host "  脚本执行完成！" -ForegroundColor Cyan
Write-Host "═══════════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host ""

exit 0
