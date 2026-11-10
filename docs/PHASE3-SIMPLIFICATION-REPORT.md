# DevHub V2 · 产品定位调整与一次性架构精简

日期：2026-11-11
状态：**版本选择器、远程版本目录和 DevHub 自选版本能力已移除；改为固定 Homebrew 命令。**

## 1. 产品收敛

DevHub 现在是：

- 图形化终端操作入口；
- 本机开发环境查看器。

安装命令完全由预定义 Homebrew 包决定，用户不再选择具体版本。例如：

```text
Node.js → brew install node
Python  → brew install python
Git     → brew install git
Go      → brew install go
Maven   → brew install maven
Gradle  → brew install gradle
Redis   → brew install redis
```

不维护可安装版本列表，不自行决定 LTS/最新版。

## 2. 界面

保留：

- 圆形图标网格、蓝色/灰色状态；
- 搜索、分类；
- 展开卡片动画；
- 安装圆环、单轨道不确定进度；
- 深色/浅色主题；
- 极简详情。

详情卡片现在仅有：

1. 图标、名称、一句话介绍；
2. 「已安装 / 未安装」；
3. 本机实例的版本和实际路径；
4. 安装或卸载按钮；
5. 必要时显示固定命令、进度、错误/技术日志。

**已彻底移除可安装版本列表、版本选择、历史目录和版本缓存 UI。**

## 3. 14 个软件的固定安装计划

所有命令均通过只读 `brew info` 验证包存在；Homebrew 缺失时不显示可用安装。

| 软件 | 包 | 类型 | 安装命令 | 当前本机 |
| --- | --- | --- | --- | --- |
| Python | python | formula | `brew install python` | 已安装（系统/Anaconda/uv） |
| Node.js | node | formula | `brew install node` | 已安装（系统 26.13 + fnm 22.14） |
| Git | git | formula | `brew install git` | 已安装（系统 2.50.1） |
| VS Code | visual-studio-code | cask | `brew install --cask visual-studio-code` | 已安装（官方 app） |
| JDK | openjdk | formula | `brew install openjdk` | 已安装（Oracle/Microsoft/JBR） |
| Go | go | formula | `brew install go` | 未安装，显示安装 |
| Docker | docker | cask | `brew install --cask docker` | 未安装，显示安装 |
| MySQL | mysql | formula | `brew install mysql` | 已安装（官方） |
| PostgreSQL | postgresql@17 | formula | `brew install postgresql@17` | 未安装，显示安装 |
| Maven | maven | formula | `brew install maven` | 已安装（手动） |
| Gradle | gradle | formula | `brew install gradle` | 未安装，显示安装 |
| Redis | redis | formula | `brew install redis` | 未安装，显示安装 |
| Rust | rust | formula | `brew install rust` | 已安装（rustup） |
| IntelliJ IDEA | intellij-idea | cask | `brew install --cask intellij-idea` | 已安装（官方 app） |

数据库/Redis 安装只安装软件包，**不会自动启动服务**；安装结果会提示“服务尚未启动”。

## 4. 本机检测结果

当前检测到 **19 个实例**：

| 软件 | 版本/路径 |
| --- | --- |
| Git | 2.50.1，`/usr/bin/git` |
| IntelliJ IDEA | 2026.1，`/Applications/IntelliJ IDEA.app` |
| JDK | 26（两份）、21.0.13.0.2、17.0.14、17.0.2 |
| Maven | 3.9.16，`/usr/local/maven/bin/mvn` |
| MySQL | 26.7.0，`/usr/local/mysql/bin/mysql` |
| Node.js | 26.13.0（系统）、26.13.0（V1 只读）、22.14.0（fnm） |
| Python | 3.9.6（系统）、3.14.6（官方/Conda）、3.12.15/3.11.18（uv） |
| Rust | 1.99.1，`~/.cargo/bin/rustc` |
| VS Code | 1.144.1，`/Applications/Visual Studio Code.app` |

本机检测不依赖 DevHub 安装记录。fnm、uv、V1、系统安装均正常显示；V1 只读且不提供卸载。

## 5. 卸载

- 只有检测到 **Homebrew 管理**的实例，才显示卸载。
- 卸载前显示确认框和固定 `brew uninstall` 命令。
- 不删除用户数据、项目、数据库文件或系统组件。
- 官网/系统/手动/fnm/uv/rustup 安装：只展示，不显示卸载。
- 后端再次检查 Homebrew 包存在、实例归属和卸载后结果。
- 不按检测到的可执行文件直接删除。

当前本机没有 Homebrew 管理的 14 软件实例，因此真实界面中无卸载按钮；Homebrew 外部安装路径已由后端校验。

## 6. 首次使用

- 首次启动通过现有检测检查 Homebrew。
- Homebrew 缺失时显示轻量提示：
  “安装功能需要 Homebrew。DevHub 不会静默获取管理员权限。”
- 提供“查看安装方法”（打开 brew.sh）和“重新检查”。
- 不静默提权，不自动安装 Homebrew；只准备真正必要的 Homebrew 组件。
- Homebrew 已存在时不重复打扰用户。

## 7. 移除/停用的复杂能力

- 删除前端 `useSoftwareVersions`、可安装版本类型和版本列表。
- 删除后端 `get_software_versions` 命令、旧 Homebrew 版本目录 API、版本缓存读写代码。
- 停用并移出编译：`fnm.rs`、`uv.rs`、`jdk.rs`、`rustup.rs`、`versions.rs` 的多版本适配代码。
- 保留旧数据库表 `install_ownership`、`uninstall_history`；不删除用户数据。
- 保留检测、共享卡片动画、进度动画、主题、固定 Homebrew 安装/卸载、技术日志。
- **未卸载/覆盖** 用户已有的 fnm、uv、Node 22.14、Python 3.12/3.11、系统软件；未修改 PATH/Shell/fnm default alias。

## 8. 测试与构建

- `npx tsc --noEmit`：通过
- `npm run test:ui`：**17 passed / 0 failed**
- `cargo test --offline --lib`：**9 passed / 0 failed / 1 ignored**
- 真实 macOS app：**31.26 MiB**，已启动
- 已展示：
  - 主界面；
  - Go 未安装卡片；
  - `brew install go` 确认框（未执行）；
  - Git 已安装、路径、无卸载按钮。
- **本轮未执行真实安装或卸载**；Go 确认框已取消。

## 9. 未完成/限制

1. Homebrew 缺失时只提供官方安装页面，不自动安装；管理员权限不在应用中静默获取。
2. 当前本机无 Homebrew 管理的 14 软件，因此真实卸载按钮未在硬件上触发；UI 和固定命令测试通过。
3. Docker/VS Code/IntelliJ 的 cask 安装/卸载依赖其 app 检测；本轮未真实执行。
4. MySQL/PostgreSQL/Redis 安装不自动启动服务。
5. Homebrew 安装速度受网络影响；安装期间只显示持续动画。
6. 旧适配器文件仍在仓库但已停用、不参与编译，作为回退参考；不删除用户环境。

## 10. 截图

- `docs/screenshots/phase3-simple-main.png`
- `docs/screenshots/phase3-simple-go.png`
- `docs/screenshots/phase3-simple-go-confirm.png`
- `docs/screenshots/phase3-simple-git.png`
