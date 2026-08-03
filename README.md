# DevShellTools 1.0.4

可扩展的 Windows PowerShell 开发与运维快捷命令模块。

## 安装

```powershell
Set-ExecutionPolicy -Scope Process Bypass
.\install.ps1
```

重新打开 PowerShell，或在当前窗口执行：

```powershell
Import-Module DevShellTools -Force
```

## 帮助

```powershell
dsh help
dsh help files
dsh help powershell
dsh help proxy
dsh help git
dsh help network
Get-Help lt -Examples
```

## 设计结构

```text
DevShellTools/
├─ DevShellTools.psd1
├─ DevShellTools.psm1
├─ Private/
│  └─ Common.ps1
└─ Public/
   ├─ Files.ps1
   ├─ PowerShell.ps1
   ├─ Proxy.ps1
   ├─ Git.ps1
   ├─ Network.ps1
   └─ Help.ps1
```

后续扩展：在 `Public` 新增分类脚本，在 `DevShellTools.psm1` 和清单中加入导出函数。

## 安全边界

- `lpr` 只修改当前 PowerShell 进程代理；`clean-user` 才删除用户级变量。
- `gclean` 只执行 dry-run，不删除文件。
- `killport` 默认只显示进程；必须显式加 `-Stop` 才终止。
- `super` 打开新管理员窗口，不会悄悄提升当前窗口。
- `psb` 仅修改当前进程执行策略，不等于管理员权限。
- Git 强制推送、硬重置、真实清理未提供快捷命令。


## 1.0.2 修复

- 修复 install.ps1 / uninstall.ps1 中 Join-Path 数组写法导致的 ChildPath 类型转换错误。


## 1.0.3 首次使用体验优化

安装后直接执行：

```powershell
dsh
```

会显示可用分类并进入交互式菜单。也支持：

```powershell
dsh list
dsh files
dsh powershell
dsh proxy
dsh git
dsh network
```

首次使用者无需记住命令，可先查看分类，再查看该分类下全部快捷命令、中文说明和示例。


## 1.0.4 显示优化

分类命令详情改为固定宽度左对齐显示：

- 命令列左对齐，并增加右侧间距
- 中文说明列左对齐，并增加右侧间距
- 示例列左对齐
- 不再依赖 `Format-Table -AutoSize`，避免中文宽度计算导致字段挤在一起
