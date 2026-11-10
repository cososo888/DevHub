# DevHub V2 · JDK 与外部安装卸载修复报告

日期：2026-11-10
状态：**JDK 26/21/17 用户自有安装已有安全卸载入口；系统/JBR 安装明确阻止；未进入第四阶段。**

## 1. 为什么之前 JDK 没有卸载按钮

实际原因不是简单的“前端没显示”：

1. `VersionRow.canUninstall` 之前只由 **Homebrew formula 已安装** 或 **DevHub 归属记录** 推导。
2. JDK 的实际来源是官方/手动安装包，不是 Homebrew formula，也没有 DevHub receipt，因此 `canUninstall=false`。
3. 检测模块只记录 `.../Contents/Home/bin/java` 可执行文件路径，没有记录：
   - JDK bundle 根目录；
   - 发行版（Oracle / Microsoft / OpenJDK / JetBrains）；
   - bundle 是否完整、是否用户所有、是否有安全卸载方式。
4. 版本列表把同一版本的多个实例合并成一行，但没有**实例级**卸载能力字段，前端无法选择具体 JDK。
5. 没有 JDK 卸载适配器；上一轮只增加了 Homebrew 外部包卸载，因此 JDK 仍被排除。

## 2. 修复后的 JDK 识别

新增 `src-tauri/src/jdk.rs`：

- 从 `.../Contents/Home/bin/java` 反推 bundle 根目录。
- 仅允许：
  - `/Library/Java/JavaVirtualMachines/<bundle>`
  - `~/Library/Java/JavaVirtualMachines/<bundle>`
  - `~/<name>.jdk`
- 校验 bundle 完整性：`Contents/Info.plist`、`Contents/Home/bin/java`、`Contents/Home/lib`。
- 读取 `CFBundleIdentifier`、`CFBundleName`、`JavaVM:JVMVendor`。
- 阻止：
  - `JetBrains Runtime` / JBR（可能被 IntelliJ 使用）；
  - root/系统所有（需要管理员权限）；
  - 结构不完整、未知发行版、未知路径。

安全卸载方式：

```
将完整 JDK bundle 移动到 ~/.Trash/DevHub-JDK-<time>-<name>
```

**不是 `rm -rf`**，不删除共享库，不修改 PATH/Shell；用户可以从废纸篓恢复。

## 3. 本机 JDK 实际能力表

| JDK 版本 | 实际 bundle | 发行版/来源 | 可否卸载 | 机制 / 原因 |
| --- | --- | --- | --- | --- |
| 26 | `~/Library/Java/JavaVirtualMachines/openjdk-26` | Oracle OpenJDK，用户所有 | **可以** | 完整 bundle → 废纸篓 |
| 26 | `/Library/Java/JavaVirtualMachines/jdk-26.jdk` | Oracle，root 所有 | 否 | 系统/管理员权限 |
| 21.0.13.0.2 | `~/Library/Java/JavaVirtualMachines/ms-21.0.13.0.2` | Microsoft OpenJDK，用户所有 | **可以** | 完整 bundle → 废纸篓 |
| 17.0.19 | `~/Library/Java/JavaVirtualMachines/ms-17.0.19` | Microsoft OpenJDK，用户所有 | **可以** | 完整 bundle → 废纸篓 |
| 17.0.14 | `~/Library/Java/JavaVirtualMachines/jbr-17.0.14` | JetBrains Runtime | 否 | 可能被 IntelliJ IDEA 使用 |
| 17.0.2 | `~/jdk-17.0.2.jdk` | Oracle JDK，用户所有 | **可以** | 完整 bundle → 废纸篓；当前 PATH 使用此 JDK，确认框提示风险 |

说明：

- **26 行**包含两个实例；前端只允许选择可安全卸载的 `openjdk-26`，不会动 `/Library` 的 root JDK。
- 21、17.0.19、17.0.2 行显示「卸载」。
- 17.0.14 显示 `?`，点击显示“可能被 IntelliJ IDEA 使用”的原因。
- 相同版本多个实例时，卸载确认可选择一个具体实例；只有可安全卸载的实例出现在选择器里。

## 4. 卸载确认与安全

- 确认框显示实际 bundle 路径。
- 对于非 Homebrew 的 JDK，提示“若当前 PATH 指向此 JDK，卸载后相关命令会失效”。
- 后端重新验证：实例仍在检测结果、bundle 完整、用户所有、非 JBR、非系统、非 Homebrew。
- 通过 `rename` 移到废纸篓；失败不会回退为删除。
- 卸载后重新检测；其他 JDK 保留。
- 不做 `rm -rf`、不执行 `brew cleanup`、不修改旧版 DevHub。

当前机器**未执行真实 JDK 卸载**；上面的“可以”是安全分析结果，需由你在界面确认后执行。

## 5. 其他软件卸载覆盖

| 软件/实例 | 当前状态 | 原因 |
| --- | --- | --- |
| Homebrew formula（Git/Go/Maven/Gradle/Node/Python/JDK/Rust） | 支持（若已安装、实例和依赖通过） | `brew uninstall` |
| 系统 Git `/usr/bin/git` | 不支持 | 系统保护 |
| Node `/usr/local/bin/node` | 不支持 | 未识别为 Homebrew/版本管理器 |
| Python 系统/Anaconda/Framework | 不支持 | 系统/Conda/官方安装，未接入对应卸载 |
| Maven `/usr/local/maven` | 不支持 | 未识别为 Homebrew |
| Rust rustup | 不支持 | 未接入 rustup 卸载 |
| MySQL `/usr/local/mysql` | 不支持 | 官方安装 |
| VS Code / IntelliJ | 不支持 | app bundle，不做文件级删除 |
| V1 受管安装 | 不支持 | 严格只读 |
| JDK 用户自有（上表） | 支持 | JDK bundle → 废纸篓 |
| JDK root / JBR | 不支持 | 管理员权限 / IDE 共享 |

**当前本机没有 Homebrew 管理的开发工具**，所以 Homebrew 外部卸载仍没有真实对象可测；JDK 是当前主要新增的真实可卸载类别。

## 6. 进度条与主界面（同批修复）

- 不确定进度条改为单轨道伪元素 `left: -40% → 100%`，不再用子元素 transform；与卡片宽度/共享元素动画分离。
- 连续 5 次关闭/打开：单节点、无子元素、`animationName=indeterminate-sweep`；切换软件后仍正常。
- 删除主界面「选择你的开发工具」及副标题；保留检测状态、搜索、分类和圆形网格。
- 截图：`phase3-followup-main.png`、`phase3-followup-progress.png`、`phase3-followup-progress-reopen.png`。

## 7. 测试与构建

- `npx tsc --noEmit`：通过
- `npm run build`：通过
- `npm run test:ui`：**20 passed / 0 failed**
  - JDK 可卸载实例、JBR 阻止原因、多个安全实例选择、精确路径；
  - 外部 Homebrew formula、进度条多次重开、极简详情、确认、键盘、主题、搜索、分类。
- `cargo test --offline --lib`：**14 passed / 0 failed / 2 ignored**
  - JDK bundle 根解析、拒绝未知路径、版本/归属/固定 brew 命令/缓存。
- 真实只读 JDK 分析：6 个 bundle 识别成功，4 个可安全卸载、1 个 root 阻止、1 个 JBR 阻止。
- 真实 macOS 应用：**31.57 MiB**，已启动。
- 本轮未执行真实 JDK 卸载、安装或外部 Homebrew 卸载。

## 8. 截图

- `docs/screenshots/phase3-jdk-uninstall.png`：JDK 版本列表，26/21/17.0.19/17.0.2 显示卸载，17.0.14 显示原因。
- `docs/screenshots/phase3-jdk-confirm.png`：JDK 26 卸载确认，目标 `openjdk-26` 和 PATH 风险提示。
- `docs/screenshots/phase3-jdk-native.png`：真实 macOS 窗口。
- `docs/screenshots/phase3-followup-main.png`：无标题主界面。
- `docs/screenshots/phase3-followup-progress*.png`：进度条修复。

## 9. 尚待验收

1. 你在 GUI 中实际卸载一个可安全 JDK（建议先 17.0.19 或 21.0.13.0.2，避免当前 PATH 的 17.0.2）验证废纸篓与重新检测。
2. 外部 Homebrew 卸载仍需一个真实外部 brew 开发包验证。
3. 系统/官方/Conda/rustup/app 安装未提供卸载，原因见上表。
4. 未进入第四阶段。

完成后停止，等待验收。
