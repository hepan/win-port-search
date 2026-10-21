# Windows 本机 TCP/UDP 数据能力调查

研究问题：Windows 7–11 和相应 Windows Server 的本机 TCP 监听、UDP 端点如何取得端口/PID/进程名/可执行文件路径，以及管理员运行后的例外。本文仅记录微软公开接口所支持的能力与可用于规格的约束；**不是实机测试结果，也不替产品决定支持/分发策略**。

## 接口与字段

| 需求 | 受支持的接口与可得数据 | 边界 |
| --- | --- | --- |
| TCP 监听端口与 PID | IP Helper `GetExtendedTcpTable(AF_INET/AF_INET6, TCP_TABLE_OWNER_PID_LISTENER)` 分别返回 `MIB_TCPTABLE_OWNER_PID` / `MIB_TCP6TABLE_OWNER_PID`；行内本地地址、网络字节序本地端口、`dwState`、`dwOwningPid`。`MIB_TCP_STATE_LISTEN` 为 2。只读监听，不必把 `*_ALL` 中已建立连接误当监听。[TCP 表][TCP IPv4 行][TCP IPv6 行] | IPv6 的 `TCP_TABLE_BASIC_*` **不受支持**；应选 `OWNER_PID_*`。表表示“应用可获得的端点”；两次查询 IPv4/IPv6 和随后查进程不是原子快照。[TCP 表] |
| UDP 本地端点与 PID | `GetExtendedUdpTable(AF_INET/AF_INET6, UDP_TABLE_OWNER_PID)` 返回 `MIB_UDPTABLE_OWNER_PID` / `MIB_UDP6TABLE_OWNER_PID`；行内本地地址、网络字节序本地端口及 `dwOwningPid`。UDP 表是**已绑定端点**而非 TCP 式 LISTEN 状态；即使仅为发送而绑定，仍可在表里。[UDP 表][UDP IPv4 行][UDP IPv6 行] | 两种 UDP 行的 PID 为 **0 表示不可获得所有者信息**，不能解释为拥有者 PID 0，更不能凭端口倒推名称。[UDP IPv4 行][UDP IPv6 行] |
| 进程名与运行路径 | 对非零 PID 调 `OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION)`，然后 `QueryFullProcessImageNameW(handle, 0, ...)` 取得 **Win32 格式、运行映像的全路径**；文件名可从此路径提取。此 API 也支持 `PROCESS_QUERY_INFORMATION`，`PROCESS_NAME_NATIVE` 则取 native 路径。另一独立进程名来源：Toolhelp `CreateToolhelp32Snapshot` + `Process32FirstW`/`Process32NextW` 的 `PROCESSENTRY32W.szExeFile`（只有文件名，不是完整路径）。[打开进程][进程权限][映像路径][Toolhelp 行] | PID 和端点表不含进程名或运行路径；它们来自**后续单独的进程查询**，可能无权访问或对象已退出；不能把 PID→路径绑定当作端点表的保证。[TCP IPv4 行][UDP IPv4 行][打开进程][映像路径] |
| 与端点关联的模块名/模块路径（不同语义） | 用 `TCP_TABLE_OWNER_MODULE_LISTENER` / `UDP_TABLE_OWNER_MODULE` 拿 OWNER_MODULE 行，再分别调用 `GetOwnerModuleFromTcpEntry`/`Tcp6Entry`/`UdpEntry`/`Udp6Entry`，`TCPIP_OWNER_MODULE_INFO_BASIC` 产出模块名/路径。[TCP 表][UDP 表][TCP 模块][TCP6 模块][UDP 模块][UDP6 模块] | 模块标识可能为 `svchost.exe`、服务 `RPC` 或组件 `timer.dll`，**不能**直接当成“进程名/进程 EXE 路径”。成功返回也可能为空字符串；应与进程映像路径区别标注。[TCP 模块][UDP 模块] |

以上两个 Extended Table 函数、四类 OWNER_MODULE API、`QueryFullProcessImageNameW` 的官方最低要求分别覆盖 Vista/Server 2008（Extended Table 和模块查询文档还列 XP SP2/Server 2003 SP1）；因此 **Win7 / Server 2008 R2 及以后并无公开的“老系统缺失进程路径 API”分界**。`PROCESS_QUERY_LIMITED_INFORMATION` 自 Vista/Server 2008 提供。API 最低支持要求不是对每行都能查到 PID/进程路径的承诺。[TCP 表][UDP 表][TCP 模块][UDP 模块][映像路径][进程权限]

## 版本与架构矩阵（能力、非产品承诺）

| 客户端 | 同期 Server（示例，非逐一精确配对） | 基础接口差异与实测边界 |
| --- | --- | --- |
| Windows 7 | Server 2008 R2（并纳入 Server 2008） | Vista/Server 2008 起的上述四路 OWNER_PID 表、`QueryFullProcessImageNameW` 均满足最低系统要求。Win7 不需靠现代 PSAPI `K32*` 命名；此研究不要求 PSAPI。[TCP 表][UDP 表][映像路径][版本] |
| Windows 8 / 8.1 | Server 2012 / 2012 R2 | 同一 API 下限，无证据显示按这一代缺少所列字段。Win8.1/Server 2012 R2 起可有受保护的反恶意软件服务；管理员身份也不能据此保证访问其内部信息。[TCP 表][UDP 表][映像路径][受保护服务][版本] |
| Windows 10 | Server 2016 / 2019 / 2022 | 同一 API 下限；差异主要是实际进程权限/保护及快照时序，不应凭 OS 年代推断每条都能给路径。[TCP 表][UDP 表][进程权限][服务器版本] |
| Windows 11 | Server 2025（仅同期产品列示） | Server 2025 为当期 LTSC；以上为 API 文档最低版本向后适用的推论，**未在各具体版本/硬件架构实机逐项验证**，不能把 Server 2025 硬说成 Win11 的同核等价版本。[TCP 表][UDP 表][映像路径][服务器版本] |

32 位观察进程运行于 64 位 Windows 时，Toolhelp 文档明确要求用 `QueryFullProcessImageName` 取 64 位目标进程的**完整路径**；不要把 32 位模块快照/PSAPI 当作跨位数的路径保证。`EnumProcessModulesEx` 文档称 WOW64 下的 32 位调用者忽略模块筛选标志。对于其他具体 CPU 架构（含 ARM64）/发行映像的“单 EXE 可运行”资格，以上 API 文档不构成证明，需单独规定构建与实测。[Toolhelp 行][PSAPI 模块]

## 即使请求管理员权限仍要处理的缺失

1. UAC：OWNER_MODULE 官方文档规定，管理员组的非内置 Administrator 未提升时，对受保护项可能**调用成功但名称/路径空串**；指定 manifest `requestedExecutionLevel=requireAdministrator` 或“以管理员身份运行”才可访问这类受保护模块信息。TCP 模块文档还特别举 Windows 系统目录下的程序作为其所谓保护情形；**不要混同下面的内核 protected process 机制**。[TCP 模块][UDP 模块][TCP6 模块][UDP6 模块]
2. 进程句柄权限：`OpenProcess` 按安全描述符校验访问；PID 0 (`System Idle Process`) 为 `ERROR_INVALID_PARAMETER`，System 和 CSRSS 的访问限制会导致 `ERROR_ACCESS_DENIED`。管理员启动 ≠ 所有句柄都能打开；即使启用 SeDebugPrivilege，也不等于绕过受保护进程的特别限制。[打开进程][进程权限]
3. Vista 起的 protected process 禁止普通调用者申请 `PROCESS_QUERY_INFORMATION`、`PROCESS_VM_READ` 等权限；较小的 `PROCESS_QUERY_LIMITED_INFORMATION` 专为部分信息提供，**不能把所有 protected-process 路径一概判作不可得，也不能承诺必可得**。Windows 8.1 / Server 2012 R2 起的受保护反恶意软件服务还能抵抗管理员进程的注入；对每个进程须以 `OpenProcess` / `QueryFullProcessImageNameW` 的实际结果判断。[进程权限][映像路径][受保护服务]
4. 即使 OWNER_MODULE API 返回成功，模块字符串也可能为空；TCP 还明确列 `ERROR_NOT_FOUND`（包括 PID 为 0/找不到）及 `ERROR_PARTIAL_COPY`。端点收集、模块查询、进程句柄查询有先后间隔；进程退出或 PID 复用导致的误关联风险属**由非原子接口推断**，产品规格应允许“PID 已显示但名称/路径不可用或已过期”，而非捏造路径。[TCP 模块][TCP6 模块][打开进程]

## 可写进规格的可验证约束（候选，尚非地图决策）

- Win7 / Server 2008 R2 到 Win11 / Server 2025 的测试矩阵至少包括 IPv4、IPv6 的 TCP 监听和已绑定 UDP 端点；核对同一个本地端口可有多条不同地址/协议/进程记录，不将 UDP 解释成连接或 TCP LISTEN。对于 Server 2008 加入额外最低版本样本。匹配时把网络字节序端口转成主机字节序。[TCP IPv4 行][TCP IPv6 行][UDP IPv4 行][UDP IPv6 行]
- 记录每行原始 PID；UDP PID 0 或无法访问 PID 时，保留端口并明确标示“所有者/进程资料不可用”，而非隐藏记录或用模块名冒充进程名。正常可查询目标则将 `QueryFullProcessImageNameW` 的 Win32 全路径和文件名分别验收。[UDP IPv4 行][UDP IPv6 行][映像路径][TCP 模块]
- 同一测试场景分别运行未提升与要求管理员提升的实例，覆盖 Windows 系统目录相关模块返回空字符串、System/CSRSS 的访问拒绝、快速退出进程，以及 32 位观察器看 64 位目标的完整路径。无法由本研究证明各版本/机器上每一行的真实值；**须在目标系统实机复核**，以 API 返回码/空字段记录证据。[TCP 模块][打开进程][Toolhelp 行]

[版本]: https://learn.microsoft.com/en-us/windows/win32/sysinfo/operating-system-version
[服务器版本]: https://learn.microsoft.com/en-us/windows/release-health/windows-server-release-info
[TCP 表]: https://learn.microsoft.com/en-us/windows/win32/api/iphlpapi/nf-iphlpapi-getextendedtcptable
[UDP 表]: https://learn.microsoft.com/en-us/windows/win32/api/iphlpapi/nf-iphlpapi-getextendedudptable
[TCP IPv4 行]: https://learn.microsoft.com/en-us/windows/win32/api/tcpmib/ns-tcpmib-mib_tcprow_owner_pid
[TCP IPv6 行]: https://learn.microsoft.com/en-us/windows/win32/api/tcpmib/ns-tcpmib-mib_tcp6row_owner_pid
[UDP IPv4 行]: https://learn.microsoft.com/en-us/windows/win32/api/udpmib/ns-udpmib-mib_udprow_owner_pid
[UDP IPv6 行]: https://learn.microsoft.com/en-us/windows/win32/api/udpmib/ns-udpmib-mib_udp6row_owner_pid
[打开进程]: https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-openprocess
[进程权限]: https://learn.microsoft.com/en-us/windows/win32/procthread/process-security-and-access-rights
[映像路径]: https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-queryfullprocessimagenamew
[Toolhelp 行]: https://learn.microsoft.com/en-us/windows/win32/api/tlhelp32/ns-tlhelp32-processentry32
[PSAPI 模块]: https://learn.microsoft.com/en-us/windows/win32/api/psapi/nf-psapi-enumprocessmodulesex
[TCP 模块]: https://learn.microsoft.com/en-us/windows/win32/api/iphlpapi/nf-iphlpapi-getownermodulefromtcpentry
[TCP6 模块]: https://learn.microsoft.com/en-us/windows/win32/api/iphlpapi/nf-iphlpapi-getownermodulefromtcp6entry
[UDP 模块]: https://learn.microsoft.com/en-us/windows/win32/api/iphlpapi/nf-iphlpapi-getownermodulefromudpentry
[UDP6 模块]: https://learn.microsoft.com/en-us/windows/win32/api/iphlpapi/nf-iphlpapi-getownermodulefromudp6entry
[受保护服务]: https://learn.microsoft.com/en-us/windows/win32/services/protecting-anti-malware-services-
