<#
.SYNOPSIS
卸载 DevShellTools（从 PowerShell 注销）。
.DESCRIPTION
清理 Profile 中的 Import-Module；删除非 Studio 工作区的模块副本。
若目录含 .studio（Studio 工作区），则保留文件，但禁用模块清单，
避免新开窗口因命令发现自动加载仍可用。
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

function Disable-DstShellModule([string]$ModuleDir) {
    # 同时禁用 .psd1 与 .psm1：仅改清单时，PS 仍可能按同名 .psm1 自动加载
    foreach ($pair in @(
        @{ Active = "DevShellTools.psd1"; Disabled = "DevShellTools.psd1.dst-disabled" },
        @{ Active = "DevShellTools.psm1"; Disabled = "DevShellTools.psm1.dst-disabled" }
    )) {
        $active = Join-Path $ModuleDir $pair.Active
        $disabled = Join-Path $ModuleDir $pair.Disabled
        if (Test-Path -LiteralPath $active) {
            if (Test-Path -LiteralPath $disabled) {
                Remove-Item -LiteralPath $disabled -Force
            }
            Rename-Item -LiteralPath $active -NewName $pair.Disabled
            Write-Host "[成功] 已禁用：$disabled" -ForegroundColor Green
        }
    }
}

function Clear-DstProfileImports([string]$ProfilePath) {
    if (-not (Test-Path -LiteralPath $ProfilePath)) { return }
    $lines = @(Get-Content -LiteralPath $ProfilePath)
    $filtered = @($lines | Where-Object {
        $_ -notmatch "^\s*#\s*DevShellTools\s*$" -and
        $_ -notmatch "^\s*Import-Module\s+DevShellTools\b"
    })
    if ($filtered.Count -ne $lines.Count) {
        Set-Content -LiteralPath $ProfilePath -Value $filtered -Encoding UTF8
        Write-Host "[成功] 已清理 Profile：$ProfilePath" -ForegroundColor Green
    }
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
        Disable-DstShellModule -ModuleDir $target
        Write-Host "[跳过] Studio 工作区文件保留：$target" -ForegroundColor Yellow
        continue
    }
    Remove-Item -LiteralPath $target -Recurse -Force
    Write-Host "[成功] 已删除：$target" -ForegroundColor Green
}

# 清理 MyDocuments 与 USERPROFILE\Documents 两套 Profile（文件夹重定向时可能不一致）
$profileCandidates = [System.Collections.Generic.List[string]]::new()
foreach ($docsRoot in @($documents, (Join-Path $env:USERPROFILE "Documents"))) {
    if ([string]::IsNullOrWhiteSpace($docsRoot)) { continue }
    $profileCandidates.Add((Join-Path $docsRoot "WindowsPowerShell\Microsoft.PowerShell_profile.ps1"))
    $profileCandidates.Add((Join-Path $docsRoot "WindowsPowerShell\profile.ps1"))
    $profileCandidates.Add((Join-Path $docsRoot "PowerShell\Microsoft.PowerShell_profile.ps1"))
    $profileCandidates.Add((Join-Path $docsRoot "PowerShell\profile.ps1"))
}
$profileCandidates | Select-Object -Unique | ForEach-Object { Clear-DstProfileImports -ProfilePath $_ }

# 清理 Modules 下历史 DevShellTools.backup.*（旧版把备份放在 Modules 内会导致递归自动加载）
$modulesRoot = Join-Path $documents "WindowsPowerShell\Modules"
if (Test-Path -LiteralPath $modulesRoot) {
    $quarantine = Join-Path $documents "DevShellTools-backups\orphaned-module-backups"
    Get-ChildItem -LiteralPath $modulesRoot -Directory -ErrorAction SilentlyContinue |
        Where-Object { $_.Name -like "DevShellTools.backup*" } |
        ForEach-Object {
            New-Item -ItemType Directory -Path $quarantine -Force | Out-Null
            $dest = Join-Path $quarantine $_.Name
            if (Test-Path -LiteralPath $dest) {
                $dest = "$dest.$([Guid]::NewGuid().ToString('N').Substring(0, 8))"
            }
            Move-Item -LiteralPath $_.FullName -Destination $dest -Force
            Write-Host "[成功] 已移出 Modules 污染备份：$($_.Name)" -ForegroundColor Yellow
        }
}

Write-Host "[完成] 已从 PowerShell 注销。新开窗口中 dsh 等命令应不可用；Studio 工作区仍可编辑。" -ForegroundColor Cyan
