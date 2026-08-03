<#
.SYNOPSIS
安装或升级 DevShellTools。
.DESCRIPTION
把模块复制到当前用户的 Windows PowerShell 5.1 和 PowerShell 7 模块目录，
并向对应 Profile 写入 Import-Module DevShellTools。
.EXAMPLE
Set-ExecutionPolicy -Scope Process Bypass
.\install.ps1
#>

[CmdletBinding()]
param([switch]$SkipProfile)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$moduleName = "DevShellTools"
$source = $PSScriptRoot
$documents = [Environment]::GetFolderPath("MyDocuments")

$targets = @(
    (Join-Path $documents "WindowsPowerShell\Modules\$moduleName")
    (Join-Path $documents "PowerShell\Modules\$moduleName")
)

foreach ($target in $targets) {
    if (Test-Path -LiteralPath $target) {
        $backup = "$target.backup.$(Get-Date -Format 'yyyyMMddHHmmss')"
        Copy-Item -LiteralPath $target -Destination $backup -Recurse -Force
        Write-Host "[备份] $backup" -ForegroundColor Yellow
        Remove-Item -LiteralPath $target -Recurse -Force
    }

    New-Item -ItemType Directory -Path $target -Force | Out-Null
    Copy-Item -Path (Join-Path $source "*") -Destination $target -Recurse -Force
    Remove-Item -LiteralPath (Join-Path $target "install.ps1") -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath (Join-Path $target "uninstall.ps1") -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath (Join-Path $target "README.md") -Force -ErrorAction SilentlyContinue
    Write-Host "[成功] 已安装：$target" -ForegroundColor Green
}

if (-not $SkipProfile) {
    $profiles = @(
        (Join-Path $documents "WindowsPowerShell\Microsoft.PowerShell_profile.ps1")
        (Join-Path $documents "PowerShell\Microsoft.PowerShell_profile.ps1")
    )

    foreach ($profilePath in $profiles) {
        $dir = Split-Path -Parent $profilePath
        New-Item -ItemType Directory -Path $dir -Force | Out-Null
        if (-not (Test-Path -LiteralPath $profilePath)) {
            New-Item -ItemType File -Path $profilePath -Force | Out-Null
        }

        $content = Get-Content -LiteralPath $profilePath -Raw -ErrorAction SilentlyContinue
        if ($null -eq $content) { $content = "" }

        if ($content -notmatch "(?m)^\s*Import-Module\s+DevShellTools\s*(\s|$)") {
            Add-Content -LiteralPath $profilePath -Value ""
            Add-Content -LiteralPath $profilePath -Value "# DevShellTools"
            Add-Content -LiteralPath $profilePath -Value "Import-Module DevShellTools -Force -ErrorAction SilentlyContinue"
            Write-Host "[成功] 已更新 Profile：$profilePath" -ForegroundColor Green
        }
    }
}

Remove-Module DevShellTools -Force -ErrorAction SilentlyContinue
Import-Module DevShellTools -Force

Write-Host ""
Write-Host "[成功] DevShellTools 1.0.5 安装完成。" -ForegroundColor Green
Write-Host "首次使用：dsh"
Write-Host "直接分类：dsh files | powershell | proxy | git | network"
Write-Host ""
Write-Host "[兼容提醒] 如果已安装旧 ProxyCtl，lpr 可能存在同名命令。" -ForegroundColor Yellow
Write-Host "当前模块已导出统一版 lpr；可使用 which lpr 查看实际来源。" -ForegroundColor Yellow