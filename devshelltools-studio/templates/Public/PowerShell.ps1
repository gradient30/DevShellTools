<#!
@DST-Category
Name: powershell
Title: 管理员与 PowerShell
Description: 管理员窗口、执行策略、Profile、命令来源
Aliases: ps
@DST-Category-End
#>

function isadmin {
<#
.SYNOPSIS
检查当前 PowerShell 是否具有管理员权限。
.EXAMPLE
isadmin
#>
    [CmdletBinding()]
    param()
    if (Test-DstAdministrator) {
        Write-DstOk "当前 PowerShell 已具有管理员权限。"
        return $true
    }
    Write-DstWarn "当前 PowerShell 不是管理员会话。"
    return $false
}

function super {
<#
.SYNOPSIS
打开新的管理员 PowerShell；不会修改当前窗口权限。
.EXAMPLE
super
.EXAMPLE
super -CurrentDirectory
#>
    [CmdletBinding()]
    param([switch]$CurrentDirectory)
    if (-not (Test-DstWindows)) { throw "super 仅支持 Windows。" }
    if (Test-DstAdministrator) {
        Write-DstInfo "当前 PowerShell 已经是管理员会话。"
        return
    }
    $exe = (Get-Process -Id $PID).Path
    $argList = @()
    if ($CurrentDirectory) {
        $escaped = (Get-Location).Path.Replace("'","''")
        $argList += @("-NoExit","-Command","Set-Location -LiteralPath '$escaped'")
    }
    Start-Process -FilePath $exe -Verb RunAs -ArgumentList $argList
    Write-DstOk "已请求打开管理员 PowerShell。"
}

function psb {
<#
.SYNOPSIS
仅在当前 PowerShell 进程临时允许执行脚本。
.DESCRIPTION
等价于 Set-ExecutionPolicy -Scope Process Bypass -Force；这不是管理员提权。
.EXAMPLE
psb
#>
    [CmdletBinding()]
    param()
    Set-ExecutionPolicy -Scope Process Bypass -Force
    Write-DstOk "当前 PowerShell 进程的执行策略已设为 Bypass。"
    Write-DstWarn "这不代表获得管理员权限，关闭窗口后自动失效。"
}

function reload {
<#
.SYNOPSIS
重新加载当前 PowerShell Profile。
.EXAMPLE
reload
#>
    [CmdletBinding()]
    param()
    if (Test-Path -LiteralPath $PROFILE) {
        . $PROFILE
        Write-DstOk "已重新加载：$PROFILE"
    } else {
        Write-DstWarn "Profile 不存在：$PROFILE"
    }
}

function profile {
<#
.SYNOPSIS
打开当前 PowerShell Profile 文件。
.EXAMPLE
profile
#>
    [CmdletBinding()]
    param()
    $dir = Split-Path -Parent $PROFILE
    if (-not (Test-Path -LiteralPath $dir)) { New-Item -ItemType Directory -Path $dir -Force | Out-Null }
    if (-not (Test-Path -LiteralPath $PROFILE)) { New-Item -ItemType File -Path $PROFILE -Force | Out-Null }
    if (Test-DstWindows) { Start-Process notepad.exe -ArgumentList @($PROFILE) } else { Invoke-Item $PROFILE }
}

function which {
<#
.SYNOPSIS
查看命令的真实来源和所有匹配项。
.EXAMPLE
which codex
.EXAMPLE
which git
#>
    [CmdletBinding()]
    param([Parameter(Mandatory,Position=0)][string]$Name)
    Get-Command $Name -All | Select-Object CommandType, Name, Source, Definition
}