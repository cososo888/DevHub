# DevHub V2 · Windows x64 交接说明

日期：2026-11-11
目标：Windows 10/11 x64，winget，NSIS。

## 1. 本仓库在 macOS 侧已完成

- 100 款 macOS 精选目录与 Homebrew 全库搜索；
- 全局搜索修复：输入 ≥2 字符后忽略顶部分类，清空后恢复分类；
- 共享 UI、检测、安装/卸载、日志；
- `docs/windows/winget-candidates.json`：100 个 Windows 候选 ID（**尚未在 Windows 验证**）；
- `docs/windows/verify-winget.ps1`：候选验证脚本；
- `src-tauri/tauri.windows.conf.json`：NSIS 基础配置；
- 本说明。

**Windows 代码尚未实现，不能声称 Windows 版已完成。**
macOS 当前的 Homebrew 后端不能直接在 Windows 使用；Windows 需要新增 provider 层。

## 2. 最终范围

- Windows 10/11 x64
- 仅 winget（不接 msstore、Chocolatey、Scoop）
- 不提供版本选择
- 安装包：NSIS `.exe`
- 未签名，接受 SmartScreen 提示
- UAC 由 Windows/winget 弹出，DevHub 不静默提权

## 3. Windows 构建前置

在 Windows x64 上安装：

1. Git
2. Node.js 20+（含 npm）
3. Rust stable MSVC：
   ```powershell
   rustup default stable-x86_64-pc-windows-msvc
   ```
4. Visual Studio Build Tools，勾选“使用 C++ 的桌面开发”
5. WebView2 Runtime（Windows 11 通常自带；Tauri NSIS 也可 bootstrapper）
6. 确认：
   ```powershell
   winget --version
   rustc -V
   node -v
   ```

## 4. 第一步：验证 winget 候选

```powershell
cd docs\windows
powershell -ExecutionPolicy Bypass -File .\verify-winget.ps1
```

输出：

```text
winget-verified.json
```

要求：

- 有效项达到 100；
- 每个 ID 都必须由 `winget show --id ... --source winget` 成功验证；
- 候选失效时补充，不要伪造；
- 通过后把 `winget-verified.json` 作为 Windows 目录数据源。

注意：`winget show` 的退出码/输出格式取决于 winget 版本；先记录版本再验证。

## 5. 需要实现的 Windows 适配层

当前 macOS 后端 `src-tauri/src/homebrew.rs` 不能直接用于 Windows。

建议新增：

```text
src-tauri/src/provider/
├── mod.rs          PackageProvider trait + PackageRef
├── homebrew.rs     macOS（可逐步迁移现有实现）
└── winget.rs       Windows
```

统一能力：

```rust
trait PackageProvider {
    fn available(&self) -> Result<bool, String>;
    fn installed(&self) -> Result<Vec<PackageState>, String>;
    fn search(&self, query: &str) -> Result<Vec<SearchResult>, String>;
    fn install(&self, package: &PackageRef) -> Result<String, String>;
    fn uninstall(&self, package: &PackageRef) -> Result<String, String>;
    fn verify_installed(&self, package: &PackageRef) -> Result<bool, String>;
}
```

前端只传 `PackageRef { provider, id, source }`，不传命令。

### winget 命令草案

先探测：

```text
winget --version
```

搜索：

```text
winget search --query <q> --source winget --accept-source-agreements
winget show --id <id> --source winget --accept-source-agreements
```

安装：

```text
winget install --id <id> --exact --source winget --accept-package-agreements --accept-source-agreements
```

卸载：

```text
winget uninstall --id <id> --exact --source winget
```

验证：

```text
winget list --id <id> --exact --source winget
```

安全要求：

- 只使用固定 argv，不拼 PowerShell/cmd 字符串；
- ID 必须匹配 `^[A-Za-z0-9][A-Za-z0-9._-]{0,127}$`；
- 只接受 `winget` source 的有效 ID；
- 不传 `--override`、不执行任意脚本；
- 安装/卸载前检查包状态；
- 安装后必须 `winget list` 验证；
- 失败时保留短错误 + 完整日志。

### UAC

- DevHub 本身不请求管理员权限；
- winget 安装器可能触发 UAC；
- 用户拒绝/取消 UAC 时显示“已取消/权限不足”，不重试提权；
- 不静默接受所有包协议；确认框需说明会接受所选包的协议。

## 6. 检测

Windows 不能使用 macOS 的 `/usr/bin`、`.app`、Homebrew 路径。

- 启动/刷新只执行一次 `winget list`，不要逐包 expensive 调用；
- 14 款深度检测使用：
  - `where.exe`
  - 注册表 App Paths / Uninstall
  - 常见目录
  - 服务状态（数据库/Redis）
- 其他精选 100 用 winget 状态；
- 显示：
  - 名称
  - 已安装版本
  - 实际路径/来源
  - Homebrew/winget 管理状态
- 不因不是 DevHub 安装而显示未安装。

## 7. UI

保留现有：

- 圆形网格、蓝/灰状态
- 搜索/分类
- 卡片展开动画
- 不确定进度
- 深色/浅色
- 极简详情

搜索要求：

- 输入 ≥2 字符后 **全局搜索，忽略分类**；
- 清空后恢复分类；
- Windows 全库搜索使用 winget；
- 无图标默认使用名称缩写。

## 8. NSIS

本仓库已有 `src-tauri/tauri.windows.conf.json`：

```json
{
  "bundle": {
    "targets": ["nsis"],
    "windows": {
      "nsis": {
        "installMode": "currentUser",
        "displayLanguageSelector": false
      }
    }
  }
}
```

构建：

```powershell
npm ci
npm run tauri -- build --bundles nsis
```

预期产物：

```text
src-tauri\target\release\bundle\nsis\DevHub V2_0.1.0_x64-setup.exe
```

未签名会有 SmartScreen；这是预期，不是构建失败。

## 9. Windows 手动验收清单

1. 干净 Windows 10/11 x64（无 DevHub 数据）安装 NSIS。
2. 首页显示约 100 款 Windows 软件。
3. 分类切换正常。
4. 搜索 `postman` / `git` / 一个精选外软件。
5. 搜索时切换分类，结果范围不变；清空恢复分类。
6. 打开详情卡片，确认名称、简介、状态、路径。
7. 安装一个小包（如 ripgrep），确认 UAC/协议提示。
8. 安装后 `winget list`、应用蓝色状态、版本/path。
9. 卸载该 winget 包，确认提示和结果；系统/官网安装不可卸载。
10. 失败场景：断网、取消 UAC、失败日志。
11. 中文用户名、空格路径、125%/150% DPI。
12. 不修改 PATH/Shell；不删除用户数据。

## 10. 已知风险

1. winget 版本差异、输出本地化/表格解析。
2. winget 候选 ID 失效；必须由 Windows 验证。
3. 部分安装器需要交互/重启；不能假设退出码 0 等于可用。
4. msstore source 不在范围内。
5. 未签名包会触发 SmartScreen。
6. Windows 代码尚未实现；macOS 后端不能直接跨平台复用。
7. 无 Windows 真机/VM 时不能完成 UI/UAC/NSIS 验收。
