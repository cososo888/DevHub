# DevHub V2 · Windows 版开发报告与交接策划

日期：2026-11-11
状态：**Mac 侧准备完成；Windows 实现由队友在 Windows x64 上继续。**

## 1. 决策

- Windows 10/11 **x64**
- 仅 **winget**
- 不接 msstore / Chocolatey / Scoop
- 不做版本选择
- NSIS 安装包
- 未签名，接受 SmartScreen 提示
- 接受 UAC，不静默提权
- Windows 软件目录与 macOS 不完全相同

## 2. 与 macOS 版的区别

| 层 | macOS | Windows |
| --- | --- | --- |
| 包管理器 | Homebrew | winget |
| 包类型 | formula / cask | winget package ID |
| 安装 | `brew install` | `winget install --id --exact` |
| 卸载 | `brew uninstall` | `winget uninstall --id --exact` |
| 状态 | `brew list` | `winget list` |
| 检测 | PATH + .app + Homebrew | `where.exe` + 注册表 + winget |
| 权限 | 一般无 UAC | 可能 UAC |
| 安装包 | DMG | NSIS `.exe` |
| 签名 | ad-hoc / notarization | Authenticode，当前未签名 |
| 运行时 | WebKit | WebView2 |
| 架构 | arm64 | x64 |

## 3. 底层逻辑问题

1. **不能移植 Homebrew**：Windows 必须新增 WingetProvider，现有后端只能保留 macOS 分支。
2. **前端 `formula/cask` 不是跨平台概念**：应改为 `PackageRef { provider, id, source }`。
3. **winget 输出解析**：表格列宽、语言、版本差异；必须先做 Windows spike，确认 JSON/表格能力。
4. **UAC**：winget 安装可能弹管理员授权；不能静默提权或绕过。
5. **协议**：不能无提示接受所有包协议；确认框要说明。
6. **退出码不等于安装成功**：必须 `winget list --id --exact` 二次验证。
7. **Windows 检测**：不能用 `/usr/bin`、`.app`、Cellar；需要 `where.exe` + 注册表 + App Paths。
8. **100 款目录不能照搬**：Mac 专属软件在 Windows 不存在，必须 Windows 独立目录。
9. **交叉编译**：macOS 不能可靠产出 Windows NSIS；必须在 Windows x64 构建。
10. **未签名**：SmartScreen 会拦截，不是程序崩溃。

## 4. Mac 侧已完成

- macOS 100 款精选目录、8 分类、Homebrew 全库搜索。
- **全局搜索修复**：查询 ≥2 字符时忽略顶部分类，清空后恢复分类。
- 100 个 Windows winget **候选** ID，按 8 分类，尚未验证。
- PowerShell 验证脚本 `docs/windows/verify-winget.ps1`。
- NSIS 配置 `src-tauri/tauri.windows.conf.json`（currentUser，x64 构建时使用）。
- Windows 交接文档 `docs/windows/WINDOWS-HANDOFF.md`。
- Mac 侧测试：UI **19 passed**，Rust **9 passed / 2 ignored**，Homebrew live search passed。
- Mac DMG 已重新构建，包含全局搜索修复。

**注意：Windows 候选 ID 是候选，不是已验证。Mac 没有 winget，不能声称它们全部存在。**

## 5. Windows 队友需要完成

完整入门文档：`docs/WINDOWS-TEAMMATE-GUIDE.md`。

1. 在 Windows x64 安装 Node、Rust MSVC、VS Build Tools、WebView2。
2. 运行：
   ```powershell
   cd docs\windows
   powershell -ExecutionPolicy Bypass -File .\verify-winget.ps1
   ```
3. 有效 ID 达到 100 后，生成 Windows 目录（替换/新增 `catalog.windows.ts`）。
4. 实现 WingetProvider：搜索、已安装列表、安装、卸载、验证。
5. Windows 版 14 款深度检测：`where.exe`、注册表、服务状态。
6. 接入主界面：全局搜索、详情、UAC 确认、日志。
7. 构建：
   ```powershell
   npm ci
   npm run tauri -- build --bundles nsis
   ```
8. 产出：
   ```text
   DevHub-V2-0.1.0-x64-setup.exe
   ```
9. 完成 Windows 手动验收清单（见交接文档）。

## 6. 交接到 GitHub

本机目前没有 GitHub remote，因此不能直接推送。

需要你提供：

- 一个空的 private GitHub 仓库 URL；或
- 你已有的仓库 URL 和推送权限。

之后我会：

1. 把当前分支提交；
2. 创建 `windows-handoff` 分支；
3. 推送分支；
4. 把分支链接交给队友。

当前不推送，不创建远程仓库。

## 7. 已知限制

- Windows 代码尚未实现，不能宣称 Windows 版完成。
- 100 个 winget ID 尚未验证。
- NSIS 配置尚未在 Windows 编译。
- 未签名。
- 无 Windows 真机/VM，本机无法做 Windows UI/UAC 验收。
- winget 版本/输出格式需要先验证。
