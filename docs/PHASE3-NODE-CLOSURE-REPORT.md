# DevHub V2 · Node.js 多版本闭环报告

日期：2026-11-11
状态：**安装、卸载、版本保留闭环已验证通过；22.23.4 由用户手动确认卸载，当前保留安装入口。**

## 1. fnm 与测试版本

- fnm 1.39.0，通过 Homebrew 安装（用户已授权）。
- 测试 Node **22.23.4** 通过 DevHub 界面安装成功，完成于 23:53:59。
- 安装位置：`~/.local/share/fnm/node-versions/v22.23.4`
- 实际占用：约 **192 MB**。
- 安装后检测到 **20** 个安装实例，Node 22.23.4 显示「已安装 / 卸载」。
- 默认别名保持 **22.14.0**；PATH Node 仍是 `/usr/local/bin/node` 26.13.0；未修改 Shell。
- 首次尝试因 300 秒下载超时失败；将 fnm 安装超时提高到 1800 秒后重试成功。失败尝试留下约 **118 MB** 的 fnm `.downloads` 临时目录，未清理。

## 2. 真实安装验证

- 界面确认版本：22.23.4。
- 进度条显示「正在通过 fnm 安装 Node.js…」，无伪造百分比。
- 安装后刷新：20 instances；22.23.4 已安装；22.14.0 仍已安装且受默认别名保护；26.13.0 未被修改。
- 截图：`phase3-node-installing.png`、`phase3-node-installed.png`。

## 3. 真实卸载验证

用户在卸载确认框显示目标路径后**手动点击确认**，22.23.4 于约 00:03:32 通过 fnm 正确卸载。此前“未经确认”的判断是错误的，特此更正。

- 卸载来源记录：`uninstall_history` `node 22.23.4 fnm ...`。
- 确认框显示的目标：`/Users/yujian/.local/share/fnm/node-versions/v22.23.4/installation/bin/node`，版本号 22.23.4。
- 影响：只移除 fnm 的 22.23.4；22.14.0 默认版本、26.13.0、PATH/Shell 均未改变。
- 卸载后重新检测为 19 个实例；Node 22.23.4 行保留为「安装」，验证了可重装入口。

## 4. 卸载安全校验

- 版本号、实例路径必须精确匹配 fnm 的 `version_root(version)`。
- 默认别名版本直接阻止；22.14.0 的 `?` 原因即为“fnm 默认别名，卸载会改变默认 Node 版本”。
- 未匹配、目标不是 fnm 管理、默认别名均返回错误。
- 用户手动确认后执行真实卸载，目标与版本严格匹配；默认别名未被修改。
- 非破坏性测试还验证了默认别名阻止与版本/路径不匹配。

## 5. 版本目录与性能

- Node 展示 **12 行**：每大版本最新 patch + 已安装版本；其中 **10 个可安装**。
- 首屏实测 **2.04 秒**（fnm 远程目录 + 检测）；fnm 目录缓存 10 分钟。
- 没有展示旧的无 arm64 构建版本（过滤 major < 16）。
- 每个展示为可安装的版本由 adapter-aware live test 验证：fnm/uv/Homebrew 至少一个真实适配器可解析。
- 列表保持极简；没有为所有 patch 铺满 200+ 行。

## 6. 测试与构建

- `npx tsc --noEmit`：通过
- `npm run test:ui`：**21 passed / 0 failed**
- `cargo test --offline --lib`：**19 passed / 0 failed / 4 ignored**
- live version catalog：**通过**（fnm/uv/Homebrew 按 adapter 校验）
- node catalog timing：12 rows，约 2.04s
- 真实 macOS app：**32.03 MiB**，已启动
- 未继续开发 Python uv 或其它软件；uv 代码仅作为中断前已有工作保留。

## 7. 限制与遗留

1. 22.23.4 已按用户确认卸载；当前保留安装入口，可随时重新安装。
2. 现有官方 Node 26.13.0 仍是 root/pkg 安装，不支持自动卸载。
3. fnm 默认 22.14.0 不允许卸载，这是保护行为。
4. 首次失败尝试留下约 118 MB 的 fnm `.downloads` 临时目录，未清理。
5. 首次超时问题已修复（30 分钟），但真实下载耗时仍受网络影响。
6. 本轮仅完成 Node.js；uv/Python 未继续扩展。

## 8. 截图

- `docs/screenshots/phase3-node-installing.png`
- `docs/screenshots/phase3-node-installed.png`
- `docs/screenshots/phase3-node-uninstall-confirm.png`
- `docs/screenshots/phase3-node-final.png`
