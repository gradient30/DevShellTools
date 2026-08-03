
Set-StrictMode -Version Latest

function Write-DstTitle {
    param([Parameter(Mandatory)][string]$Text)
    Write-Host ""
    Write-Host ("=" * 72) -ForegroundColor Cyan
    Write-Host "  $Text" -ForegroundColor Cyan
    Write-Host ("=" * 72) -ForegroundColor Cyan
}

function Write-DstInfo { param([string]$Text) Write-Host "[信息] $Text" -ForegroundColor Cyan }
function Write-DstOk   { param([string]$Text) Write-Host "[成功] $Text" -ForegroundColor Green }
function Write-DstWarn { param([string]$Text) Write-Host "[警告] $Text" -ForegroundColor Yellow }
function Write-DstFail { param([string]$Text) Write-Host "[失败] $Text" -ForegroundColor Red }

function Assert-DstCommand {
    param([Parameter(Mandatory)][string]$Name)
    if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) {
        throw "未找到命令：$Name"
    }
}

function Test-DstWindows {
    return ($env:OS -eq "Windows_NT")
}

function Test-DstAdministrator {
    if (-not (Test-DstWindows)) { return $false }
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = [Security.Principal.WindowsPrincipal]::new($identity)
    return $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
}

function Get-DstProxyVariables {
    $names = @("HTTP_PROXY","HTTPS_PROXY","ALL_PROXY","http_proxy","https_proxy","all_proxy")
    foreach ($name in $names) {
        [PSCustomObject]@{
            Name  = $name
            Value = [Environment]::GetEnvironmentVariable($name, "Process")
        }
    }
}

function Set-DstProcessEnvironment {
    param(
        [Parameter(Mandatory)][string[]]$Names,
        [AllowNull()][string]$Value
    )
    foreach ($name in $Names) {
        [Environment]::SetEnvironmentVariable($name, $Value, "Process")
    }
}
