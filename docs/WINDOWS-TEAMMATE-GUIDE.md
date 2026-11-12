# 给 Windows 队友的操作手册

你不需要会写代码。  
你需要做的是：**把项目下载下来，用 AI 编程工具操作它，按步骤确认，最后把 Windows 安装包交回来。**

这份手册只讲“怎么操作”，不要求你理解项目代码结构。

---

## 先说清楚这个项目是干什么的

**DevHub V2** 是一个桌面小工具：

- 首页是一堆圆形软件图标；
- 点软件可以看本机有没有安装、版本、路径；
- 点“安装”会让系统包管理器安装；
- 点“卸载”只处理由包管理器管理的软件；
- 不提供版本选择。

现在：

- **macOS 版已经做好**；
- **Windows 版还没有做**；
- 你要用 AI 工具，把 Windows 版做出来。

Windows 版的大方向已经定好：

- 只做 **Windows 10/11 x64**；
- 只使用 Windows 自带的 **winget**；
- 不接 Chocolatey、Scoop、微软商店；
- 最后打成 **NSIS 安装包**（`.exe`）；
- 不需要你手写代码。

---

## 你需要准备什么

### 硬件

一台 Windows 10/11 **x64** 电脑，最好不是你的主力工作机，或者至少先备份重要文件。

### 项目负责人会发给你

1. **GitHub 仓库邀请**；
2. **DeepSeek Harness 的 Windows 安装包或官方下载链接**；
3. DeepSeek Harness 的登录方式/工作区账号（如果有）；
4. 如果负责人有代码签名证书，也会另行说明；没有就做未签名包。

> 不要从第三方下载站下载 DeepSeek Harness。  
> 如果负责人没给安装包，就直接问他：“请把 DeepSeek Harness 的 Windows 安装包/官方链接发我。”

### 你需要自己安装

- GitHub Desktop
- DeepSeek Harness（用负责人给的安装包）
- 项目运行需要的 Windows 开发组件（下面会让 AI 帮你装）

---

# 第一步：接受 GitHub 邀请并下载代码

## 1.1 接受邀请

项目负责人会邀请你加入 private 仓库。

1. 打开你邮箱里的 GitHub 邀请；
2. 点击 **Accept invitation**；
3. 如果没有邮箱，登录 GitHub 后看通知。

仓库地址：

```text
https://github.com/cososo888/DevHub
```

如果你打开是 404，说明邀请没接受或没权限，找负责人。

## 1.2 安装 GitHub Desktop

下载地址：

https://desktop.github.com/

安装后：

1. 打开 GitHub Desktop；
2. 用你的 GitHub 账号登录；
3. 选择 **File → Clone repository**；
4. 选 **URL**；
5. 输入：

```text
https://github.com/cososo888/DevHub.git
```

6. 本地保存位置建议：

```text
C:\Users\你的用户名\Documents\DevHub
```

7. 克隆完成后，切换到分支：

```text
windows-handoff
```

在 GitHub Desktop 顶部 **Current branch** 里选择 `windows-handoff`。

> 不要直接在 `main` 上改。  
> 你所有工作都在 `windows-handoff` 分支。

## 1.3 如果 GitHub Desktop 登录失败

- 确认 GitHub 账号已经接受邀请；
- 确认你登录的是被邀请的账号；
- 退出 GitHub Desktop 重新登录；
- 还不行就找负责人。

---

# 第二步：安装 AI 工具 DeepSeek Harness

## 2.1 DeepSeek Harness 是什么

它是一个 **AI 编程工具**。

你可以把它理解成：

> 一个能看项目文件、能改代码、能运行命令的 AI 助手。

你不需要自己写代码，但你要：

- 把项目文件夹打开给它；
- 把任务用中文告诉它；
- 它执行命令前，你确认；
- 它改完，你检查结果；
- 它构建出安装包，你测试。

## 2.2 安装

1. 使用项目负责人给你的 Windows 安装包；
2. 双击安装；
3. 如果 Windows 提示“已保护你的电脑”，点 **更多信息 → 仍要运行**（仅限负责人给你的安装包）；
4. 打开 DeepSeek Harness；
5. 按负责人给的登录方式登录；
6. 选择 **打开文件夹 / Open Folder**；
7. 选择刚才克隆的：

```text
C:\Users\你的用户名\Documents\DevHub
```

8. 确认当前文件夹里有 `README.md`、`src`、`src-tauri`、`docs`。

> 如果找不到“打开文件夹”，看软件左上角菜单，或在欢迎页选择“打开项目/工作区”。

---

# 第三步：先让 AI 读懂项目，不要马上改

打开 DeepSeek Harness 的**新对话**，把下面整段复制进去：

```text
你是 DevHub V2 的 Windows 开发助手。

项目位置：C:\Users\你的用户名\Documents\DevHub
当前分支：windows-handoff

请先只读阅读这些文件：
- README.md
- docs/WINDOWS-PORT-REPORT.md
- docs/windows/WINDOWS-HANDOFF.md
- docs/windows/winget-candidates.json
- src-tauri/tauri.windows.conf.json

然后回答我：
1. 当前 Windows 版还缺什么；
2. 第一阶段应该先改哪些文件；
3. 你准备执行哪些命令；
4. 哪些操作有风险。

现在不要改代码，也不要安装/卸载任何软件，等我确认。
```

AI 会给你一份计划。你不需要看懂代码，重点看：

- 它有没有说“Windows 代码还没实现”；
- 它有没有提 winget；
- 它有没有想直接删除/覆盖很多东西；
- 它有没有要求管理员权限。

如果它说“项目已经完成 Windows 支持”，直接让它重新阅读文档，因为这是错的。

---

# 第四步：让 AI 检查并安装开发环境

看完计划后，发下面这段：

```text
请检查这台 Windows 电脑是否具备构建条件，并把结果列出来：
- Windows 版本和是否为 x64
- winget --version
- git --version
- node -v / npm -v
- rustc -V / cargo -V
- Visual Studio Build Tools 是否安装了 C++ 桌面开发
- WebView2 Runtime 是否存在

缺少的组件请先告诉我名称、下载方式、是否需要管理员权限。
不要直接安装，等我确认。
```

AI 会报告缺什么。

常见需要安装：

- Git
- Node.js LTS
- Rust（MSVC 工具链）
- Visual Studio Build Tools（C++ 桌面开发）
- WebView2 Runtime

你可以让 AI 执行安装，但**每一条安装命令都要先看清楚**，尤其是：

- 需要 UAC 的；
- 要下载大文件的；
- 要改系统设置的。

**允许**：安装 Git、Node、Rust、VS Build Tools、WebView2。  
**不允许**：关闭 Windows 安全中心、关闭 UAC、修改 PATH、卸载系统软件。

安装完成后让 AI 再验证一次。

---

# 第五步：验证 100 个 winget 软件 ID

这一步非常关键。  
`docs/windows/winget-candidates.json` 里的 100 个 ID 只是**候选**，还没有在 Windows 验证过。

把下面这段发给 AI：

```text
请执行 docs/windows/verify-winget.ps1，验证 100 个 winget 候选 ID。

要求：
1. 先说明脚本会做什么；
2. 运行前确认不会修改系统；
3. 输出有效数量/无效数量；
4. 对无效的 ID 列出来；
5. 不要伪造有效 ID；
6. 把结果写入 docs/windows/winget-verified.json；
7. 如果有效数少于 100，先不要继续做 UI，等我决定是否补充候选。
```

如果 PowerShell 说脚本不能运行，让 AI 用：

```powershell
powershell -ExecutionPolicy Bypass -File docs\windows\verify-winget.ps1
```

把结果截图或复制给负责人。

---

# 第六步：让 AI 做 Windows 适配

只有 winget ID 验证完成后，才发下面这段。

```text
现在开始 Windows 适配。

硬性要求：
- 只做 Windows 10/11 x64；
- 只使用 winget；
- 不接 msstore、Chocolatey、Scoop；
- 不提供版本选择；
- 保留现有圆形网格、搜索、分类、详情卡片和主题；
- 不能破坏 macOS 现有代码；
- 不修改 PATH/Shell；
- 不静默提权；
- 不删除用户数据。

请按 docs/WINDOWS-TEAMMATE-GUIDE.md 和 docs/windows/WINDOWS-HANDOFF.md 执行：

1. 让 Windows 能编译；
2. 实现 winget 的状态查询、搜索、安装、卸载、验证；
3. 把 Windows 100 款软件接入现有界面；
4. 实现 Windows 本机检测；
5. 保留全局搜索：输入查询时忽略分类；
6. 安装/卸载前必须有确认；
7. 安装后必须用 winget 再查一次，不能只看退出码；
8. 失败要显示短原因和可展开日志；
9. 每完成一项，先向我报告改了什么、测试结果，再继续下一项。

不要一次性大改。不要删除 Mac 功能。不要 force push。
```

你不需要检查代码。  
你要让 AI **一项一项做**，每完成一项就停下来报告。

可以追问：

```text
请用非程序员能听懂的话，告诉我：
1. 这次改了什么；
2. 我怎么手动测试；
3. 有什么风险；
4. 下一步是什么。
```

---

# 第七步：让 AI 构建 Windows 安装包

当 AI 说功能完成、测试通过后，发：

```text
请构建 Windows x64 NSIS 安装包。

先确认：
- 当前分支是 windows-handoff；
- 所有改动已提交；
- npm run test:ui 通过；
- cargo test 通过；
- 没有强推、没有删除用户文件。

然后运行：
npm ci
npm run tauri -- build --bundles nsis

把生成的 .exe 路径告诉我，并计算 SHA256。
```

预期产物类似：

```text
src-tauri\target\release\bundle\nsis\DevHub V2_0.1.0_x64-setup.exe
```

未签名包会触发 SmartScreen，这是正常的，不代表安装包坏了。

---

# 第八步：手动测试安装包

用一台测试 Windows 电脑，不要用有重要数据的机器。

测试顺序：

1. 双击 `.exe` 安装；
2. 打开 DevHub V2；
3. 首页应显示约 100 个软件；
4. 切换分类；
5. 搜索 `git`；
6. 搜索 `postman`，确认有 Homebrew/winget 之外的全库结果；
7. 搜索时切换分类，结果范围不应变化；
8. 打开一个软件详情；
9. 点击安装，先看确认框和 UAC；
10. 建议测试安装 `ripgrep`，不要先拿数据库测试；
11. 安装后确认软件变蓝、版本和路径出现；
12. 卸载 `ripgrep`，确认只影响 winget 管理的包；
13. 断网、取消 UAC，确认有错误提示和重试/日志；
14. 测试中文用户名、空格路径、125%/150% 缩放。

**不要测试卸载系统软件。**  
**不要测试删除用户数据。**  
**不要修改 PATH/Shell。**

---

# 第九步：提交到 GitHub

AI 可以帮你提交，但你需要确认它没有搞坏东西。

让 AI 执行：

```text
请检查 git status，把本次 Windows 相关修改提交到 windows-handoff 分支。
不要提交测试安装产生的临时文件，不要 force push。
提交后 push 到 origin。
```

然后在 GitHub 页面：

1. 打开仓库；
2. 会看到 **Compare & pull request**；
3. base 选 `main`；
4. compare 选 `windows-handoff`；
5. 标题写清楚：`feat: add Windows winget support`；
6. 描述里写测试结果；
7. 不要直接合并，等负责人检查。

如果 GitHub Desktop 更方便：

- 左下角写 Summary；
- **Commit to windows-handoff**；
- **Push origin**；
- 网页创建 Pull Request。

---

# 第十步：交给负责人什么

完成后发这些：

1. Pull Request 链接；
2. 最终 commit SHA；
3. `DevHub-V2-0.1.0-x64-setup.exe`；
4. SHA-256；
5. `winget --version`；
6. 测试结果；
7. 截图：
   - 首页
   - 搜索
   - 详情
   - 安装确认
8. 还没解决的问题。

---

# 和 AI 说话的原则

## 一定要说

- “先解释你要做什么，再执行。”
- “执行删除、覆盖、卸载、改 PATH 前必须问我。”
- “一次只做一项。”
- “做完告诉我怎么手动测试。”
- “不要 force push。”
- “不要把没测试的功能说成完成。”

## 绝对不要说

- “随便改，不用问我。”
- “把系统清理一下。”
- “卸载所有旧版本。”
- “关闭 UAC。”
- “把 PATH 改一下。”
- “强制覆盖远程分支。”

---

# 常见问题

## AI 说找不到 winget

让 AI 检查 Windows 版本，并从 Microsoft Store 安装 **App Installer**。  
不要从第三方网站下载 winget。

## 构建时报 `link.exe not found`

让 AI 检查 Visual Studio Build Tools 是否安装了 **C++ 桌面开发**。

## 安装包打不开 / SmartScreen

未签名包的正常现象。测试机右键 → 更多信息 → 仍要运行。

## GitHub 不能 push

确认 GitHub Desktop 已登录、账号已被邀请、当前分支是 `windows-handoff`。  
不要 force push。

## AI 把 Mac 功能改坏了

立刻停止，让它：

```text
请检查 git diff，撤回与 Windows 无关的修改。不要删除文件。
```

然后让负责人看。

---

# 一句话记住

> 你不是程序员也没关系。  
> 你的工作是：**打开项目 → 让 AI 按步骤做 → 你确认危险操作 → 构建 → 测试 → 提交 PR。**
