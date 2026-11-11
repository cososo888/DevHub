# DevHub V2 · 100 软件目录与 Homebrew 全库搜索报告

日期：2026-11-11
版本：0.1.0（Apple Silicon / aarch64）
安装包：`dist/DevHub-V2-0.1.0-aarch64.dmg`

## 1. 实际收录

- **100 款精选软件**
- **84 个 formula + 16 个 cask**
- 100 个包名逐个用本机 `brew info --json=v2 --formula/--cask` 验证；缺失 0 个。
- 首页约 100 个圆形图标，8 个分类。

| 分类 | 数量 |
| --- | ---: |
| 语言与运行时 | 20 |
| 开发工具 | 20 |
| 终端工具 | 15 |
| 数据库与服务 | 9 |
| 网络与云 | 13 |
| 容器与云 | 10 |
| 效率工具 | 9 |
| 设计媒体 | 4 |
| **合计** | **100** |

## 2. Homebrew 全库搜索

实现方式：

1. 用户输入 ≥ 2 字符后，前端 **320 ms 防抖**。
2. 前端按 query 缓存本次会话搜索结果。
3. 后端执行固定 argv：
   - `brew search --formula <query>`
   - `brew search --cask <query>`
4. 每个类型最多取 12 个结果，再用一次批量：
   - `brew info --json=v2 --formula ...`
   - `brew info --json=v2 --cask ...`
5. 解析名称、包名、类型、简介、版本。
6. 后端内存缓存 5 分钟；结果按 `kind:package` 去重，最多 24 条。
7. 不启动时下载全库，不逐包执行昂贵检测。

搜索界面：

- 搜索中显示“正在搜索 Homebrew 全库…”
- 有结果时显示“精选 N 款 · Homebrew 全库 N 款”
- 无结果显示空状态
- 失败显示短错误 + 重试
- 搜索到的软件进入同一圆形网格和同一详情卡片
- 无官方图标时使用简洁默认图标（名称缩写），不会因缺图崩溃

## 3. 安装状态与通用安装

### 包状态

启动/刷新时执行两次状态查询：

```text
brew list --formula --versions
brew list --cask --versions
```

得到包名、版本、Homebrew 路径（`/opt/.../opt/<package>` 或 `Caskroom/<package>`）。

- 原有 14 款继续使用深度检测（PATH、实际路径、多实例）。
- 其他精选/搜索软件使用 Homebrew 包状态，不写专用检测器。
- Homebrew 状态与 DevHub 安装记录独立。

### 安装

统一命令，不做版本选择：

```text
brew install <package>
brew install --cask <package>
```

- 前端只提交 `package + kind`。
- 后端校验包标识字符集、用 `brew info` 验证包存在、检查是否已安装。
- 使用安全 argv 调用，不经过 Shell 字符串拼接。
- 命令结束不等于成功：再用 `brew list` 确认包存在，并刷新检测。
- 安装中沿用圆环和单轨道不确定进度。
- 失败显示短原因 + 可展开技术日志。

### 卸载

- 只有 Homebrew 当前安装的包才提供卸载。
- 官网/系统/手动/fnm/uv/rustup 安装只展示，不直接删除。
- 确认框显示 `brew uninstall` 命令；只移除 Homebrew 管理的文件。
- 系统或其他来源的同名安装不会被删除。

## 4. 真实运行验证

- 真实 release app 从 DMG 启动成功。
- 首页显示 100 款精选软件、8 个分类。
- 全局搜索：输入查询后忽略顶部分类；清空后恢复分类。
- 搜索 `postman`：精选 0 款、Homebrew 全库 5 款；结果包含 Postman、Postman CLI 等。
- 点击 Postman 打开统一详情卡片。
- 点击安装显示确认命令：
  `brew install --cask postman`
- 已取消确认，**没有执行真实安装**。
- 未执行真实卸载。

截图：

- `docs/screenshots/phase3-catalog-100.png`
- `docs/screenshots/phase3-homebrew-search.png`
- `docs/screenshots/phase3-search-detail.png`
- `docs/screenshots/phase3-search-confirm.png`
- `docs/screenshots/phase3-release.png`

## 5. 测试与构建

| 项目 | 结果 |
| --- | --- |
| 100 包名 `brew info` 验证 | 100/100 |
| `npx tsc --noEmit` | 通过 |
| `npm run test:ui` | **18 passed / 0 failed** |
| `cargo test --offline --lib` | **9 passed / 0 failed / 1 ignored** |
| 真实 Homebrew 搜索测试 | 通过（formula/cask、简介、路径） |
| release DMG | **3.2 MB**，aarch64 |
| DMG 校验 | VALID |
| release app 启动 | 通过 |

DMG SHA-256：

```text
aa7990bc6828fae52c0d40de7eb73f1059bd989b7dc995a5847eb5d5bd511e53
```

## 6. 安装包

```text
dist/DevHub-V2-0.1.0-aarch64.dmg
dist/SHA256SUMS.txt
```

安装：打开 DMG，将 `DevHub V2.app` 拖入 Applications。

**签名说明**：本包为本地 ad-hoc 签名、未 notarize。首次打开若被 Gatekeeper 拦截，请右键应用选择“打开”，或在“系统设置 → 隐私与安全性”允许。没有 Apple Developer ID，不能声称已通过 Gatekeeper/notarization。

## 7. 修改的核心文件

- `src/catalog.ts`：100 款真实软件目录、8 分类、包名/类型
- `src/App.tsx`：包状态、320ms 搜索防抖、搜索结果合并、详情接线
- `src/components/DetailOverlay.tsx`：通用包详情、状态/路径、重复安装提示、日志
- `src/hooks/useInstallTask.ts`：通用 `package + kind` 安装/卸载
- `src/lib/ipc.ts`：包状态、搜索、安装事件类型
- `src-tauri/src/homebrew.rs`：包状态、搜索、固定 argv 安装/卸载
- `src-tauri/src/lib.rs`：Tauri 命令与验证
- `scripts/*.test.mjs`：100 目录与 UI 测试
- `docs/PHASE3-CATALOG-REPORT.md`、截图、`dist/` DMG
- `docs/windows/`：Windows 交接资料；`docs/WINDOWS-PORT-REPORT.md`

## 8. 尚未解决

1. 未执行真实 Homebrew 安装/卸载（需要用户确认）。
2. 搜索只展示每类最多 12、总计 24 条，不提供全库分页。
3. 通用检测只覆盖 Homebrew 包状态；非 Homebrew 的官网 app 对精选 100 不作专用识别。
4. 数据库/Redis 安装不自动启动服务。
5. DMG 未签名/未 notarize。
6. 首次缺少 Homebrew 时只引导打开 brew.sh，不静默提权自动安装。
7. 旧多版本适配器源码仍保留但未编译/未使用。
