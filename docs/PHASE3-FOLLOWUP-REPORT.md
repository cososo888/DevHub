# DevHub V2 · 第三阶段后续修复报告

> 后续补充：JDK 卸载能力已实现，见 `docs/PHASE3-JDK-UNINSTALL-REPORT.md`；本报告第 2 节中“JDK 不支持卸载”的结论已作废。

日期：2026-11-10
状态：**外部 Homebrew 卸载、进度条动画、主界面文案三项修复完成；未进入第四阶段。**

## 1. 修改文件

| 文件 | 修改 |
| --- | --- |
| `src-tauri/src/versions.rs` | 外部 Homebrew 实例的安全卸载判断；依赖检查；版本行能力 |
| `src-tauri/src/lib.rs` | 卸载公式白名单/实例校验；外部包卸载；版本化命令 |
| `src-tauri/src/homebrew.rs` | 公式信息与依赖查询 |
| `src-tauri/src/storage.rs` | 版本级归属查询；移除不再使用的宽泛查询 |
| `src/components/Orb.tsx` / `OrbGrid.tsx` | 用圆形本身作为动画起点；展开时隐藏源圆形 |
| `src/components/DetailOverlay.tsx` | 极简版本列表；外部卸载确认；稳定进度条 |
| `src/App.tsx` / `src/styles.css` | 删除标题；进度动画；间距 |
| `src/hooks/useInstallTask.ts` / `useSoftwareVersions.ts` | 版本参数与刷新 |
| `scripts/ui.test.mjs` | 外部卸载、重复打开、标题移除测试 |
| `docs/screenshots/phase3-followup-*.png` | 新截图 |

## 2. 外部安装软件的卸载支持

**判断条件不再是“是否 DevHub 安装”**，而是：

1. 能定位到 Homebrew 的 formula；
2. 能定位到对应的真实安装实例（`/Cellar/<formula>/` 或 `/opt/<formula>/`）；
3. formula 当前确实已安装；
4. `brew uses --installed <formula>` 没有其他依赖；
5. 公式在审核白名单内；
6. 不是系统软件、V1 管理、未知路径或无法确认归属的安装。

满足这些条件的 **外部 Homebrew 安装也会显示卸载按钮**。DevHub 自己安装的包仍按归属记录管理；卸载外部包不会写入新的安装归属，也不会接管它。

### 当前本机支持情况

| 本机安装 | 是否支持卸载 | 原因 |
| --- | --- | --- |
| 系统 Git `/usr/bin/git` | 否 | 不是 Homebrew；系统保护 |
| Maven `/usr/local/maven` | 否 | 未识别为 Homebrew 安装 |
| Node `/usr/local/bin/node` | 否 | 未识别为 Homebrew 安装 |
| Python `/usr/bin`、Anaconda、Framework | 否 | 系统/Conda/官方安装，未实现相应管理器卸载 |
| JDK 6 个安装 | 否 | 官方/手动安装，未识别为 Homebrew |
| Rust `~/.cargo/bin/rustc` | 否 | rustup 管理；未实现 rustup 卸载 |
| MySQL `/usr/local/mysql` | 否 | 官方安装，未识别为 Homebrew |
| VS Code / IntelliJ IDEA | 否 | 官方 app bundle；不做文件级删除 |
| 任何本机 **Homebrew formula** 安装（Git/Go/Maven/…） | **是**（如果实例和依赖检查通过） | 使用对应 formula 卸载 |
| V1 受管安装 | 否 | 严格只读，不修改 V1 |
| DevHub 安装 | 是 | 版本级归属；仍使用 Homebrew 卸载 |

**当前机器上没有任何 Homebrew 管理的开发工具**（`brew list --versions` 只有 ncmdump/taglib/utf8cpp），所以当前软件行的“卸载”按钮为空；这是安全结果，不是遗漏。外部 `brew install` 的受支持工具会在重新检测后获得卸载按钮。

## 3. 卸载安全与确认

- 前端只传 `softwareId + version + formula`，后端再次验证 formula 属于该软件白名单、安装实例路径、已安装状态和依赖。
- 外部 Homebrew 包不写归属，不自动接管；DevHub 归属记录仍用于精确标记。
- 同一版本有系统 + Homebrew 实例时，只卸载 Homebrew 那一份；确认文案提示其他来源不受影响。
- 卸载后重新检测；系统 Git 额外检查 `/usr/bin/git`。
- 不执行 `--zap`、`cleanup`、`upgrade`；不直接删除文件；不修改 PATH/Shell/V1。

本轮**未执行真实外部包卸载**（本机没有可安全测试的外部 Homebrew 开发包）；UI 和后端路径已测试，真实外部 Homebrew 卸载需你后续安装一个外部 Homebrew 工具后验证。

## 4. 进度条异常：根本原因

旧实现：

```html
<div class="indeterminate"><span /></div>
```

```css
.indeterminate span {
  width: 38%;
  animation: indeterminate 1.2s ease-in-out infinite;
}
@keyframes indeterminate {
  0% { transform: translateX(-110%); }
  100% { transform: translateX(300%); }
}
```

问题：

1. 蓝线是一个子元素，使用**相对自身宽度的 transform**；它依赖进度条容器宽度。
2. 详情卡从圆形（约 98px）扩张到 380px，进度条容器在动画期间持续变宽，百分比宽度/位移导致视觉位置被重新缩放。
3. 关闭后再打开，进度条在新的卡片里重新挂载，动画从一个很小的容器宽度重新开始；同时详情内容有展开/淡入，蓝线相位与宽度不同步，出现半途跳回。
4. 隐藏的测量节点也渲染了同一个动画，形成重复动画实例。
5. 卡片外壳的共享元素动画与进度条子元素的 transform 在同一视觉层叠加，放大了跳变感。

## 5. 修复后的实现

- 进度条改为**单一轨道伪元素**，不再有子元素 transform：

```css
.indeterminate::after {
  content: "";
  position: absolute;
  left: -40%;
  width: 40%;
  animation: indeterminate-sweep 1.2s linear infinite;
}
@keyframes indeterminate-sweep {
  from { left: -40%; }
  to { left: 100%; }
}
```

- 动画作用在稳定轨道上，与卡片宽度/共享元素 transform 分离。
- 每轮蓝线从左侧完全外进入，离开右侧后下一轮从左侧进入；不伪造百分比。
- `prefers-reduced-motion` 下停用动画，显示静态位置。
- 卡片重新挂载时动画从稳定轨道重新开始，不受卡片宽度过渡影响。

## 6. 重复打开验证

模拟安装任务（不执行真实安装）连续测试：

- 连续打开/关闭 **5 次**：每次 `.indeterminate` 都是单节点、无子元素，`animationName=indeterminate-sweep`。
- 切换 Git 再返回 Go：动画仍为 `indeterminate-sweep`。
- 关闭 90ms 时内容淡出，外壳收缩；关闭后源圆形恢复。
- 进度条位置采样连续，蓝线经过右边界后从左侧重新开始，不再在中途跳回。
- 截图：`phase3-followup-progress.png`、`phase3-followup-progress-reopen.png`。

## 7. 主界面文案

- 删除「选择你的开发工具」及其说明性副标题。
- 不增加新的标题或宣传语。
- 保留搜索、分类、圆形网格、检测状态、刷新、排序。
- 调整 `.main` / `.grid-heading` 间距，避免顶部空白。
- 截图：`phase3-followup-main.png`、`phase3-followup-native.png`。

## 8. 测试结果

- `npx tsc --noEmit`：通过。
- `npm run build`：通过。
- `npm run test:ui`：**18 passed / 0 failed**。
  - 外部 Homebrew 版本可卸载并传入 formula；
  - 无归属系统版本不显示卸载；
  - 进度条单轨道、5 次重挂载无子节点、动画名称稳定；
  - 极简详情、真实版本列表、确认、错误重试、键盘滚动、锚点、主题、搜索、分类。
- `cargo test --offline --lib`：**12 passed / 0 failed / 2 ignored**。
  - 版本合并/排序、归属记录、固定 brew 命令（无 zap/cleanup）、Homebrew 缺失、缓存。
- 真实 macOS 应用：`DevHub V2.app` 31.51 MiB，已启动；主界面无标题截图已捕获。
- 本轮未执行真实安装/卸载；模拟任务仅用于动画验证。

## 9. 尚未完成 / 限制

1. 当前机器没有外部 Homebrew 开发包，外部卸载只完成代码/UI 测试，未在真实外部包上执行。
2. 当前本机的系统 Git、Node、Python、JDK、Maven、Rust、MySQL、VS Code、IntelliJ 均没有可靠统一卸载方式，按安全原则不显示卸载。
3. Conda/rustup/官方 app 卸载尚未接入；不能直接删文件。
4. 仍只支持 8 类 Homebrew 工具的安装；其他软件只检测。
5. 未进入第四阶段（后台任务、托盘、通知）。

## 10. 验收

请检查：
- 主界面无「选择你的开发工具」；
- 已安装/未安装及版本列表正常；
- 外部 Homebrew 工具安装后出现安全卸载（需你用真实外部 brew 包验证）；
- 模拟安装任务下多次打开详情进度条平滑循环；
- 原有的圆形动画、无遮罩、极简详情和主题没有回归。

完成后停止，不进入第四阶段。
