# DevHub V2 · 第一阶段验收报告

日期：2026-11-10（含当日 UI 修订）
状态：**第一阶段 UI 修订与滚动条修复完成，等待用户再次验收；未进入第二阶段。**

## 1. 独立项目与隔离

| 项 | 值 |
| --- | --- |
| 项目目录 | `/Users/yujian/Documents/DevHubV2` |
| Git | 独立仓库；无 remote |
| 应用名 | DevHub V2 |
| Bundle ID | `com.yujian.devhub.v2` |
| 数据目录 | 本阶段不使用数据库；未创建/迁移任何 SQLite |
| 构建目录 | `DevHubV2/src-tauri/target`（已 gitignore） |
| 旧版 DevHub | `/Users/yujian/Documents/DevHub`，未修改；Git 干净 |
| 旧版备份 | `/Users/yujian/DevHub-backups/2026-11-10-pre-v2`，只读未动 |

旧版只被读取用于核对 Tauri 配置与复制应用图标；没有共享数据库、依赖或构建目录。

## 2. 本次 UI 修订（针对第一轮反馈）

### 2.1 锚点变形，不再居中

- 详情卡由 `anchoredCard()` 计算：根据圆形在窗口左/右半边，选择矩形向左或向右展开；根据上下可用空间，选择向下或向上展开。
- 卡片初始几何 = 被点击圆形的真实 `getBoundingClientRect()`；随后过渡到目标矩形，关闭时反向过渡回同一圆形。
- 空间足够时，圆形所在角保持为卡片的一个角；空间不足时保持左右锚点，并把纵向位置夹紧到窗口内，绝不超出窗口。
- 动画时长 400ms，使用轻微回弹曲线；关闭路径与打开相反。
- 详情层通过 portal 挂载到 `.app` 内部，使浅/深主题变量正确继承（修复了此前 portal 到 body 导致卡片背景透明的问题）。

### 2.2 移除全屏变暗与大范围模糊

- 删除 `.detail-backdrop` 深色遮罩及背景模糊。
- 改用透明 `.detail-catcher` 只承接“点击外部关闭”，不改亮度、不加色、不模糊。
- 展开时主界面、其他圆形和背景保持原亮度。

### 2.3 近不透明卡片

- 浅色卡片底色 `#fcfdff`，深色 `#1b2332`；只保留边框、阴影和极轻的层次感，不再使用高透明度毛玻璃。
- 文字直接继承当前主题的 `--text` / `--muted`，对比度已实际截图检查。
- 关闭按钮、版本选择、操作按钮均有独立边框/底色。

### 2.4 紧凑尺寸与单列

- 卡片宽度固定为 **380px**（符合 340–400px 目标），按窗口宽度最多收缩到 `窗口宽度 - 32px`。
- 高度由隐藏测量节点按内容自然测量；超出窗口时才在卡片内部滚动，不再使用固定的 520/560/680 高度。
- 所有内容单列纵向排列：Logo/名称/介绍 → 状态 → 已安装版本 → 可安装版本 → 安装来源 → 操作 → 说明 → 高级详情。
- 未安装时不展示无意义的“安装路径”；后续有真实安装数据时再显示。
- 禁用操作使用灰色底/边框，避免蓝色主按钮造成“可点击”的误解。

## 3. 验证结果

- `npx tsc --noEmit`：通过。
- `npm run build`：通过。
- `npm run test:ui`：**10 passed / 0 failed**。
  - 目录唯一性、分类、首批 Git/Node 标记；
  - 圆形网格数量、已安装 class、origin 回传；
  - 搜索展开/输入/关闭清空、分类切换、主题选择；
  - 详情卡从 90px 圆形开始，展开到 380px、24px 圆角；Esc 关闭时宽度回到 90px；
  - 隐藏滚动条后，详情内容仍可通过 PageDown/End 等键盘操作滚动；
  - 透明 catcher 存在且不存在 `.detail-backdrop`；
  - 单列区块顺序、未安装时不出现“安装路径”、按钮禁用；
  - `anchoredCard` 的上左、下右、上右三个方向/边缘场景均不越界；
  - reduced-motion 直接打开目标尺寸并立即关闭；
  - App 搜索过滤且不出现“已安装”虚假文案。
- 原生 macOS 应用：`src-tauri/target/debug/bundle/macos/DevHub V2.app`（28.19 MiB）已重新构建并实际启动，窗口 1200×800。


## 3.1 详情卡滚动条修复

- 实际来源：`.detail-inner` 旧规则显式设置了 `scrollbar-width: thin` 和 `::-webkit-scrollbar { width: 6px; }`，内容溢出时显示右侧滚动条。
- 修复：`scrollbar-width: none`、`-ms-overflow-style: none`，WebKit 滚动条宽度设为 0；**保留 `overflow-y: auto`**，不裁切内容。
- 键盘：详情内容容器设为可聚焦（`tabIndex=0`），并增加 PageUp/PageDown/Arrow/Home/End 滚动处理；鼠标滚轮和触控板使用原生滚动。
- 浏览器实测（1200×500 视口，强制溢出）：`scrollbarWidth=0`、`overflowY=auto`，内容 `scrollHeight=522` / `clientHeight=466`；滚轮后 `scrollTop=56`，PageDown 从顶部同样到 56；没有内容被裁切。
- 1200×800 正常视口：`scrollbarWidth=0`，详情卡尺寸/位置/动画/单列布局均未改变。

## 4. 截图

详情截图按原生窗口内容高度（1200×800 视口）采集。

- `docs/screenshots/phase1-grid-native.png`：**真实 macOS 原生窗口**，圆形网格。
- `docs/screenshots/phase1-grid-light.png`：1200×800 浅色网格。
- `docs/screenshots/phase1-detail-light.png`：浅色主题，Python 圆形左上角锚点展开（卡片 380×522，无可见滚动条）。
- `docs/screenshots/phase1-detail-dark.png`：深色主题，Python 圆形锚点展开。
- `docs/screenshots/phase1-detail-edge.png`：右下角 IntelliJ IDEA，卡片向左上展开且不越界。
- `docs/screenshots/phase1-grid-dark.png`：深色网格。

## 5. 第一阶段明确未做

- 真实环境检测、版本读取；
- 下载、安装、卸载、Homebrew 调用；
- PATH、Shell、系统配置修改；
- SQLite / 数据迁移；
- 托盘常驻、后台任务、系统通知、安装队列；
- 正式 Logo 资产（当前为自绘占位标识）；
- 生产签名/公证（当前为 debug 应用）。

## 6. 已知限制

1. 原生窗口中的自动点击仍受 macOS 辅助功能权限限制；详情卡截图由同一前端在无头 Chromium 中点击生成，原生窗口截图与 DOM 测试覆盖网格和交互。
2. 圆形图标为自绘占位标识，不是官方 Logo；发布前需替换并确认许可。
3. 主题切换当前位于顶栏；独立设置页面留到后续阶段。
4. 若卡片内容高于整个窗口，才启用卡片内部滚动；当前在 1200×800 原生窗口中内容完整可见。

## 7. 下一阶段入口（未开始）

Phase 2 才接入：Homebrew 只读清单、PATH/已知目录检测、V1 受管安装只读识别、统一实例模型与缓存。
在用户再次验收本阶段前，不开始 Phase 2。
