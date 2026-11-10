# DevHub V2 · 第三阶段优化与重构报告

日期：2026-11-10
状态：**详情卡片、共享元素动画、统一版本列表与多软件安装能力已完成；用户已真实验收 Go 安装/卸载。未进入第四阶段。**

## 0. 用户验收结果（2026-11-10）

用户在新版版本列表界面中真实完成 Go 安装与卸载：

| 项 | 实际结果 |
| --- | --- |
| 安装时间 | 2026-11-10 20:55:59 |
| 安装版本 | Go **1.27.3**（Homebrew formula `go`） |
| 安装路径 | `/opt/homebrew/Cellar/go/1.27.3/bin/go` |
| 安装前已存在 | 否（`preexisting=0`） |
| 卸载时间 | 2026-11-10 20:57:06 |
| 归属记录 | `status=success`，已写入 `uninstalled_at` |
| 卸载后 Homebrew Go | 已不存在，`brew list --versions go` 为空 |
| 卸载后系统 Go | 未安装（`command -v go` 无结果） |
| 归属表 | 另保留 Git 上一轮成功安装/卸载记录 |

这验证了新版本列表的真实安装 → 检测 → 归属记录 → 卸载 → 重新检测闭环。当前机器未留下 Go 安装。

## 1. 本次解决的问题

| 原问题 | 处理 |
| --- | --- |
| 只能安装 Git | Homebrew 支持扩展到 Git、Go、Maven、Gradle、Node、Python、JDK、Rust 八类工具的真实公式渠道 |
| 未安装软件没有安装操作 | 每个真实可安装版本行提供“安装” |
| “Homebrew 当前版本”不明确 | 版本列表直接显示真实版本号（例如 2.56.0、1.27.3、9.8.2） |
| 详情信息冗余 | 只保留图标、名称、一句话介绍、版本列表 |
| 用户被迫理解 Homebrew | 详情不展示来源、路径、包名、依赖或安装机制；主界面也移除了 Homebrew 技术文案 |
| 展开/关闭有拼接感 | 单一外壳 + 隐藏原圆形 + 连续 Logo 变形 + 内容先后淡入淡出 |

## 2. 实际修改文件

| 文件 | 作用 |
| --- | --- |
| `src-tauri/src/homebrew.rs` | 真实 Homebrew formula 元数据、版本渠道、固定 install/uninstall 命令 |
| `src-tauri/src/versions.rs` | 已安装实例 + 可安装 formula 合并、去重、排序、安装/卸载能力 |
| `src-tauri/src/lib.rs` | 版本查询命令、按版本安装/卸载、单任务、重新检测、归属记录 |
| `src-tauri/src/storage.rs` | 版本级安装归属查询 |
| `src/components/DetailOverlay.tsx` | 极简单列版本列表、单一变形外壳、连续 Logo、确认与进度 |
| `src/components/Orb.tsx` / `OrbGrid.tsx` | 用圆形本身作为动画起点；展开时隐藏源圆形 |
| `src/hooks/useSoftwareVersions.ts` | 真实版本目录查询与重试 |
| `src/hooks/useInstallTask.ts` | 按版本跟踪安装/卸载任务 |
| `src/App.tsx` / `src/lib/ipc.ts` / `src/styles.css` | 接线、类型和视觉系统 |
| `scripts/ui.test.mjs` | 新版本列表、确认、错误、进度测试 |
| `docs/screenshots/phase3-refine-*.png`、`docs/phase3-refine-version-catalogs.txt` | 证据 |

`HomebrewPanel.tsx` 与旧 `useHomebrewPreview.ts` 已删除；不再有独立 Homebrew 区块。

## 3. 新详情卡片

默认只有：

- 软件图标（同一位移/缩放连续变形）
- 软件名称
- 一句话介绍
- 版本行：`2.56.0 — 安装`、`2.50.1 — 已安装`、有归属时“卸载”

不再常驻：安装状态大框、已安装/可安装两个区块、安装来源、Homebrew 区块、后续支持、安装机制说明、技术路径、健康徽章、分隔线堆叠。

版本行规则：

- 后端合并已安装实例与 Homebrew 真实渠道；同版本不同路径显示一行，实例仍保留在后端。
- 已安装版本显示“已安装”；只有 DevHub 归属的实例显示“卸载”。
- 未安装且 `brew info` 真实可解析的版本显示“安装”。
- 已安装的版本不重复展示安装按钮。
- 查询失败显示“版本查询失败 + 重试”。
- 安装/卸载确认在对应版本行内展开，不再出现大块确认面板。

## 4. 真实版本与安装支持

只调用 **Homebrew formula**，没有自研下载、解压、依赖解析或包管理器。

| 软件 | 可安装的真实版本（Homebrew 当前渠道） | 已安装情况 |
| --- | --- | --- |
| Git | 2.56.0 | 系统 2.50.1 |
| Go | 1.27.3 | 无 |
| Maven | 3.10.0 | 系统 3.9.16 |
| Gradle | 9.8.2 | 无 |
| Node.js | 24.22.2（`node@24`） | 系统 26.13.0；当前 node 26.13.0 已安装，故不重复提供 |
| Python | 3.14.10、3.13.18、3.12.16 | 3.14.6、3.9.6 |
| JDK | 27.0.1.0.3、21.0.13.0.3、17.0.21.0.3 | 26、21.0.13.0.2、17.0.19、17.0.14、17.0.2 |
| Rust | 无不同版本（Homebrew 1.99.1） | 1.99.1；同版本不重复安装 |

说明：

- Node `node@22` 在 Homebrew JSON 中已 deprecated，未展示；`node@20` disabled，未展示。
- 所有“安装”行在真实环境测试中逐一调用 `brew info --json=v2` 解析并再次用 `resolve_install` 校验，确保不是虚构版本。
- 版本来自 Homebrew 当前 formula；不提供 Homebrew 没有的历史版本。
- 其他软件（MySQL、PostgreSQL、Redis、Docker、VS Code、IntelliJ）本阶段不提供安装入口；已安装的仍会在列表显示。
- Rust 本机 Homebrew 版本与已安装版本相同，因此没有可安装行；不是伪造“安装”按钮。

## 5. 安装/卸载后端

- `get_software_versions(softwareId)`：检测实例 + Homebrew 元数据 + 归属记录 → 排序后的版本行。
- `start_homebrew_install(softwareId, version)`：后端解析该版本对应的真实 formula，拒绝已安装/无法解析版本；执行固定 `brew install --formula <formula>`。
- `start_homebrew_uninstall(softwareId, version)`：只允许存在该版本的 DevHub 成功归属；检查 `brew uses --installed`；执行 `brew uninstall <formula>`。
- 单任务互斥；安装/卸载后重新检测并验证；失败不会授予归属。
- 归属记录精确到 software/version/package；外部安装不可卸载；系统 Git 不可删除。
- 不执行 `brew upgrade`、`brew cleanup`、`--zap`；不删除未知路径；不修改 PATH/Shell。

## 6. 展开/关闭动画：根本原因与修复

### 根本原因

1. 点击时取的是**整个 orb 按钮**的矩形，包含名称区域，不是圆形本身；动画起点比圆形大。
2. 原圆形仍留在网格中，卡片是另一个独立元素，形成两个外壳。
3. 卡片从第一帧就使用卡片底色/边框/阴影，而圆形有自己的渐变/边框/阴影，边缘不一致。
4. 内容透明度从 0 开始，圆形 Logo 立即消失，随后头部 Logo 出现，造成图标跳跃。
5. 关闭时卡片收缩与原圆形重新出现没有严格衔接，内容与外壳同时变化。

### 修复

- 点击时读取 `.orb-circle` 的 rect；动画严格从圆形边缘开始。
- 展开时给源圆形加 `.is-source`，保持布局但隐藏；直到关闭动画结束并卸载后才恢复。
- 使用**单一 `.morph-card` 外壳**：初始背景/边框/阴影/圆角取自圆形，随后过渡到卡片表面；没有第二个圆形/卡片边框。
- 单一 `.morph-logo` 覆盖层：从圆形中心 52% 尺寸连续移动到卡片左上角 44px；没有 Logo 跳变。
- 展开：外壳 400ms，内容延迟 160ms 淡入；关闭：内容先约 100ms 淡出，外壳 320ms 收回。
- 支持 `prefers-reduced-motion`；增加窗口 resize 重新布局；关闭使用一次性 timer，避免快速点击残影。
- 透明 catcher 只负责关闭，不加遮罩/模糊。

### 实测（1200×800）

| 阶段 | 结果 |
| --- | --- |
| 点击前圆形 | left 342.44 / top 261.69 / 98.39×98.39 |
| 展开早期 | 卡片同位置同尺寸 98.39²，圆角 49.2，背景为圆形表面，Logo 50.1 居中，源圆形 hidden，内容 opacity 0 |
| 展开完成 | 卡片 380×209，圆角 24，背景切换为卡片底色，Logo 44px，内容 opacity 1 |
| 关闭 90ms | 内容 opacity 0.016，卡片宽度 160，源圆形仍 hidden |
| 关闭完成 | 卡片移除，源圆形 visibility 恢复 |
| 缩放到 1000×700 | 卡片 right 722 / bottom 470，仍在窗口内 |

没有出现双重边框、白边、接缝或图层闪烁；详情截图使用真实检测快照与真实只读版本目录渲染。

## 7. 测试

- `npx tsc --noEmit`：通过。
- `npm run build`：通过。
- `npm run test:ui`：**16 passed / 0 failed**。
  - 极简详情结构；版本行安装/卸载确认；无归属不出现卸载；错误重试；逐版本进度；隐藏源圆形；键盘滚动；锚点几何；搜索/分类/主题/reduced-motion。
- `cargo test --offline --lib`：**12 passed / 0 failed / 2 ignored**。
  - 版本合并/排序、归属 pending→success、固定 brew 命令（无 zap/cleanup）、Homebrew 缺失、缓存。
- 真实只读版本目录 QA：8 个软件全部查询成功；每个“安装”行再次通过 `resolve_install` 校验，耗时约 12.84 秒。
- 真实 macOS 应用：31.51 MiB，已启动并完成检测；**本轮未执行真实安装或卸载**。

## 8. 尚未完成 / 限制

1. 本轮的安装/卸载代码改动尚未由你亲自做真实安装验证；上一轮你验收的是旧版 Git 流程。需重新在界面验证新版本列表的 Git/Go 等安装。
2. 仅支持 Homebrew 当前可用版本；Homebrew 没有的历史版本不显示。
3. Node 当前版本与系统版本相同，仅提供 `node@24`；Node 22 已 deprecated，未展示。
4. Rust 本机 Homebrew 版本与已安装版本相同，暂无可安装行。
5. 未接入 MySQL、PostgreSQL、Redis、Docker、VS Code、IntelliJ 安装；它们只显示已安装检测结果。
6. 仍未实现第四阶段的后台任务队列、托盘常驻、通知。
7. 未重写软件网格；动画层局部重构。

## 9. 验收对照

| 要求 | 结果 |
| --- | --- |
| 极简详情 | 只保留图标/名称/简介/版本列表 |
| 真实版本号 | 全部来自 Homebrew 元数据或真实检测 |
| 未安装软件可安装 | Go、Gradle 等有真实安装按钮 |
| 无错误按钮 | 不可安装无按钮；系统安装无卸载 |
| 同版本多路径合并 | 后端合并、保留实例，前端一行 |
| 动画连续 | 单一外壳 + 圆点起点 + 连续 Logo |
| 消除拼接 | 源圆形隐藏、无第二外壳、无双重边框 |
| 不同位置展开 | 锚点/边缘测试与真实截图 |
| 快速点击/缩放 | 关闭 guard、resize 重新布局 |
| 浅/深主题 | 截图 + 测试 |
| 安装后刷新 | 版本 key 依赖检测 refreshedAt 与任务结果 |
| 卸载保护其他来源 | 版本级归属 + 包级剩余检测 |
| 成熟工具 | 仅 Homebrew，无自研引擎 |
| V1/系统安全 | V1 只读；不修改 PATH/Shell/系统 Git |

## 10. 验收结果

用户已在新版界面完成 Go 1.27.3 的安装与卸载，验收通过。当前 Homebrew 无 Go，归属表记录已卸载；未进入第四阶段。
