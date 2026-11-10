# DevHub V2 · 第三阶段验收报告

日期：2026-11-10
状态：**第三阶段代码与自动化验证完成，真实安装/卸载等待用户亲自操作；未进入第四阶段。**

## 1. 结论先行

本阶段只接入 **Homebrew**，首批只开放 **Git**：

- 已实现安装计划预览、用户二次确认、Homebrew 安装调用、安装后重新检测、版本验证、SQLite 归属记录、安全卸载检查与卸载确认。
- **未执行任何真实 `brew install` / `brew uninstall` / `brew upgrade` / `brew cleanup`。**
- 当前 Mac 的 Homebrew 仍未安装 Git；系统 Git 仍为 `/usr/bin/git` 2.50.1；`install_ownership` 表为空。真实安装/卸载必须由你在 DevHub V2 图形界面确认后触发。

## 2. 实际修改文件

| 文件 | 作用 |
| --- | --- |
| `src-tauri/src/homebrew.rs` | Homebrew 探测、Git formula 预览、安装/卸载命令、依赖查询、卸载安全检查 |
| `src-tauri/src/storage.rs` | `install_ownership` 表与待验证/成功/卸载状态、缓存 |
| `src-tauri/src/lib.rs` | 安装/卸载 Tauri 命令、单任务互斥、进度事件、重新检测、归属记录、系统 Git 验证 |
| `src-tauri/src/process.rs` | 受控额外环境变量（禁用自动更新/清理） |
| `src-tauri/Cargo.toml` / `Cargo.lock` | 依赖变化 |
| `src/components/HomebrewPanel.tsx` | 安装计划、确认框、不确定进度、结果/错误输出、卸载确认 |
| `src/components/DetailOverlay.tsx` | 单列详情接入安装面板与确认状态 |
| `src/components/Orb.tsx` / `OrbGrid.tsx` | 安装中的圆形不确定进度环 |
| `src/hooks/useHomebrewPreview.ts` | 安装/卸载预览 |
| `src/hooks/useInstallTask.ts` | 单任务状态、进度与完成事件 |
| `src/App.tsx` | Phase 3 状态、预览、任务、圆形进度环接线 |
| `src/lib/ipc.ts` | 新 IPC 类型 |
| `src/styles.css` | 进度环、确认框、不确定进度、Homebrew 输出 |
| `scripts/ui.test.mjs` | 安装/卸载 UI 测试 |
| `docs/screenshots/phase3-*.png`、`docs/phase3-git-preview.json` | 证据 |

## 3. Homebrew 调用流程

**只允许 Git。** 包名固定为 `git`，类型固定为 `formula`；前端只传 `softwareId`。

### 安装预览（只读）

```
brew --prefix
brew --version
brew list --versions git
brew info --json=v2 git
brew list --versions
```

环境固定：`HOMEBREW_NO_AUTO_UPDATE=1`、`HOMEBREW_NO_INSTALL_CLEANUP=1`、`HOMEBREW_NO_ENV_HINTS=1`。

真实预览结果（2026-11-10，只读）：

- Git 当前 Homebrew 版本：**2.56.0**
- 依赖：**curl、pcre2、gettext**（当前均未安装，将新增）
- 已安装：否
- 可安装：是

### 安装（用户确认后）

```
brew install --formula git
```

超时 1800 秒，输出上限 8 MiB；不执行 `--force`、`upgrade`、`cleanup`。

### 卸载预览（只读）

```
brew list --versions git
brew uses --installed git
```

要求：
- 存在 DevHub 成功归属记录；
- `preexisting=false`；
- 当前 Homebrew 仍安装该包；
- 没有其他 Homebrew 包依赖它；
- 否则阻止卸载并说明原因。

### 卸载（用户确认后）

```
brew uninstall git
```

不带 `--zap`；不执行 `brew cleanup`；不删除任何用户数据或系统文件。卸载后重新检测，并验证：
- Homebrew Git 不再存在；
- 系统 Git `/usr/bin/git` 仍然存在且健康。

## 4. Git 图形界面操作说明（待你执行）

1. 启动 DevHub V2，点击圆形 **Git**。
2. 详情显示系统 Git 2.50.1（PATH）以及 Homebrew 安装计划：
   - `git 2.56.0 · Homebrew 当前版本`
   - 依赖 `curl、pcre2、gettext`
   - `brew install --formula git`
3. 点击 **安装**，出现确认框，列出目标版本和新增依赖；此时不会执行任何命令。
4. 点击 **确认安装** 后才调用 Homebrew。
5. 圆形外围显示不确定进度环；详情显示阶段与 Homebrew 输出（完成后可展开）。
6. 成功后自动重新检测：Git 圆仍为蓝色，详情出现系统 Git 与 Homebrew Git 两条实例，Homebrew Git 标记为 DevHub 管理。
7. 点击 **卸载** 前会再次显示确认框与依赖检查结果；确认后只卸载 DevHub 记录的 Homebrew Git。
8. 卸载后详情应只剩系统 Git；`/usr/bin/git` 仍可执行。

## 5. 安装归属（V2 独立 SQLite）

表 `install_ownership`：

| 字段 | 含义 |
| --- | --- |
| `software_id` | 软件 ID（git） |
| `package` / `package_type` | git / formula |
| `status` | pending / success / failed / verification_failed |
| `preexisting` | 安装前是否已存在 |
| `installed_at` | 安装时间 |
| `uninstalled_at` | 卸载时间 |
| `instance_path` / `version` | 验证到的实际实例 |
| `message` | 失败/验证信息 |

流程：
1. 确认后写入 `pending`；
2. 执行 Homebrew；
3. 重新检测并验证到健康的 Homebrew Git 后，才写 `success`；
4. 未验证到可用实例 → `verification_failed`，不授予卸载归属；
5. 卸载成功并确认 Homebrew Git 消失后写 `uninstalled_at`。

当前表为空，因为尚未执行真实安装。

## 6. 安全设计

- 后端只接受 `softwareId`；包名、类型、命令由后端固定，前端不能传 URL、路径或 shell 命令。
- 安装/卸载单任务互斥；已有任务时拒绝第二个。
- 安装前写 pending；失败/验证失败不会变成可卸载归属。
- 外部 Homebrew 包不提供卸载；系统 Git 不可能通过此路径删除。
- 依赖检查使用 `brew uses --installed git`；有依赖时阻止。
- 不删除未知路径；不执行 `--zap`、`cleanup`、`upgrade`。
- 卸载后验证系统 Git 仍在。
- 不使用 shell 字符串；进程固定参数、超时、输出上限和进程组终止。

## 7. 测试结果

- `npx tsc --noEmit`：通过。
- `npm run build`：通过。
- `npm run test:ui`：**17 passed / 0 failed**。
  - 安装预览必须二次确认；
  - 外部 Homebrew Git 禁止卸载；
  - DevHub 归属 Git 卸载确认与依赖安全文案；
  - 不确定进度、不伪造百分比、安装结果与版本验证展示；
  - 原有锚点、单列、滚动条、主题、搜索、分类、reduced-motion 全部保留。
- `cargo test --offline --lib`：**11 passed / 0 failed / 2 ignored**。
  - 假 brew 夹具验证只调用 `install --formula git` 和 `uninstall git`，日志中无 `--zap`、`cleanup`；
  - Homebrew 不存在、formula 映射、外部包不可卸载；
  - 归属记录 pending → success → uninstalled；
  - 检测解析、去重、SQLite 缓存。
- 真实只读预览（ignored QA）：Git 2.56.0，依赖 curl/pcre2/gettext，0.75 秒。
- 真实 macOS 应用：`DevHub V2.app` 31.35 MiB，已启动并完成检测；**未点击安装确认**。

## 8. 截图

- `docs/screenshots/phase3-grid-native.png`：真实 macOS 窗口，Phase 3 网格。
- `docs/screenshots/phase3-git-panel.png`：Git 详情中的 Homebrew 安装计划（真实预览）。
- `docs/screenshots/phase3-git-confirm.png`：安装前确认框（真实预填版本/依赖；未执行安装）。
- `docs/phase3-git-preview.json`：真实 Homebrew Git 预览数据。

详情截图使用与原生应用相同的前端，在无头 Chromium 中注入真实检测快照和真实只读 Git 预览；原生窗口截图来自真实应用。没有把模拟安装结果当成真实安装。

## 9. 自动化已验证 vs 待你亲自验证

### 已由自动化/只读操作验证

- Homebrew 可用、版本 7.0.12；真实 Git 预览为 2.56.0，依赖 curl/pcre2/gettext。
- 安装确认不会在点击“确认安装”前调用后端。
- 后端命令只使用固定 `brew install --formula git` / `brew uninstall git`。
- 假 brew 日志没有 `--zap` 或 `cleanup`。
- 外部 Homebrew 包没有卸载按钮；归属记录与 pending/success 流程测试通过。
- 检测后 Git 系统版本 2.50.1、Homebrew 候选 2.56.0 共存展示。
- 应用实际启动；未改变 Homebrew 安装列表、系统 Git 或 PATH。

### 尚待你亲自操作验证

- 真实 `brew install git` 是否成功、耗时、真实输出。
- 安装后检测是否出现 Homebrew Git 实例、归属是否写入 success。
- 真实 `brew uninstall git`、依赖检查与系统 Git 仍可用的最终结果。
- 安装失败、超时或中断时的真实现场表现。

当前机器证据：`brew list --versions` 仍只有 `ncmdump`、`taglib`、`utf8cpp`；没有 git；`/usr/bin/git` 2.50.1；`install_ownership` 无记录。

## 10. 边界

未实现第四阶段的任务队列、托盘常驻、系统通知；未实现 cask、Maven/Gradle 安装，也未重新开发下载/解压/依赖解析/包管理器。下一步等待你实际操作 Git 安装与卸载验收。
