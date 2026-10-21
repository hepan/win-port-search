# Rust 免安装 GUI EXE 与目标 Windows 范围

对应 [#9](https://github.com/hepan/win-port-search/issues/9)，截至 2026-10-21。只判断技术可行性；**没有编译 EXE、检查导入或在任何客户端／服务器实机运行**。矩阵沿用 [#4](https://github.com/hepan/win-port-search/issues/4)：Win7 SP1、8、8.1、10 的 x86/x64，Win11 的 x64；Server 2008 R2、2012、2012 R2、2016、2019、2022、2025 的 x64，限完整桌面 GUI 安装。这里的「免安装单 EXE」允许导入目标系统自带的 Windows DLL，但不预设另装语言运行时、旁置第三方 DLL 或安装器。

## 结论

**有条件可行，不是开箱即用的全矩阵官方保证。** Rust 可生成 PE 原生 EXE；Rust 官方另设 `i686-win7-windows-msvc`、`x86_64-win7-windows-msvc`，明确以 Windows 7 为基线，包含 `std`，可据此探索 x86/x64 老系统构建；Win32 原生窗口／列表控件及微软 `windows` Rust 绑定提供不强制 WebView/.NET 的 GUI 路径。对 Win8/8.1 和 Server 2008 R2/2012/2012 R2 的覆盖是由 Win7 基线与所用 Win32 API 下限作出的**待实测推断**，不是 Rust 对每一个服务器版本的明确承诺。若要求仅用普通 rustup 预编译目标、或要求 Rust 官方 Tier 1 保证所有旧系统，则答案是**否**。[Win7 MSVC 目标][Windows 基线变更][Win32 窗口][Win32 列表][Windows Rust 绑定][目标等级]

| 关注点 | 官方事实与限制 |
| --- | --- |
| 目标与构建主机 | 自 Rust 1.78 起常用 `i686-pc-windows-msvc` / `x86_64-pc-windows-msvc` 的**生成程序和工具链**最低为 Windows 10；当前 MSVC 目标页的服务器下限是 Server 2016，不能拿普通目标证明 Win7/8/8.1 和旧 Server。Win7 专用两目标为 **Tier 3**：Rust 不提供预编译目标组件，需自行构建对应标准库／工具链或用 `build-std` 等方式；Rust 对 Tier 3 不提供必定构建的官方保证。用较新受支持系统作构建主机，不要求在 Win7 上运行 rustc。[Windows 基线变更][普通 MSVC 目标][Win7 MSVC 目标][目标等级] |
| x86/x64 | Win7 专用 MSVC 目标分别列出 i686、x86_64；须为两种位数各构建，且目标范围仅 Win7–10 x86/x64、Win11/Server x64。Rust 官方已公告从 **1.100.0** 起普通 i686 Windows 目标将停止分发宿主编译器，但继续分发其标准库；这不意味着不能从 x64 宿主构建 x86 程序，也不提升 Win7 专用目标的 Tier 3 等级。[Win7 MSVC 目标][i686 变更] |
| 单文件与运行时 | Rust 默认 `bin` 将 Rust 依赖链接入可分发可执行文件，但不能因此断言没有**原生**动态依赖。Rust Reference 的 `-C target-feature=+crt-static` 可控制适用 MSVC 目标的 CRT 静态链接，且明确要求检查成品；若仍动态使用 UCRT，旧 Windows 的 UCRT 可需系统更新（初版 KB2999226）或应用本地 DLL，后者已不是严格单文件。第三方 C/C++ DLL、插件、资源、系统补丁仍须逐项核对；不能把最新 VC++ 再分发包当老系统的通用依赖前提。[Rust 链接][UCRT 部署][VC 最新包] |
| GUI 与 API | 微软 `windows` crate 提供 Win32、COM 等 API 绑定，**绑定本身不保证调用的 API 在 Win7 可用**。可直接用 Win32 窗口和系统列表控件做本机 GUI：微软 `CreateWindowExW` 文档的桌面客户端/服务器最低为 Windows 2000，列表控件文档明确包含多列 details/report view；因此此路线不存在 Web GUI 引擎的强制运行时门槛。这只证明基本控件路径，具体进程/端口 API、控件功能和每项外部库仍须按目标最低版本审核。[Windows Rust 绑定][Win32 窗口][Win32 列表] |
| Server 与证明边界 | Rust Win7 目标文档没有逐一列 Server 2008 R2–2025 的受支持矩阵，也没有为未打 SP 的 Server 2008 R2 或桌面环境缺失情形作保证。目标可编译 ≠ 某个程序可启动、可签名验证、能完成数据查询。产出后须检查 x86/x64 PE 的 DLL/API 导入、CRT/原生依赖及补丁基线，在所列各系统的 Desktop Experience/完整安装上实测 GUI 启动和功能；目前均标为「计划支持、未测试」。[Win7 MSVC 目标][普通 MSVC 目标][Rust 链接][#4](https://github.com/hepan/win-port-search/issues/4) |

这不是选定 Rust 或提出实现方案，也不改产品支持政策；只说明**Rust 语言本身不是阻断项，Tier 3 构建供应、老系统依赖与逐版本实测是实际约束**。

[Windows 基线变更]: https://blog.rust-lang.org/2024/02/26/Windows-7.html
[普通 MSVC 目标]: https://doc.rust-lang.org/stable/rustc/platform-support/windows-msvc.html
[Win7 MSVC 目标]: https://doc.rust-lang.org/stable/rustc/platform-support/win7-windows-msvc.html
[目标等级]: https://doc.rust-lang.org/rustc/target-tier-policy.html#tier-3-target-policy
[i686 变更]: https://blog.rust-lang.org/2026/10/02/demoting-i686-windows-targets-to-std-only/
[Rust 链接]: https://doc.rust-lang.org/reference/linkage.html
[UCRT 部署]: https://learn.microsoft.com/en-us/cpp/windows/universal-crt-deployment?view=msvc-170
[VC 最新包]: https://learn.microsoft.com/en-us/cpp/windows/latest-supported-vc-redist?view=msvc-170
[Windows Rust 绑定]: https://github.com/microsoft/windows-rs/blob/master/crates/libs/windows/readme.md
[Win32 窗口]: https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-createwindowexw
[Win32 列表]: https://learn.microsoft.com/en-us/windows/win32/controls/list-view-controls-overview
