<p align="center">
  <img src="docs/images/main-grid.png" alt="DevHub V2 软件网格" width="860" />
</p>

<h1 align="center">DevHub V2</h1>

<p align="center">
  面向 macOS 的图形化开发环境管理器：安装、检测、卸载，都在一个极简圆形网格里完成。
</p>

<p align="center">
  <a href="https://github.com/cososo888/DevHub"><img alt="GitHub" src="https://img.shields.io/badge/GitHub-cososo888%2FDevHub-181717?logo=github&logoColor=white" /></a>
  <a href="https://github.com/cososo888/DevHub/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/cososo888/DevHub/actions/workflows/ci.yml/badge.svg" /></a>
  <a href="LICENSE"><img alt="License: MIT" src="https://img.shields.io/badge/license-MIT-2EA44F" /></a>
  <img alt="macOS" src="https://img.shields.io/badge/platform-macOS%20Apple%20Silicon-000000?logo=apple&logoColor=white" />
  <img alt="Tauri" src="https://img.shields.io/badge/Tauri-2-24C8DB?logo=tauri&logoColor=white" />
  <img alt="React" src="https://img.shields.io/badge/React-19-61DAFB?logo=react&logoColor=111111" />
  <img alt="Rust" src="https://img.shields.io/badge/Rust-stable-CE422B?logo=rust&logoColor=white" />
  <img alt="TypeScript" src="https://img.shields.io/badge/TypeScript-6-3178C6?logo=typescript&logoColor=white" />
</p>

<p align="center"><a href="https://github.com/cososo888/DevHub/releases/download/v0.2.0/DevHub-V2-0.2.0-macOS-arm64.dmg"><strong>下载 macOS 安装包 · Apple Silicon</strong></a> · <a href="https://github.com/cososo888/DevHub/releases/tag/v0.2.0">发布说明及校验文件</a> · <a href="README_EN.md">English</a></p>

---

## 简介

DevHub V2 是一个 **Tauri 2 + React + Rust** 的桌面应用，用来查看和管理本机开发环境。

它不维护自己的安装器，也不提供版本选择器。点击安装时，DevHub 只执行预先定义、经过审核的 **Homebrew** 命令；具体安装版本由 Homebrew 决定。

> **当前仅开发和维护 macOS 版，优先修复安装可靠性与界面体验。Windows 开发已暂停；原交接资料仅保留作历史参考。**

核心原则：

- 不替代包管理器；
- 不伪造安装状态；
- 不静默获取管理员权限；
- 不删除用户数据或系统组件；
- 检测独立于 DevHub 的安装记录。

## 核心功能

### 精选软件库

- **100 款真实 Homebrew 软件**
- **84 个 formula + 16 个 cask**
- 8 个分类：
  - 语言与运行时
  - 开发工具
  - 终端工具
  - 数据库与服务
  - 网络与云
  - 容器与云
  - 效率工具
  - 设计媒体

所有包名均经过 Homebrew 验证；没有虚构包名。

### 搜索

- 点击顶部放大镜进入独立搜索页，返回时保留软件库的分类、安装筛选和滚动位置；
- 精选库与 Homebrew 更多结果分组展示，关键词和结果在页面切换间保留；
- **Homebrew 全库搜索**：`brew search` + 批量 `brew info`；
- 查询 **大小写不敏感**；
- 搜索页覆盖全部分类，不受软件库筛选影响；
- 250 ms 防抖、前端会话缓存、后端 5 分钟缓存；
- 每类最多 12 条，合并去重后最多 24 条；
- 加载、无结果、错误重试状态；
- 搜索结果使用同一个圆形详情卡片。

### 本机环境检测

- 原有 14 款软件使用深度检测：
  - 版本
  - 实际路径
  - 多实例
  - PATH / 安装来源
- 其他精选/搜索软件通过 Homebrew 包状态通用检测；
- 批量读取软件包状态，保留已检测到的版本和路径；
- 系统、官网、Homebrew、fnm、uv、rustup 等已有安装都会展示；
- 检测范围覆盖常见路径和指定安装来源，无法确认时会标记状态。

### 安装与卸载

- 不提供版本选择；
- 固定命令：
  ```text
  brew install --formula <package>
  brew install --cask <package>
  brew uninstall --formula <package>
  brew uninstall --cask <package>
  ```
- 安装前二次确认；
- 固定 argv 调用，不拼 Shell 字符串；
- 安装后通过 `brew list --versions` 和精确 formula JSON 二次验证；
- 查询失败不会伪装成“未安装”；
- 只允许卸载 Homebrew 当前管理的包；
- 系统/官网/手动安装不会被直接删除；
- 数据库/Redis 安装不会自动启动服务。

### 界面

- 圆形软件图标网格；
- 已安装软件显示柔和蓝色光晕；
- 点击图标展开共享 morph 卡片；
- 安装圆环和单轨道不确定进度；
- 深色 / 浅色 / 自动主题；
- 关闭窗口后保留在程序坞，点击程序坞恢复，右键退出或 ⌘Q 结束应用；
- 安装核验成功后发送原生系统通知，并保留应用内完成提醒；
- 左栏「使用指南」中的「安装提醒设置」可开启通知、查看权限和发送测试通知；
- 隐藏页面滚动条，保留滚轮/触控板/键盘滚动；
- 本地图标保留品牌颜色，未知软件使用通用图标。

## 界面展示

<p align="center">
  <img src="docs/images/main-grid.png" alt="100 款精选软件与分类" width="860" />
</p>

<p align="center"><sub>100 款精选软件与 8 个分类</sub></p>

| Homebrew 全库搜索 | 搜索结果详情 |
| --- | --- |
| <img src="docs/images/homebrew-search.png" alt="Homebrew 全库搜索" width="420" /> | <img src="docs/images/search-detail.png" alt="搜索结果详情" width="420" /> |
| 搜索 `postman`，结果来自 Homebrew 全库 | 搜索结果进入统一详情卡片 |

<p align="center">
  <img src="docs/images/install-confirm.png" alt="安装确认" width="640" />
</p>

<p align="center"><sub>安装前显示固定 Homebrew 命令，确认前不会执行</sub></p>

## 技术栈

| 层 | 技术 |
| --- | --- |
| 桌面框架 | Tauri 2 |
| 后端 | Rust 2024 |
| 前端 | React 19 + TypeScript 6 |
| 构建 | Vite 8 |
| 数据 | SQLite（rusqlite） |
| 包管理 | Homebrew CLI |
| 测试 | Node test runner + Testing Library + Rust test |

## 安装

### 直接使用 macOS 安装包

适用于搭载 M 系列芯片的 Mac，应用部署目标为 macOS 11 及以上。下载 DMG 后即可安装。校验文件与安装包同在发布页。[下载 v0.2.0 安装包](https://github.com/cososo888/DevHub/releases/download/v0.2.0/DevHub-V2-0.2.0-macOS-arm64.dmg)

1. 打开 DMG，将 `DevHub V2.app` 拖到「应用程序」，再从那里打开。当前构建仅使用本地签名，尚未经过 Apple 公证，首次打开可能被 macOS 拦截。[Apple 的应用安全说明](https://support.apple.com/zh-cn/102445)
2. 打开后即可浏览软件、搜索精选目录并查看检测到的本机环境。使用打包好的应用无需安装 Node.js 或 Rust。
3. 安装、卸载和搜索 Homebrew 全库需要 Homebrew。DevHub 在常见位置未检测到它时会展示「使用指南」。按指南打开官网，核对系统要求，安装官方 `.pkg` 后返回 DevHub 点「重新检测」。指南也可从左栏随时打开。[Homebrew 官网](https://brew.sh/)

### 从源码运行

前置条件：

- macOS（Apple Silicon）
- Node.js 24.15+ (24.x) / 26+
- Rust stable
- Xcode Command Line Tools
- Homebrew（安装/卸载功能需要）

```bash
git clone https://github.com/cososo888/DevHub.git
cd DevHub
npm ci
npm run tauri -- dev
```

### 构建 macOS 安装包

```bash
npm run package:macos
```

产物：

```text
src-tauri/target/release/distribution/0.2.0/
```

## 开发

### 检查与测试

```bash
npm run check
npm run test:ui
cargo test --manifest-path src-tauri/Cargo.toml
```

### 常用命令

| 命令 | 作用 |
| --- | --- |
| `npm run dev` | 只启动 Vite 前端 |
| `npm run tauri -- dev` | 启动完整 Tauri 应用 |
| `npm run build` | 构建前端静态文件 |
| `npm run package:macos` | 构建 DMG |
| `npm run test:ui` | UI/组件测试 |
| `npm run check` | TypeScript 类型检查 |
| `cargo test --manifest-path src-tauri/Cargo.toml` | Rust 测试 |

## 项目结构

```text
.
├── src/                          React 前端
│   ├── App.tsx                   首页、全局搜索、详情接线
│   ├── catalog.ts                100 款 macOS 精选目录
│   ├── components/               圆形网格、详情卡片、Logo、工具栏
│   ├── hooks/                    检测、安装任务、主题、减弱动画
│   └── lib/                      IPC 类型、搜索过滤
├── src-tauri/                    Rust/Tauri 后端
│   ├── src/lib.rs                Tauri 命令与任务
│   ├── src/homebrew.rs           Homebrew 适配、状态、搜索、安装/卸载
│   ├── src/detection.rs          本机环境检测
│   ├── src/process.rs            有界、固定参数的进程执行器
│   ├── src/storage.rs            SQLite 缓存与卸载历史
│   ├── tauri.conf.json           macOS 应用配置
│   └── tauri.windows.conf.json   Windows NSIS 规划配置
├── docs/                         报告、截图、Windows 交接资料
├── scripts/                      UI 与目录测试
├── dist/                         Vite 生成的前端资源
└── package.json
```

## 版本记录

本项目当前维护 macOS Apple Silicon 版。历次更新见[变更日志](CHANGELOG.md)。历史 Windows 交接资料保存在 [docs/windows](docs/windows)。

## 已知限制

- 真实安装/卸载需要用户在图形界面确认；自动化测试不执行真实写操作。
- macOS DMG 未 notarize。
- 搜索最多展示 24 条结果，没有全库分页。
- 非 Homebrew 官网 app 不在 100 款通用检测范围内；原有 14 款保留深度检测。
- 数据库/Redis 安装不会自动启动服务。
- 首次缺少 Homebrew 时只提供安装引导，不静默提权。
- Windows 版尚未实现。

## 安全原则

- 固定命令和参数，不拼接用户输入；
- 包 ID 字符集校验；
- 安装/卸载前确认；
- 不静默获取管理员权限；
- 不删除用户数据、项目、数据库文件或系统组件；
- 只卸载明确由 Homebrew 管理的包；
- 查询失败与“未安装”严格区分。

## 贡献

欢迎提交 Issue 和 Pull Request。

- 功能/修复请从 `main` 创建分支；
- Windows 相关工作请基于 `windows-handoff`；
- 提交前运行 `npm run check`、`npm run test:ui` 和 Rust 测试；
- 不要提交真实安装/卸载后的本机状态数据；
- 不要修改旧版 DevHub 仓库；
- 未完成的 Windows 功能不要写成已完成。

## 文档

- [English README](README_EN.md)
- [开发指南](docs/development.md)
- [贡献指南](CONTRIBUTING.md)
- [安全策略](SECURITY.md)
- [变更日志](CHANGELOG.md)
- [Phase 3 目录与搜索报告](docs/PHASE3-CATALOG-REPORT.md)
- [Go 安装验证误报修复](docs/GO-VERIFICATION-FIX.md)
- [Windows 版差异与底层问题](docs/WINDOWS-PORT-REPORT.md)
- [Windows 队友入门指南](docs/WINDOWS-TEAMMATE-GUIDE.md)
- [Windows 构建交接清单](docs/windows/WINDOWS-HANDOFF.md)

## 状态

- 版本：`0.2.0`
- 当前平台：macOS Apple Silicon
- Windows：已暂停
- 许可证：[MIT](LICENSE)

---

<p align="center"><sub>DevHub V2 · 图形化终端操作与环境查看</sub></p>
