# DevHub V2 · 第二阶段验收报告

日期：2026-11-10
状态：**第二阶段真实检测完成，等待验收；未进入第三阶段，未开发安装/卸载。**

## 1. 范围

本阶段只做只读检测：

- PATH 与常见安装目录；
- Homebrew 已安装 formula/cask；
- 官方应用包；
- 旧版 DevHub 受管目录（严格只读）；
- V2 预留受管目录（只读扫描，当前为空）。

没有执行 `brew install/upgrade/uninstall/cleanup`，没有修改 V1 安装、V1 PATH、Shell、系统环境变量或 Homebrew 状态。

## 2. 实现结构

| 模块 | 作用 |
| --- | --- |
| `src-tauri/src/detection.rs` | 固定工具注册表、PATH/已知目录扫描、Homebrew JSON、官方应用、V1/V2 受管目录、去重与版本探针 |
| `src-tauri/src/process.rs` | 固定绝对程序与参数、`env_clear`、超时、输出上限、进程组终止 |
| `src-tauri/src/storage.rs` | V2 独立 SQLite 缓存（`devhub-v2.sqlite3`） |
| `src-tauri/src/lib.rs` | AppState、启动缓存、后台刷新线程、`detection://updated` 事件 |
| `src/hooks/useDetection.ts` | 启动先读缓存，再后台刷新；订阅事件；手动刷新 |
| `src/App.tsx` | 已安装排序、蓝色圆形、检测状态/刷新、真实实例传入详情 |
| `src/components/DetailOverlay.tsx` | 单列展示版本/来源/路径/健康状态；保持锚点动画与无遮罩 |

## 3. 真实检测环境

- macOS arm64。
- Homebrew：`/opt/homebrew/bin/brew`，7.0.12。
- Homebrew 当前只安装了 `ncmdump`、`taglib`、`utf8cpp`，没有目录中的开发工具 formula/cask；Homebrew 适配代码未执行任何写入。
- 旧版 QA profile 中存在 Node.js 26.13.0 受管安装；V1 正式 profile 无受管版本；V1 `versions/go` 为空。

## 4. 真实检测结果（17 个实例，9 个软件）

| 软件 | 版本 | 来源 | 路径 | 健康 |
| --- | --- | --- | --- | --- |
| Node.js | 26.13.0 | 旧版 DevHub（devhubV1） | `~/Library/Application Support/com.yujian.devhub.validation/storage-v1/versions/node/26.13.0-arm64/bin/node` | 正常 |
| Node.js | 26.13.0 | PATH | `/usr/local/bin/node` | 正常 |
| Python | 3.9.6 | PATH | `/usr/bin/python3` | 正常 |
| Python | 3.14.6 | PATH | `/opt/anaconda3/bin/python3` | 正常 |
| Python | 3.14.6 | PATH | `/Library/Frameworks/Python.framework/Versions/3.14/bin/python3` | 正常 |
| Git | 2.50.1 | PATH | `/usr/bin/git` | 正常 |
| JDK | 26 | PATH | `~/Library/Java/JavaVirtualMachines/openjdk-26/Contents/Home/bin/java` | 正常 |
| JDK | 26 | PATH | `/Library/Java/JavaVirtualMachines/jdk-26.jdk/Contents/Home/bin/java` | 正常 |
| JDK | 21.0.13.0.2 | PATH | `~/Library/Java/JavaVirtualMachines/ms-21.0.13.0.2/Contents/Home/bin/java` | 正常 |
| JDK | 17.0.2 | PATH | `~/jdk-17.0.2.jdk/Contents/Home/bin/java` | 正常 |
| JDK | 17.0.19 | PATH | `~/Library/Java/JavaVirtualMachines/ms-17.0.19/Contents/Home/bin/java` | 正常 |
| JDK | 17.0.14 | PATH | `~/Library/Java/JavaVirtualMachines/jbr-17.0.14/Contents/Home/bin/java` | 正常 |
| Maven | 3.9.16 | PATH | `/usr/local/maven/bin/mvn` | 正常 |
| Rust | 1.99.1 | PATH | `~/.cargo/bin/rustc` | 正常 |
| MySQL | 26.7.0 | PATH | `/usr/local/mysql/bin/mysql` | 正常 |
| Visual Studio Code | 1.144.1 | 官方应用 | `/Applications/Visual Studio Code.app` | 正常 |
| IntelliJ IDEA | 2026.1 | 官方应用 | `/Applications/IntelliJ IDEA.app` | 正常 |

**未检测到**：Go、Gradle、PostgreSQL、Redis、Docker。它们在本机没有 PATH 可执行文件、没有对应 Homebrew 包、没有检测到官方应用；在网格中保持灰色，详情显示“未安装”。

**未验证实例**：本机真实扫描中没有出现未验证实例。未验证路径/无法执行探针的状态由代码和 DOM 测试覆盖，不会把未知状态标成正常。

Homebrew 状态：可用，`7.0.12`，前缀 `/opt/homebrew`；本机没有目录内开发工具的 brew 包，所以 Homebrew 来源实例为 0，这是真实结果。

## 5. 检测机制

- PATH：遍历全部 PATH 目录，不只看第一个；按真实路径（canonicalize）去重。Node 的 V1 与 PATH 两个文件是不同实际文件，保留两条实例。
- 已知目录：Homebrew、`/usr/local`、`/usr/bin`、Anaconda、`~/.cargo`、`~/.nvm`、`~/.pyenv`、Java VirtualMachines、`~/jdk-*.jdk`、V1/V2 受管版本。
- Homebrew：`brew --prefix`、`brew --version`、`brew info --json=v2 --installed`；超时 30 秒、输出上限 16 MiB；只读。
- 官方应用：读取 `Contents/Info.plist` 的 `CFBundleShortVersionString`（PlistBuddy，固定参数）。
- V1：只扫描 `versions/<软件>/<版本>/...`，不读取/写入 V1 数据库，不执行 V1 安装状态机。
- V2：扫描独立 `com.yujian.devhub.v2` 数据目录下的预留 `versions`，当前为空。
- 版本探针：所有程序均为检测到的绝对路径，固定参数，`env_clear`，超时 12 秒、单流 512 KiB，进程组超时终止；Java 同时读取 stderr。
- 受信任目录策略：仅对系统/Homebrew/官方目录、常见版本管理器、官方 JDK 与 V1/V2 受管目录执行探针；其他 PATH 路径标记“未验证”，不执行。

## 6. 缓存与刷新

- 数据库：`~/Library/Application Support/com.yujian.devhub.v2/devhub-v2.sqlite3`，schema v1，单行 `detection_cache`（payload + refreshed_at + stale）。
- 启动：前端先 `get_detection_snapshot` 读取缓存并立即渲染；随后调用 `refresh_detection` 在 Rust 后台线程执行只读检测。
- 缓存载入后 `stale=true`，UI 显示“缓存结果 · 正在重新验证”；更新事件到达后变为“已检测 … · 更新于 …”。
- 刷新失败/异常：保留上次快照但保持 `stale=true`，显示“缓存结果已过期 · 等待刷新”；不把旧缓存标记为已验证。
- 手动刷新：标题区刷新按钮；刷新中禁用并旋转。
- 快照中的 `errors` 会在标题区显示“检测警告”，详情高级信息中列出；Homebrew 不可用不会崩溃，会显示“未检测到 Homebrew”。

## 7. 测试

- `npx tsc --noEmit`：通过。
- `npm run build`：通过。
- `npm run test:ui`：**12 passed / 0 failed**。
  - 原有动画、锚点、单列、滚动条、主题、搜索、分类、reduced-motion 全部保留；
  - 新增真实实例列表、版本/来源/路径/健康、未安装状态测试。
- `cargo test --offline --lib`：**6 passed / 0 failed / 1 ignored**。
  - 版本解析、可信路径、去重来源合并、Homebrew 不存在、formula/cask 映射、SQLite 缓存 round-trip。
- `cargo test --offline --lib live_detection_report -- --ignored --nocapture`：通过，真实扫描约 **2.05 秒**，17 个实例。
- 真实 macOS 应用构建：`DevHub V2.app` 31.07 MiB，已启动。
- 真实应用数据库：`detection_cache` 已写入 17 个实例，`stale=0`，Homebrew 7.0.12。

## 8. 截图

- `docs/screenshots/phase2-grid-native.png`：**真实 macOS 窗口**，检测后蓝色/灰色圆形与已安装排序。
- `docs/screenshots/phase2-node-detail.png`：Node.js 两个版本（V1 + PATH）与路径/健康状态。
- `docs/screenshots/phase2-jdk-detail.png`：JDK 六个版本纵向列表。
- `docs/screenshots/phase2-go-detail.png`：Go 未安装状态。
- `docs/screenshots/phase2-node-dark.png`：深色主题。
- `docs/screenshots/phase2-grid-snapshot.png`：同一真实快照的 1200×800 渲染。
- `docs/phase2-detection-snapshot.json`：本次真实检测快照原始数据。

详情截图使用与原生应用相同的前端，在无头 Chromium 中注入**实际应用导出的快照**进行渲染；原生窗口截图来自真实应用。macOS 辅助功能权限不允许自动点击原生窗口，因此没有伪造原生点击。

## 9. 已知限制

1. 本机没有 Go/Gradle/PostgreSQL/Redis/Docker，只能确认“未检测到”，不能验证它们的安装检测探针；这些条目保持灰色。
2. 本机 Homebrew 没有目录内的开发工具 formula/cask，因此 Homebrew 实际映射只验证了“可用/无匹配包”；formula/cask 映射有单元测试，但未在真实 brew 开发包上执行。
3. JDK 出现两个 26、Python 出现两个 3.14.6，来自不同实际安装路径，不是同一个文件；按真实路径去重后保留。用户可能希望按“版本”合并，需产品决定。
4. 本机没有未验证实例；未验证/异常状态的展示由 DOM 测试覆盖。
5. 尚无 V2 受管安装，所以 V2 预留守管目录为空。
6. 未做安装、卸载、PATH 修改、系统通知、后台任务队列；这些属于后续阶段。

## 10. 验收对照

| 标准 | 结果 |
| --- | --- |
| 真实检测主要开发工具 | 9 个软件、17 个实例 |
| 多版本/多来源 | Node 2、Python 3、JDK 6；Node 含 V1+PATH |
| 已安装/未安装/未验证/失败区分 | 已安装蓝、未安装灰；未验证/异常有健康字段与文案；失败有错误/缓存状态 |
| Homebrew 缺失不崩溃 | 单元测试 `missing_homebrew_is_a_normal_state` |
| 不修改系统环境 | 只读命令、V1 只读、V2 独立 DB；无 brew 写命令 |
| 启动缓存 + 后台刷新 | SQLite 缓存 + 事件更新 + 手动刷新 |
| 圆形与详情一致 | 实测 9 个软件蓝色，详情实例一致 |
| 第一阶段 UI 不回归 | 12 项 DOM 测试通过；锚点/滚动条/主题保留 |
| Rust/TS/UI 测试 | 6 + 12 通过 |
| 实际启动 macOS 应用 | 已启动并截图 |

## 11. 下一步

Phase 2 完成，停止等待验收。第三阶段只按最高优先级约束推进：优先 Homebrew 安装、验证和安全卸载；不重新开发通用安装引擎，不提前实现安装代码。
