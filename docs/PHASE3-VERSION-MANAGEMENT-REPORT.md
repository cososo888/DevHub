# DevHub V2 · 第三阶段版本管理与卸载架构报告

日期：2026-11-10
状态：**版本缓存/历史、实例级管理器归因、JDK 历史保留已完成；版本目录仍受安全适配器边界限制。未进入第四阶段。**

## 1. 真实可安装版本目录（14 个软件）

当前只把“有真实、经过验证安装适配器”的版本计入可安装；没有把官方历史版本冒充为可安装。

| 软件 | 可安装版本 | 数量 | 已安装 | 来源/适配器 |
| --- | --- | --- | --- | --- |
| Python | 3.14.10、3.13.18、3.12.16、3.11.18 | 4 | 3.14.6、3.9.6 | Homebrew `python@3.14/3.13/3.12/3.11` |
| Node.js | 24.22.2 | 1 | 26.13.0 | Homebrew `node@24` |
| Git | 2.56.0 | 1 | 2.50.1 | Homebrew `git` |
| VS Code | 0 | 0 | 已安装 | 官方 app，只检测；无安全安装目录 |
| JDK | 27.0.1.0.3、25.0.5.0.3、21.0.13.0.3、17.0.21.0.3 | 4 | 26、21.0.13.0.2、17.0.14、17.0.2 | Homebrew `openjdk`、`openjdk@25/21/17` |
| Docker | 0 | 0 | 已安装 | 只检测，无安装适配器 |
| MySQL | 0 | 0 | 已安装 | 官方/独立安装，只检测 |
| PostgreSQL | 0 | 0 | 已安装 | 只检测 |
| Maven | 3.10.0 | 1 | 3.9.16 | Homebrew `maven` |
| Gradle | 9.8.2、8.14.6 | 2 | 无 | Homebrew `gradle`、`gradle@8` |
| Redis | 0 | 0 | 已安装 | 只检测 |
| Rust | 0（当前 Homebrew stable 与已安装相同） | 0 | 1.99.1 | rustup 检测；无额外可安装行 |
| IntelliJ IDEA | 0 | 0 | 已安装 | 官方 app，只检测 |
| Go | 1.27.3、1.26.10 | 2 | 无 | Homebrew `go`、`go@1.26` |

合计：**8 个软件有真实可安装目录，6 个只有检测**。

### 版本目录来源的边界

- **Homebrew formula**：真实、可安装、可卸载；版本号来自 `brew info --json=v2`。
- **本机 JDK bundle**：真实检测，来源为 Oracle/Microsoft/OpenJDK/JBR；JDK 的“安装”仍走 Homebrew，官方 vendor 下载适配器尚未接入。
- **rustup、Conda、官方 Node/Python/Go 发行版**：已识别管理器/安装，但尚未接入“列出全部官方版本 + 安全下载安装”的适配器；因此**不列出**无法安装的版本。
- 没有新增通用下载/解包引擎；没有把 V1 的完整安装引擎复制进 V2。

## 2. 安装/卸载机制

| 类型 | 安装 | 卸载 |
| --- | --- | --- |
| Homebrew formula | 固定 `brew install` | `brew uninstall`，依赖检查 |
| 外部 Homebrew formula | 不安装，只识别 | 安全卸载，实例+依赖校验 |
| 用户自有 JDK bundle | 不安装，只识别 | 完整 bundle → 废纸篓 |
| Homebrew JDK | `brew install` | `brew uninstall` |
| Conda（Python base） | 未接入 | **不卸载**：base 环境风险 |
| rustup（Rust） | 未接入新版本 | **不卸载**：当前 active/default 工具链风险 |
| 系统 Git/Python/Node 等 | 无 | 保护，不删除 |
| 官方 Maven/MySQL/app | 无 | 不做文件级删除 |
| V1 受管 | 无 | 只读 |

## 3. 本机已有实例的可卸载性

| 实例 | 路径 | 管理器/来源 | 卸载 |
| --- | --- | --- | --- |
| JDK 26 | `~/Library/Java/JavaVirtualMachines/openjdk-26` | Oracle OpenJDK | 是（废纸篓） |
| JDK 26 | `/Library/Java/JavaVirtualMachines/jdk-26.jdk` | Oracle，root | 否（管理员权限） |
| JDK 21.0.13.0.2 | `~/Library/Java/JavaVirtualMachines/ms-21.0.13.0.2` | Microsoft OpenJDK | 是 |
| JDK 17.0.19 | 已被用户移入废纸篓 | Microsoft OpenJDK | 已卸载；见第 4 节 |
| JDK 17.0.14 | `~/Library/Java/JavaVirtualMachines/jbr-17.0.14` | JetBrains Runtime | 否（IDE 共享风险） |
| JDK 17.0.2 | `~/jdk-17.0.2.jdk` | Oracle JDK | 是；当前 PATH/JAVA_HOME 使用，确认框警告 |
| 系统 Git | `/usr/bin/git` | macOS | 否 |
| Node 26 | `/usr/local/bin/node` | 独立安装 | 否 |
| Python 3.14.6 | `/opt/anaconda3/...` | Conda base | 否（base 环境） |
| Python 3.9.6 | `/usr/bin/python3` | macOS | 否 |
| Maven 3.9.16 | `/usr/local/maven` | 手动安装 | 否（边界/共享不确定） |
| Rust 1.99.1 | `~/.cargo/bin/rustc` | rustup active/default | 否（当前默认工具链） |
| MySQL/VS Code/IntelliJ | 官方路径 | 官方安装/app | 否 |

所有不可卸载行现在都有**具体管理器原因**，不再统一显示“未识别到可安全卸载的安装管理器”。

## 4. JDK 废纸篓检查

目标：`~/.Trash/DevHub-JDK-1794319608-ms-17.0.19`

- 命名由 `jdk.rs` 生成：`DevHub-JDK-<epoch 秒>-<原 bundle basename>`；`1794319608` = 2026-11-10 22:06:48。
- 原 bundle 是 `~/Library/Java/JavaVirtualMachines/ms-17.0.19`，Microsoft OpenJDK 17.0.19；源路径现已不存在。
- 卸载代码只对**目标 bundle 根目录**做一次 `rename` 到废纸篓；不会包含其他 JDK，也不会递归删除其他目录。
- 保留恢复可能：整个 bundle 在废纸篓中，可拖回原路径。
- **本次无法只读检查其内部**：macOS TCC 拒绝访问 `~/.Trash`（`Operation not permitted`）。因此不能声称已核对 contents；只能根据代码路径说明它只移动了单一根目录。
- 原有引用：本机 `PATH`/`JAVA_HOME` 当前指向 17.0.2，不指向 17.0.19；代码**不修改** IDE/Maven/Gradle/JAVA_HOME 配置，若 IDE 曾引用 17.0.19 会显示缺失，需要用户自行更新。
- 没有清空废纸篓、没有清理引用、没有修改 Shell。

### 当前 17.0.19 为什么不在列表

本次实现之前没有 `uninstall_history`/catalog cache，且卸载前的检测缓存已被刷新覆盖；`.Trash` 又被 TCC 阻止读取。因此**当前无法安全地重建 17.0.19 的历史行**，不能凭废纸篓名字伪造版本。
从本版本起，之后的 JDK/Homebrew 卸载会写入 `uninstall_history`，版本行会以“已卸载”保留；见第 5 节。

## 5. 卸载后版本列表保留

新增：

- `version_catalog_cache`：每个软件最近一次成功目录。
- `uninstall_history`：卸载版本、来源、路径、时间。
- `versions::merge_history`：
  - 仍可安装的版本 → 保留 live 行，状态从“已安装”变“安装”；
  - 当前无安装渠道 → 保留为 `historical` 行，显示“已卸载”，不伪造安装按钮；
  - 查询失败 → 使用缓存并标记 `stale`，禁用安装，保留重试；
  - 不会把本机扫描结果当完整可安装目录。

UI 测试覆盖历史行；未来卸载 17.0.19 这类 JDK 时会保留行。**当前这一次 17.0.19 因发生在历史表/缓存实现之前，仍无法恢复。**

## 6. 尚未支持

1. 官方 Node.js / Python / Go / Maven / Gradle 的全量版本目录和下载安装适配器；未接入 nvm/pyenv/官方归档，避免新增通用下载引擎。
2. Conda：只识别 base；未实现命名环境卸载。
3. rustup：只识别 active/default；未实现活动工具链卸载。
4. 官方 Node/Python/Maven/MySQL/app 的独立卸载。
5. Homebrew 外部包的**真实**卸载测试（本机没有外部 Homebrew 开发包）。
6. JDK 废纸篓内部审计（TCC 拒绝）。
7. 旧版 DevHub 受管安装保持只读。

## 7. 测试

- `npx tsc --noEmit`：通过
- `npm run build`：通过
- `npm run test:ui`：**21 passed / 0 failed**
  - 历史版本行、JDK 实例选择/原因、外部 Homebrew、进度条重挂载、极简详情、主题、搜索、分类。
- `cargo test --offline --lib`：**16 passed / 0 failed / 2 ignored**
  - 缓存/历史、JDK 路径、版本合并、归属、固定 brew 命令。
- 真实目录只读验证：8 个软件共 15 个可安装版本；6 个软件无目录。
- 真实 macOS 应用：**31.76 MiB**，已启动；数据库已创建 `version_catalog_cache`、`uninstall_history`。
- 本轮未真实安装/卸载，未清空废纸篓，未修改 V1/PATH/Shell。

## 8. 截图

- `docs/screenshots/phase3-jdk-uninstall.png`：JDK 卸载按钮与 JBR 原因。
- `docs/screenshots/phase3-jdk-confirm.png`：JDK 26 目标路径和 PATH 风险。
- `docs/screenshots/phase3-expanded-native.png`：真实 macOS 应用。
- `docs/screenshots/phase3-followup-main.png`：无标题极简主界面。
