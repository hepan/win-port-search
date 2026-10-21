# 免安装 EXE 技术可行性调查

对应 [研究票据 #7](https://github.com/hepan/win-port-search/issues/7)。截至 2026-10-21，本页只汇总厂商公开资料，供后续产品规格权衡；**没有产出 EXE、没有在任何 Windows/Server 系统实机启动或验收**，不把理论 API/框架适配等同于产品兼容性。目标矩阵以 [#4 决议](https://github.com/hepan/win-port-search/issues/4) 和 README 为准：Win7 **SP1**、8、8.1、10 的 x86/x64，Win11 x64；Server 2008 R2、2012、2012 R2、2016、2019、2022、2025 的 x64，且仅桌面 GUI 安装。特别注意：微软列出的 .NET Framework 4.8 下限是 **Server 2008 R2 SP1**；票据中的「2008 R2」未明定 SP1，选择依赖 4.8 的路线不能在未更新规格/验收之前直接覆盖未打 SP1 的 Server 2008 R2。[Framework 系统要求]

## 口径与架构

「免安装」可指 (A) 用户直接运行单个 EXE，无预装额外运行时；(B) 解压目录里的 EXE 和旁置 DLL/插件，无安装器；(C) EXE 不安装自身，但需要客户先安装 .NET Framework、VC++ Redistributable 或 WebView2。这三者不是同一个承诺：只有 A 符合严格的「一文件、零额外安装」，B 符合「免安装文件夹」，C 仅是「应用免安装」。x86 程序可通过 WOW64 在 x64 Windows 上运行，但不能加载 x64 DLL；目标是 x86/x64 可用时，应分别审查每种位数和依赖，不能以 x86 在 x64 启动替代 x64 产品验收。[WOW64]

| 路线 | 最低版本与位数：官方依据 | 严格单 EXE / 免安装文件夹 / 前置依赖 | 对此矩阵的结论（未实测） |
| --- | --- | --- | --- |
| **原生 Win32 GUI（C/C++、系统控件）** | Windows SDK 文档称可针对 Win7 SP1 及以后写传统桌面应用；Windows 7/Server 2008 R2 对应 manifest 的 `supportedOS`。分别构建 PE x86、x64；实际使用的 API、导入表和编译器库须逐项审查。[Windows SDK][应用清单][WOW64] | 可用 `/MT` 静态链接 MSVC CRT，资源/清单嵌入 EXE，并只依赖目标系统原生 DLL，达到 A；`/MD` 则需要匹配架构与编译工具版本的 VC++ 运行库，应用本地 DLL 可实现 B，但微软出于安全维护原因**不推荐**应用本地运行库部署。`/MT` 不能消除 Windows API、可选第三方库及其系统补丁依赖。[MSVC 链接][VC 再分发][应用清单] | **低依赖候选**；官方 API 下限不等于可执行程序的逐版保证。特别不能把「最新 VC++ redistributable」作为全矩阵解法：微软现列最新 v14 包只支持 Win10/11 和 Server 2016–2025；需固定适用老系统的工具链/库并审查受支持性，或采用合适的静态链接路径。[VC 最新包] |
| **Qt 5.15 Widgets + 原生代码** | Qt 5.15 归档表明确列 Win7、8.1、10、11 的 x86/x86_64；Win7 及 8.1 的表列 MSVC 2019/2017、MinGW 8.1。**未列 Win8 或任何 Server**；官方说明未列配置未积极测试，不能当作已获厂商覆盖。Qt 6.12 官方矩阵从 Win10 1809、x86_64 起，不覆盖旧版本/x86 需求。[Qt 5 平台][Qt 6 Windows] | 动态 Qt 可用 `windeployqt` 制作 B，需匹配 Qt DLL、`platforms/qwindows.dll`、编译器运行库及实际使用模块的其他 DLL；不是单 EXE。静态编译 Qt 和所需静态插件可探索 A，但需检查所有依赖和授权；Qt 官方列 LGPLv3 静态链接/用户重链接义务，商业许可或其他符合许可方案应先厘清。[Qt 部署][Qt 静态插件][Qt 授权][VC 再分发] | **有条件候选**，但 Win8/Server 在 Qt 5.15 平台表中的空缺和单文件许可/编译成本需特别标红；不宜承诺全矩阵受 Qt 厂商验证。若只做本机表格 UI，Qt Widgets 官方注明可不要求硬件加速，别混用 Qt WebEngine 的平台限制。[Qt 5 平台] |
| **.NET Framework WinForms/WPF** | 微软表列 4.8 可单独安装于 Win7 SP1、Win8.1、Server 2008 R2 SP1/2012/2012 R2/2016/2019；较新系统预装 4.8/4.8.1，4.x 为就地升级。**Win8（非 8.1）仅在旧版历史表中列可安装 4.6.1，不在 4.8 清单**；故要包括 Win8，不能以 4.8 作统一最低版本。Windows x64 支持 WOW64 和原生 x64。[Framework 系统要求][Framework 版本] | 标准框架应用可发单个业务 EXE，但**依赖机器已安装相应 .NET Framework**（C），不是自带 CLR 的零前置 A。Win7/Server 2008 R2 未预装 4.8；安装 Framework 需管理员权限；Win8 的 4.6.1/4.5 与新系统版本差异要核对选定 API 和安全维护政策。[Framework 系统要求][Framework 版本] | **若允许安装或已有 Framework 才是候选**；4.8 直接漏掉 Win8 及 Server 2008 R2 未打 SP1，4.6.1 满足 Win8 的历史安装清单但不是 2026 年可统一声称受支持的全矩阵运行时。不能把「业务 EXE 是一文件」写成「运行时零依赖」。[Framework 系统要求][Framework 版本] |
| **现代 .NET WinForms/WPF、single-file self-contained** | .NET 官方当前矩阵 Win7/8.1 均不支持；历史最后为 .NET 6，支持在 2024-11-12 结束；Server 2008 R2 不在当前矩阵；Win8 亦不在其中。`.NET 6` 单文件文档虽然说明 Win7 需 runtime ≥6.0.3，这只是历史运行条件，不恢复官方支持。[现代 .NET Windows][.NET 单文件] | 发布可指定 win-x86/win-x64 的独立运行时与单文件，但单文件是特定 OS/架构；原生库默认可能仍在外部，合并原生库要自行解压至 `%TEMP%/.net` 并审查提权启动安全。[.NET 单文件] | **不作为覆盖全矩阵的受支持路线**，即使旧版本某些旧系统上能运行。[现代 .NET Windows][.NET 单文件] |
| **WebView2 / Tauri、Electron 等 Web GUI** | Tauri Windows UI 依赖 WebView2；WebView2 Runtime/Edge 在 Win7/8/8.1、Server 2008 R2/2012/2012 R2 的支持止于 Edge 109；Electron **22** 是最后支持 Win7/8/8.1 的主版本，Electron 23 仅 Win10+，Electron 22 后续安全维护也于 2023-10-10 结束。WinUI 3/Windows App SDK 官方最低 Win10 1809，亦无法满足旧系统。[Tauri 依赖][Edge 支持][Electron 旧系统][Windows App SDK] | WebView2 Evergreen 需客户机运行时且在旧系统已不再获新版本支持；Fixed Version 可旁置整个运行时，不自动更新，官方指出超过 250 MB，且下载页仅提供最近两个大版本，**不能想当然取得/分发老版 109**。Electron 含 Chromium 引擎，单独 Electron EXE 不等于完备分发，旧版有安全维护缺口。[WebView2 分发][Edge 支持][Electron 旧系统] | **不推荐作为全矩阵统一受支持路线**；WebView2 特别不能因为 Tauri 写“Win7+”开发前提就忽略微软旧系统 109 截止、Server 与 x86 证据以及客户运行时安装。[Tauri 依赖][Edge 支持][WebView2 分发] |

## 分发、提权、签名的共同约束

1. 管理员 GUI 可在 EXE 内嵌清单，以 `requestedExecutionLevel=requireAdministrator` 让 UAC **在进程启动前**请求授权；取消时程序根本不会启动，**无法由尚未运行的 GUI 自己显示「取消原因」后退出**。#4 的「说明原因并退出」必须在后续规格中澄清是 OS 的 UAC 反馈、外部启动器说明，还是其他符合单 EXE 约束的交互，不能误写为提权后的窗口提示；研究不在此做设计决定。微软清单文档列不同 Windows/Server 的 `supportedOS` GUID；**写入 GUID 不是兼容性测试证书**。[管理员权限][应用清单]
2. 对 Win7 SP1 / Server 2008 R2 SP1，微软规定 SHA-2 更新 KB4474419、服务堆栈 KB4490628 并重启是安装 2019-08 以后系统更新的前提；验收发行签名/证书链时须记录基线更新和离线根证书环境，但不能由该 Windows Update 文档推导出「所有未经更新机器一定无法启动任意签名 EXE」。Windows SDK 的 SignTool 可签名、校验、时间戳，推荐 SHA256（`/fd`，RFC3161 时间戳用 `/tr` 与 `/td`）；签名提高来源可核验性，**不会增加 OS API/运行时兼容性**。[SHA-2 更新][SignTool]
3. 打包约定应明确「单个 PE」还是「ZIP 解压后运行 EXE」及**不得弹出运行时安装器**的要求；逐版逐架构核对 PE/依赖 DLL、插件架构及系统补丁、离线启动、UAC 拒绝/接受、签名验证与数据功能。GUI Server 仅 Desktop Experience；从客户端支持表或 API 下限**不能推断**所有 Server 版本已运行。当前 README 的每行「已测试：否」维持不变；本调查既未测试也不更新 README。[WOW64][Qt 5 平台][VC 最新包]

## 供后续规格选择的事实边界

若「免安装」定义为**严格 A（单文件且在未预装额外运行时的目标机器启动）**，原生 Win32、采用受老系统约束的构建/CRT 策略，是证据最直接的候选；Qt 静态版需额外证明插件、库、许可、Win8/Server。若允许 B，则 Qt 动态目录可行但不是「单 EXE」；若允许 C，则 .NET Framework 方案有 Win8/Server 2008 R2 SP1 的版本岔路，且运行时安装与「零前置」矛盾。此处**不代替规格决定**技术栈或改变支持政策。任何路线须产出 x86、x64（目标适用时）候选构建后逐项人工验收；本页只论官方文档的理论可能性/明显障碍，不承诺启动成功。[Windows SDK][Qt 5 平台][Framework 系统要求][WOW64]

[Windows SDK]: https://learn.microsoft.com/en-us/cpp/windows/how-to-use-the-windows-10-sdk-in-a-windows-desktop-application?view=msvc-170
[应用清单]: https://learn.microsoft.com/en-us/windows/win32/sbscs/application-manifests
[WOW64]: https://learn.microsoft.com/en-us/windows/win32/winprog64/running-32-bit-applications
[MSVC 链接]: https://learn.microsoft.com/en-us/cpp/build/reference/md-mt-ld-use-run-time-library?view=msvc-170
[VC 再分发]: https://learn.microsoft.com/en-us/cpp/windows/redistributing-visual-cpp-files?view=msvc-170
[VC 最新包]: https://learn.microsoft.com/en-us/cpp/windows/latest-supported-vc-redist?view=msvc-170
[Qt 5 平台]: https://doc.qt.io/archives/qt-5.15/supported-platforms.html
[Qt 6 Windows]: https://doc.qt.io/qt-6/windows.html
[Qt 部署]: https://doc.qt.io/archives/qt-5.15/windows-deployment.html
[Qt 静态插件]: https://doc.qt.io/archives/qt-5.15/plugins-howto.html#static-plugins
[Qt 授权]: https://www.qt.io/development/open-source-lgpl-obligations
[Framework 系统要求]: https://learn.microsoft.com/en-us/dotnet/framework/get-started/system-requirements
[Framework 版本]: https://learn.microsoft.com/en-us/dotnet/framework/install/versions-and-dependencies
[现代 .NET Windows]: https://learn.microsoft.com/en-us/dotnet/core/install/windows
[.NET 单文件]: https://learn.microsoft.com/en-us/dotnet/core/deploying/single-file/overview
[Tauri 依赖]: https://v2.tauri.app/start/prerequisites/#windows
[Edge 支持]: https://learn.microsoft.com/en-us/deployedge/microsoft-edge-supported-operating-systems
[WebView2 分发]: https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/distribution
[Electron 旧系统]: https://www.electronjs.org/blog/windows-7-to-8-1-deprecation-notice
[Windows App SDK]: https://learn.microsoft.com/en-us/windows/apps/windows-app-sdk/
[SHA-2 更新]: https://support.microsoft.com/en-us/servicing/os/windows/2020/09/2019-sha-2-code-signing-support-requirement-for-windows-and-wsus
[SignTool]: https://learn.microsoft.com/en-us/windows/win32/seccrypto/signtool
[管理员权限]: https://learn.microsoft.com/en-us/windows/win32/secbp/running-with-administrator-privileges
