<#
.SYNOPSIS
卸载 DevShellTools。
.DESCRIPTION
清理 Profile 中的 Import-Module，并删除非 Studio 工作区的模块目录。
若目录含 .studio（Studio 工作区 / PS5.1 同源路径），则保留，仅做软卸载。
#>

[CmdletBinding()]
param()

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Get-DstDocuments {
    if (-not [string]::IsNullOrWhiteSpace($env:DST_MY_DOCUMENTS)) {
        return $env:DST_MY_DOCUMENTS.TrimEnd('\', '/')
    }
    return [Environment]::GetFolderPath("MyDocuments")
}

$moduleName = "DevShellTools"
$documents = Get-DstDocuments

Remove-Module $moduleName -Force -ErrorAction SilentlyContinue

$targets = @(
    (Join-Path $documents "WindowsPowerShell\Modules\$moduleName")
    (Join-Path $documents "PowerShell\Modules\$moduleName")
)
foreach ($target in $targets) {
    if (-not (Test-Path -LiteralPath $target)) { continue }
    $studioMarker = Join-Path $target ".studio"
    if (Test-Path -LiteralPath $studioMarker) {
        Write-Host "[跳过] Studio 工作区保留：$target" -ForegroundColor Yellow
        continue
    }
    Remove-Item -LiteralPath $target -Recurse -Force
    Write-Host "[成功] 已删除：$target" -ForegroundColor Green
}

$profiles = @(
    (Join-Path $documents "WindowsPowerShell\Microsoft.PowerShell_profile.ps1")
    (Join-Path $documents "PowerShell\Microsoft.PowerShell_profile.ps1")
)
foreach ($profilePath in $profiles) {
    if (Test-Path -LiteralPath $profilePath) {
        $lines = Get-Content -LiteralPath $profilePath
        $filtered = $lines | Where-Object {
            $_ -notmatch "^\s*#\s*DevShellTools\s*$" -and
            $_ -notmatch "^\s*Import-Module\s+DevShellTools\b"
        }
        Set-Content -LiteralPath $profilePath -Value $filtered -Encoding UTF8
        Write-Host "[成功] 已清理 Profile：$profilePath" -ForegroundColor Green
    }
}

Write-Host "[完成] 请重新打开 PowerShell。" -ForegroundColor Cyan
