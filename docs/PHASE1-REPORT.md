# DevHub V2 · 第一阶段验收报告

日期：2026-11-10
状态：**第一阶段已完成，等待用户亲自体验；未进入第二阶段。**

## 1. 独立项目与隔离

| 项 | 值 |
| --- | --- |
| 项目目录 | `/Users/yujian/Documents/DevHubV2` |
| Git | 全新独立仓库，本阶段首次提交；无 remote |
| 应用名 | DevHub V2 |
| Bundle ID | `com.yujian.devhub.v2` |
| 数据目录 | 本阶段不使用数据库；未创建/迁移任何 SQLite |
| 构建目录 | `DevHubV2/src-tauri/target`（已 gitignore） |
| 旧版 DevHub | `/Users/yujian/Documents/DevHub`，未修改；Git 干净 |
| 旧版备份 | `/Users/yujian/DevHub-backups/2026-11-10-pre-v2`，只读未动 |

旧版只被读取用于：核对 Tauri 配置模式、复制应用图标。未共享数据库、依赖或构建目录。

## 2. 已交付功能

- 响应式圆形软件网格，14 个软件条目；统一圆形尺寸，名称在圆下方。
- 已安装优先排序逻辑已实现（本阶段没有检测数据，所有圆形为灰色“未检测”，不展示虚假安装）。
- 分类筛选：全部 / 语言与运行时 / 开发基础 / 数据库与服务 / 开发工具。
- 放大镜展开搜索框，按软件名称过滤；关闭时清空搜索。
- 点击圆形：从原位置/尺寸开始，以 CSS FLIP 方式在 400ms 内变形成居中悬浮圆角卡片；关闭反向收缩；背景渐暗和轻微模糊。
- 支持 `prefers-reduced-motion`：直接打开目标尺寸，关闭不等待。
- 主题：跟随系统 / 浅色 / 深色，保存到 localStorage。
- 玻璃质感、低饱和渐变、柔和阴影、统一圆角；浅/深主题均实际截图验证。
- 详情卡：Logo、名称、介绍、状态、版本选择（禁用）、计划来源、安装路径、安装/卸载（禁用）、高级详情。
- 所有未实现操作均明确禁用；文案注明第一阶段不检测、不安装、不修改系统。
- 最小 Tauri IPC：`get_app_status` 只返回阶段元信息；没有安装、检测、数据库命令。

## 3. 验证结果

- `npx tsc --noEmit`：通过。
- `npm run build`：通过，产物约 237 KB JS / 11 KB CSS。
- `npm run test:ui`：**8 passed / 0 failed**。
  - 软件目录唯一性与分类；
  - 圆形网格数量、已安装 class 传递、origin 回传；
  - 搜索展开/输入/关闭清空、分类切换、主题选择；
  - 详情 morph 到目标尺寸、禁用操作、Esc 关闭；
  - reduced-motion 直接目标尺寸、立即关闭；
  - App 搜索过滤且不出现“已安装”虚假文案。
- 原生 macOS 应用：`src-tauri/target/debug/bundle/macos/DevHub V2.app`（28.19 MiB）已实际启动，窗口 1200×800。
- 截图：
  - `docs/screenshots/phase1-grid-native.png`：**真实 macOS 原生窗口**（2600×1800 捕获）。
  - `docs/screenshots/phase1-detail-light.png`、`phase1-detail-dark.png`：同一前端在无头 Chromium 中的详情卡截图。
  - `docs/screenshots/phase1-grid-dark.png`：深色网格。

## 4. 第一阶段明确未做

- 真实环境检测、软件版本读取；
- 任何下载、安装、卸载、Homebrew 调用；
- PATH、Shell、系统配置修改；
- SQLite / 数据迁移；
- 托盘常驻、后台任务、系统通知、安装队列；
- 正式 Logo 资产（当前为自绘的占位品牌标识，后续需替换为有许可的官方 Logo）；
- 生产签名/公证（本阶段仅 debug 应用）。

## 5. 已知限制

1. 详情截图通过无头 Chromium 点击生成，因为 macOS 辅助功能权限不可用，未在原生窗口自动点击；原生窗口截图和 DOM 测试覆盖了网格与交互。
2. 软件圆形图标是自绘占位标识，不是官方 Logo；发布前必须替换并确认商标许可。
3. 当前无设置页面；主题切换位于顶栏，第二阶段再做设置分类。
4. 搜索框在最后一次原生截图中可见，属交互状态；不影响功能。
5. 旧版数据、旧版 PATH 受管块均未被读取或修改。

## 6. 下一阶段入口（未开始）

Phase 2 才接入：Homebrew 只读清单、PATH/已知目录检测、V1 受管安装只读识别、统一实例模型与缓存。
在用户体验并确认本阶段前，不开始 Phase 2。
