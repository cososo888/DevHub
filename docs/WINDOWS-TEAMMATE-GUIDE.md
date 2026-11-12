# DevHub V2 Windows 版 · 队友开发指南

写给第一次接触这个项目、也不了解 DeepSeek Harness / GitHub 的同学。

---

## 0. 先看懂这个项目是什么

**DevHub V2** 是一个 macOS/Windows 桌面应用：

- 首页是圆形图标软件网格；
- 点击软件打开一个卡片；
- 显示本机是否已安装、版本和路径；
- 点“安装”时，后台调用系统包管理器；
- 不提供版本选择，不自行下载安装包；
- 不删除用户文件，不静默提权。

macOS 版已经完成，使用 **Homebrew**。

Windows 版要做的是：

> 保留同一套 UI 和检测思路，把底层包管理器换成 **winget**。

Windows 版**不是**把 macOS 的 Homebrew 命令照搬过去，也不是重新做一个包管理器。

---

## 1. 你不需要安装 DeepSeek Harness

DeepSeek Harness 是我这边用来生成/修改代码的 AI 开发环境，**不是 DevHub 的依赖**。

你不需要：

- 安装 DeepSeek Harness；
- 购买或登录 DeepSeek；
- 使用任何 AI 工具。

你只需要：

- Windows 10/11 x64；
- VS Code 或任意编辑器；
- Windows 终端（PowerShell）；
- Git、Node.js、Rust、Visual Studio Build Tools。

如果你愿意用 AI 辅助，可以自己用；但它不是项目运行条件。

---

## 2. 先拿到仓库权限

仓库地址：

https://github.com/cososo888/DevHub

这是 **private 仓库**。项目所有者需要先邀请你：

1. GitHub 仓库 → **Settings** → **Collaborators**；
2. **Add people**；
3. 输入你的 GitHub 用户名；
4. 你接受邮件/站内邀请。

如果没有权限，`git clone` 会失败。

### 不会用命令行？用 GitHub Desktop

1. 安装 GitHub Desktop：https://desktop.github.com/
2. 登录你的 GitHub 账号；
3. **File → Clone repository → URL**；
4. 输入：
   ```text
   https://github.com/cososo888/DevHub.git
   ```
5. 选择本地目录，克隆；
6. 克隆后切换到分支：
   **Branch → windows-handoff**

### 会用命令行

```powershell
git clone --branch windows-handoff https://github.com/cososo888/DevHub.git
cd DevHub
```

**不要直接在 `main` 上开发。**  
在 `windows-handoff` 分支上工作，完成后发 Pull Request。

---

## 3. Windows 环境准备

在 Windows x64 上安装：

### 3.1 Git

```powershell
winget install --id Git.Git -e
```

### 3.2 Node.js LTS

```powershell
winget install --id OpenJS.NodeJS.LTS -e
```

### 3.3 Rust（MSVC 工具链）

```powershell
winget install --id Rustlang.Rustup -e
rustup default stable-x86_64-pc-windows-msvc
```

### 3.4 Visual Studio Build Tools

安装 **Visual Studio 2022 Build Tools**，必须勾选：

> 使用 C++ 的桌面开发

没有这个，Rust/Tauri 会报 `link.exe not found`。

### 3.5 WebView2 Runtime

Windows 11 通常自带；Windows 10 建议安装：

```powershell
winget install --id Microsoft.EdgeWebView2Runtime -e
```

### 3.6 验证

```powershell
git --version
node -v
npm -v
rustc -V
cargo -V
winget --version
```

把版本号记录到测试报告里。

---

## 4. 先不要急着构建

当前仓库的 Windows 支持**还没有实现**。

在 Windows 上直接执行：

```powershell
npm ci
npm run tauri -- build --bundles nsis
```

会失败，因为：

- `src-tauri/src/process.rs` 使用了 Unix 专用 API；
- 后端命令直接依赖 Homebrew；
- 检测模块包含 macOS 路径；
- 没有 WingetProvider。

这是正常的，不是你的环境问题。

---

## 5. 当前项目结构

```text
src/                         React 前端
  App.tsx                    首页、搜索、详情接线
  catalog.ts                 macOS 100 款目录
  components/DetailOverlay.tsx
  hooks/useInstallTask.ts
  lib/ipc.ts
  lib/catalogFilter.ts       全局搜索

src-tauri/                   Rust/Tauri 后端
  src/lib.rs                 Tauri 命令
  src/homebrew.rs            macOS 包适配
  src/detection.rs           本机检测
  src/process.rs             进程执行器（Unix 专用）
  src/storage.rs             缓存/历史
  tauri.windows.conf.json    NSIS 配置

docs/windows/
  winget-candidates.json     100 个 Windows 候选 ID（未验证）
  verify-winget.ps1          Windows 验证脚本
  WINDOWS-HANDOFF.md         交接说明
```

先读这三份：

1. `docs/WINDOWS-PORT-REPORT.md`
2. `docs/windows/WINDOWS-HANDOFF.md`
3. 本文件

---

## 6. 第一个任务：让 Windows 能编译

### 6.1 分开 Unix/Windows 进程执行器

`src-tauri/src/process.rs` 当前有：

```rust
use std::os::unix::process::CommandExt;
libc::kill(...)
```

Windows 没有这些。

需要：

- 用 `#[cfg(unix)]` 保留 macOS 逻辑；
- 用 `#[cfg(windows)]` 实现 Windows 版本：
  - `std::os::windows::process::CommandExt::creation_flags`
  - `CREATE_NO_WINDOW = 0x08000000`
  - 仍然使用固定 argv；
  - 禁止 Shell；
  - 设置 stdout/stderr 上限；
  - 超时后清理子进程；
  - 不继承奇怪的环境变量。
- 还要处理 Tauri 在 Windows 下没有 `libc` 的问题。

### 6.2 把 Homebrew 限定为 macOS

`homebrew.rs` 只应该在 macOS 编译/使用。

Windows 端新建：

```text
src-tauri/src/provider/
  mod.rs
  winget.rs
```

在 `lib.rs` 中根据 `#[cfg(target_os = "windows")]` 调用 winget。

**不要为了快速编译而删掉 macOS 检测/安装代码。**

---

## 7. 第二个任务：验证 100 个 winget ID

`docs/windows/winget-candidates.json` 里有 100 个候选 ID，但**尚未在 Windows 验证**。

执行：

```powershell
cd docs\windows
powershell -ExecutionPolicy Bypass -File .\verify-winget.ps1
```

输出：

```text
winget-verified.json
```

要求：

- 有效 ID 达到 100；
- 失效 ID 补新候选，不要伪造；
- 每个 ID 都用 `winget show --id ... --source winget` 验证；
- 记录 winget 版本。

这一步没过，不要开始做 Windows 目录 UI。

---

## 8. 第三个任务：WingetProvider

Windows 端只支持 **winget**，不支持 msstore/Chocolatey/Scoop。

### 需要实现的能力

```text
探测：winget --version
搜索：winget search --source winget <query>
详情：winget show --id <exact-id> --source winget
已安装：winget list
安装：winget install --id <exact-id> --exact --source winget
卸载：winget uninstall --id <exact-id> --exact --source winget
验证：winget list --id <exact-id> --exact --source winget
```

### 强制安全规则

- 只能传固定参数；
- 不通过 PowerShell/cmd 字符串拼接；
- 不接受前端传入的任意命令；
- 包 ID 必须匹配：
  ```text
  ^[A-Za-z0-9][A-Za-z0-9._-]{0,127}$
  ```
- 只接受 `winget` source；
- 不使用 `--override`；
- 不静默提权；
- 不默认自动接受协议；
- 安装后必须重新 `winget list` 验证，不能只看退出码。

### UAC

winget 安装某些包会弹 UAC。

- DevHub 自己不要请求管理员权限；
- 让 Windows 自己弹；
- 用户拒绝时显示“已取消/权限不足”；
- 不要用隐藏窗口绕过。

### winget 输出解析

先做一个小技术验证：

- 你的 winget 版本是什么；
- `winget search` 输出是表格还是支持 JSON；
- `winget list` 输出格式；
- 中文系统下输出是否会变化。

不要假设输出格式永远固定。  
解析不了时要显示错误，不要伪造“已安装”。

---

## 9. 第四个任务：Windows 目录

把 `winget-verified.json` 转成：

```text
src/catalog.windows.ts
```

字段：

```ts
{
  id: string;
  name: string;
  category: Category;
  description: string;
  package: string;   // winget ID
  kind: "formula" | "cask"; // Windows 侧应改为 "winget" 或 provider 字段
}
```

当前 `kind` 是 Homebrew 概念。Windows 建议改成：

```ts
provider: "homebrew" | "winget";
```

不要为了少改代码而继续叫 formula/cask。

前端还需要：

- Windows 时加载 `catalog.windows.ts`；
- macOS 时加载 `catalog.ts`；
- 不要删除 macOS 目录。

实现方式可以用：

```ts
const IS_WINDOWS = navigator.userAgent.includes("Windows");
```

或构建时选择。不要复制两套 App。

---

## 10. 第五个任务：Windows 检测

macOS 检测路径不能直接用。

Windows 14 款深度检测建议：

- Git：`where.exe git`
- Node：`where.exe node`
- Python：`where.exe python`
- JDK：`where.exe java`
- Go：`where.exe go`
- Maven：`where.exe mvn`
- Gradle：`where.exe gradle`
- Rust：`where.exe rustc`
- MySQL：`where.exe mysql` 或服务查询
- PostgreSQL：`where.exe psql` 或服务查询
- Redis：`where.exe redis-cli` 或服务查询
- VS Code：`%LOCALAPPDATA%\Programs\Microsoft VS Code`
- IntelliJ：`%LOCALAPPDATA%\JetBrains\Toolbox\apps`
- Docker：`where.exe docker` / Docker Desktop

其他精选软件：

- 只查 winget 已安装状态；
- 不为 100 个软件逐个执行昂贵命令；
- 启动时最多执行一次 `winget list`。

数据库/Redis 还要区分：

- 包已安装；
- 服务未启动；
- 命令不在 PATH。

不要把“包安装成功”显示成“服务正在运行”。

---

## 11. 第六个任务：接入现有 UI

保留：

- 圆形网格；
- 蓝色已安装/灰色未安装；
- 搜索/分类；
- 详情卡片动画；
- 单轨道不确定进度；
- 深色/浅色；
- 全局搜索（查询时忽略分类）。

只需要把 provider 从 Homebrew 换成 winget。

不要重做 UI。

---

## 12. 第七个任务：NSIS 打包

仓库已有：

```text
src-tauri/tauri.windows.conf.json
```

配置了：

- x64；
- NSIS；
- currentUser 安装；
- 不显示语言选择器。

构建：

```powershell
npm ci
npm run tauri -- build --bundles nsis
```

预期：

```text
src-tauri\target\release\bundle\nsis\DevHub V2_0.1.0_x64-setup.exe
```

未签名包会触发 SmartScreen，这是预期行为。

不要声称已签名/已 notarize。

---

## 13. Git 操作（不会也要照着做）

### 开始工作

```powershell
git switch windows-handoff
git pull
```

### 修改后

```powershell
git status
git add .
git commit -m "feat: implement winget provider"
git push
```

### 第一次 push 可能需要登录

使用 GitHub 账号 + Personal Access Token。  
不要把 token 发到聊天里。

### 提交 Pull Request

GitHub 页面会提示 **Compare & pull request**：

- base：`main`
- compare：`windows-handoff`
- 写清楚改了什么、测试结果
- 不要直接强推 main

### GitHub Desktop 用户

- 修改后左下角写 Summary；
- Commit to `windows-handoff`；
- Push origin；
- 网页创建 Pull Request。

---

## 14. 测试清单

在 Windows x64 上完成：

1. `npm run test:ui` 通过。
2. `cargo test` 通过。
3. NSIS 构建成功。
4. 干净 Windows 安装。
5. 首页约 100 个软件。
6. 分类正常。
7. 搜索大小写不敏感。
8. 搜索时忽略分类。
9. 精选外软件可搜到。
10. 详情卡片正常。
11. 安装前有确认/UAC 提示。
12. 安装后 `winget list` 验证。
13. 卸载只对 winget 管理的包可用。
14. 系统/官网安装不显示卸载。
15. 中文用户名/空格路径/DPI 正常。
16. 断网、取消 UAC、失败日志正常。
17. 不修改 PATH/Shell，不删除用户数据。

建议测试安装包：`ripgrep`，不要拿数据库做第一次安装测试。

---

## 15. 常见问题

### `link.exe not found`

安装 VS Build Tools，勾选“使用 C++ 的桌面开发”。

### `winget` 不存在

安装/更新 App Installer：从 Microsoft Store 搜索 **App Installer**，或安装 Windows 更新。不要把 msstore source 当成 winget source。

### `npm ci` 失败

确认 Node 版本 20+，删掉 `node_modules` 后重试。

### Tauri 构建失败

先确认：

```powershell
rustup show
rustc -V
```

必须是 `stable-x86_64-pc-windows-msvc`。

### 安装包被 SmartScreen 拦截

未签名，正常。右键 → 属性 → 解除锁定，或“更多信息 → 仍要运行”。

### 安装后仍显示未安装

不要直接改显示逻辑。检查：

- `winget list --id ... --exact`
- 包 ID 是否精确；
- 是否 msstore source；
- 是否安装器还在运行/需要重启；
- winget 输出解析是否失败。

---

## 16. 完成标准 / 交付

完成后给项目所有者：

1. Pull Request 链接；
2. 最终 commit SHA；
3. `DevHub-V2-0.1.0-x64-setup.exe`；
4. 测试清单结果；
5. `winget --version`；
6. 已知问题；
7. 截图（首页、搜索、详情、安装确认）。

**不要**在未确认的情况下对用户真实环境执行安装/卸载测试。  
**不要**修改 PATH、Shell、系统组件。  
**不要**删除用户数据。

---

## 17. 一句话总结

> 你不需要学 DeepSeek Harness；你需要用 Windows 上正常的 Git/Node/Rust 工具，把 DevHub 的 Homebrew 后端替换成 winget 后端，保持现有 UI 不变，最后打出 NSIS 安装包。
