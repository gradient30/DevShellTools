
[CmdletBinding()]
param()

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$moduleName = "DevShellTools"
$documents = [Environment]::GetFolderPath("MyDocuments")

Remove-Module $moduleName -Force -ErrorAction SilentlyContinue

$targets = @(
    (Join-Path $documents "WindowsPowerShell\Modules\$moduleName")
    (Join-Path $documents "PowerShell\Modules\$moduleName")
)
foreach ($target in $targets) {
    if (Test-Path -LiteralPath $target) {
        Remove-Item -LiteralPath $target -Recurse -Force
        Write-Host "[成功] 已删除：$target" -ForegroundColor Green
    }
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
