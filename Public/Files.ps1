
<#
.SYNOPSIS
目录与文件快捷命令。
#>

function lt {
<#
.SYNOPSIS
按最后修改时间倒序显示项目，默认最近 5 个。
.EXAMPLE
lt
.EXAMPLE
lt -10
.EXAMPLE
lt -20 "D:\workspace_test"
#>
    [CmdletBinding()]
    param(
        [Parameter(Position=0)]
        [ValidateRange(-100000,100000)]
        [int]$Count = 5,

        [Parameter(Position=1)]
        [string]$Path = "."
    )
    if ($Count -eq 0) { throw "数量不能为 0。" }
    if (-not (Test-Path -LiteralPath $Path)) { throw "路径不存在：$Path" }
    Get-ChildItem -LiteralPath $Path |
        Sort-Object LastWriteTime -Descending |
        Select-Object -First ([Math]::Abs($Count))
}

function ltf {
<#
.SYNOPSIS
只显示最近修改的文件，默认 5 个。
.EXAMPLE
ltf -10
#>
    [CmdletBinding()]
    param(
        [Parameter(Position=0)][ValidateRange(-100000,100000)][int]$Count = 5,
        [Parameter(Position=1)][string]$Path = "."
    )
    if ($Count -eq 0) { throw "数量不能为 0。" }
    if (-not (Test-Path -LiteralPath $Path)) { throw "路径不存在：$Path" }
    Get-ChildItem -LiteralPath $Path -File |
        Sort-Object LastWriteTime -Descending |
        Select-Object -First ([Math]::Abs($Count))
}

function ltd {
<#
.SYNOPSIS
只显示最近修改的目录，默认 5 个。
.EXAMPLE
ltd -8
#>
    [CmdletBinding()]
    param(
        [Parameter(Position=0)][ValidateRange(-100000,100000)][int]$Count = 5,
        [Parameter(Position=1)][string]$Path = "."
    )
    if ($Count -eq 0) { throw "数量不能为 0。" }
    if (-not (Test-Path -LiteralPath $Path)) { throw "路径不存在：$Path" }
    Get-ChildItem -LiteralPath $Path -Directory |
        Sort-Object LastWriteTime -Descending |
        Select-Object -First ([Math]::Abs($Count))
}

function ll {
<#
.SYNOPSIS
详细列出当前目录内容。
.EXAMPLE
ll
.EXAMPLE
ll "D:\workspace_test"
#>
    [CmdletBinding()]
    param([Parameter(Position=0)][string]$Path = ".")
    Get-ChildItem -LiteralPath $Path | Format-Table Mode, LastWriteTime, Length, Name -AutoSize
}

function la {
<#
.SYNOPSIS
列出目录内容并包含隐藏项目。
.EXAMPLE
la
#>
    [CmdletBinding()]
    param([Parameter(Position=0)][string]$Path = ".")
    Get-ChildItem -LiteralPath $Path -Force
}

function mkcd {
<#
.SYNOPSIS
创建目录并立即进入。
.EXAMPLE
mkcd demo
#>
    [CmdletBinding()]
    param([Parameter(Mandatory,Position=0)][string]$Path)
    New-Item -ItemType Directory -Path $Path -Force | Out-Null
    Set-Location -LiteralPath $Path
}

function up {
<#
.SYNOPSIS
返回上一级目录。
.EXAMPLE
up
#>
    [CmdletBinding()]
    param()
    Set-Location ..
}

function up2 {
<#
.SYNOPSIS
返回上两级目录。
.EXAMPLE
up2
#>
    [CmdletBinding()]
    param()
    Set-Location ../..
}

function open {
<#
.SYNOPSIS
使用资源管理器打开目录或文件，默认当前目录。
.EXAMPLE
open
.EXAMPLE
open ".\README.md"
#>
    [CmdletBinding()]
    param([Parameter(Position=0)][string]$Path = ".")
    Invoke-Item -LiteralPath $Path
}

function here {
<#
.SYNOPSIS
显示当前目录完整路径。
.EXAMPLE
here
#>
    [CmdletBinding()]
    param()
    (Get-Location).Path
}

function sz {
<#
.SYNOPSIS
计算目录总大小或显示文件大小。
.EXAMPLE
sz
.EXAMPLE
sz "D:\workspace_test"
#>
    [CmdletBinding()]
    param([Parameter(Position=0)][string]$Path = ".")
    if (-not (Test-Path -LiteralPath $Path)) { throw "路径不存在：$Path" }
    $item = Get-Item -LiteralPath $Path
    if (-not $item.PSIsContainer) {
        $bytes = $item.Length
    } else {
        $bytes = (Get-ChildItem -LiteralPath $Path -File -Recurse -Force -ErrorAction SilentlyContinue |
            Measure-Object -Property Length -Sum).Sum
        if ($null -eq $bytes) { $bytes = 0 }
    }
    [PSCustomObject]@{
        Path = $item.FullName
        Bytes = [int64]$bytes
        KB = [math]::Round($bytes / 1KB, 2)
        MB = [math]::Round($bytes / 1MB, 2)
        GB = [math]::Round($bytes / 1GB, 3)
    }
}
