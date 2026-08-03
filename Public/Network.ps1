
function ports {
<#
.SYNOPSIS
查看本机正在监听的 TCP 端口及进程。
.EXAMPLE
ports
#>
    [CmdletBinding()] param()
    if (-not (Test-DstWindows)) { throw "ports 当前实现仅支持 Windows。" }
    Get-NetTCPConnection -State Listen |
        Sort-Object LocalPort |
        ForEach-Object {
            $p = Get-Process -Id $_.OwningProcess -ErrorAction SilentlyContinue
            [PSCustomObject]@{
                Address = $_.LocalAddress
                Port = $_.LocalPort
                PID = $_.OwningProcess
                Process = $p.ProcessName
            }
        } | Format-Table -AutoSize
}

function port {
<#
.SYNOPSIS
检查指定主机端口，默认检查 127.0.0.1。
.EXAMPLE
port 7897
.EXAMPLE
port 443 github.com
#>
    [CmdletBinding()]
    param(
        [Parameter(Mandatory,Position=0)][ValidateRange(1,65535)][int]$Number,
        [Parameter(Position=1)][string]$HostName = "127.0.0.1"
    )
    Test-NetConnection $HostName -Port $Number
}

function myip {
<#
.SYNOPSIS
通过公共服务查询当前公网 IP。
.EXAMPLE
myip
#>
    [CmdletBinding()] param()
    Assert-DstCommand "curl.exe"
    $result = & curl.exe --silent --show-error --connect-timeout 8 --max-time 15 "https://api.ipify.org"
    if ($LASTEXITCODE -ne 0 -or -not $result) { throw "公网 IP 查询失败。" }
    $result
}

function dns {
<#
.SYNOPSIS
执行 DNS 查询。
.EXAMPLE
dns github.com
.EXAMPLE
dns github.com A
#>
    [CmdletBinding()]
    param(
        [Parameter(Mandatory,Position=0)][string]$Name,
        [Parameter(Position=1)][ValidateSet("A","AAAA","CNAME","MX","NS","TXT","SOA","PTR")][string]$Type = "A"
    )
    Resolve-DnsName -Name $Name -Type $Type
}

function pingx {
<#
.SYNOPSIS
按指定次数执行 Ping，默认 4 次。
.EXAMPLE
pingx github.com
.EXAMPLE
pingx github.com 10
#>
    [CmdletBinding()]
    param(
        [Parameter(Mandatory,Position=0)][string]$Target,
        [Parameter(Position=1)][ValidateRange(1,100)][int]$Count = 4
    )
    Test-Connection -ComputerName $Target -Count $Count
}

function curlh {
<#
.SYNOPSIS
查看 URL 的 HTTP 响应头。
.EXAMPLE
curlh https://github.com
#>
    [CmdletBinding()]
    param([Parameter(Mandatory,Position=0)][string]$Url)
    Assert-DstCommand "curl.exe"
    & curl.exe -I --location --connect-timeout 10 --max-time 30 $Url
}

function nettest {
<#
.SYNOPSIS
执行常用基础网络诊断。
.EXAMPLE
nettest
.EXAMPLE
nettest github.com 443
#>
    [CmdletBinding()]
    param(
        [Parameter(Position=0)][string]$HostName = "github.com",
        [Parameter(Position=1)][ValidateRange(1,65535)][int]$Port = 443
    )
    Write-DstTitle "DNS 解析"
    try { Resolve-DnsName $HostName -Type A -ErrorAction Stop | Select-Object -First 3 }
    catch { Write-DstFail $_.Exception.Message }

    Write-DstTitle "TCP 端口"
    Test-NetConnection $HostName -Port $Port

    Write-DstTitle "当前代理变量"
    Get-DstProxyVariables | Format-Table -AutoSize
}

function killport {
<#
.SYNOPSIS
查找占用指定端口的进程；仅在使用 -Stop 时终止。
.EXAMPLE
killport 3000
.EXAMPLE
killport 3000 -Stop
.EXAMPLE
killport 3000 -Stop -Force
#>
    [CmdletBinding(SupportsShouldProcess,ConfirmImpact="High")]
    param(
        [Parameter(Mandatory,Position=0)][ValidateRange(1,65535)][int]$Number,
        [switch]$Stop,
        [switch]$Force
    )
    if (-not (Test-DstWindows)) { throw "killport 当前实现仅支持 Windows。" }

    $connections = Get-NetTCPConnection -LocalPort $Number -ErrorAction SilentlyContinue
    if (-not $connections) {
        Write-DstInfo "未发现占用端口 $Number 的 TCP 连接。"
        return
    }

    $processes = $connections |
        Select-Object -ExpandProperty OwningProcess -Unique |
        ForEach-Object { Get-Process -Id $_ -ErrorAction SilentlyContinue }

    $processes | Select-Object Id, ProcessName, Path | Format-Table -AutoSize

    if (-not $Stop) {
        Write-DstWarn "未终止进程。需要终止时使用：killport $Number -Stop"
        return
    }

    foreach ($p in $processes) {
        $target = "$($p.ProcessName) (PID $($p.Id))"
        if ($Force -or $PSCmdlet.ShouldProcess($target, "停止进程")) {
            Stop-Process -Id $p.Id -Force:$Force
            Write-DstOk "已停止：$target"
        }
    }
}
