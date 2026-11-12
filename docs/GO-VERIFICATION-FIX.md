# Go 安装后验证误报修复

日期：2026-11-12
状态：已修复并测试；未重新安装或卸载 Go。

## 根因

`package_installed("go", "formula")` 原先执行：

```text
brew list --formula go
```

Go 的 keg 包含整个 Go 源码树，命令输出文件列表非常大，超过进程执行器的 256 KiB 安全上限。进程被中止，返回：

```text
检测命令输出超过安全上限
```

调用处使用了：

```rust
homebrew::package_installed(...).unwrap_or(false)
```

所以一次**查询失败**被错误当成“未安装”，显示：

> 安装命令结束，但 Homebrew 未确认该软件包已安装

这与实际状态无关；`brew list --versions go`、Homebrew JSON、`linked_keg` 都显示 Go 1.27.3 已安装。

## 修复

### 1. 不再读取无界文件列表

改用：

```text
brew list --formula --versions go
brew list --cask --versions <package>
```

输出为稳定的版本行，例如：

```text
go 1.27.3
```

并精确匹配第一列包名，不依赖退出码本身。

### 2. 多 formula JSON 精确匹配

formula 回退使用：

```text
brew info --json=v2 --formula go
```

解析 `formulae` 数组，按 `name`/`full_name` 精确匹配；不会读取第一个 formula。`brew info --json=v2 --installed go` 返回多个 formula 的情况不会影响验证。

### 3. 查询失败不再等于未安装

- `not installed` / `no such keg` → 明确的未安装；
- 其他非零状态、超时、超限、JSON 解析失败 → 返回错误；
- 安装任务显示“无法确认 Homebrew 状态”，保留日志，不再说“未安装”。

### 4. 竞态重试

安装完成后：

```text
最多 4 次查询，间隔 250 ms
```

避免 Homebrew 状态写入与验证查询之间的瞬时竞态。

## 回归测试

- `installed_versions_query_uses_versions_not_unbounded_file_list`
- `multi_formula_json_selects_exact_package_not_first`
- `live_installed_go_verification_is_true`（本机 Go 1.27.3，仅查询）
- 全量 UI 19 passed
- Rust 11 passed / 3 ignored
- Homebrew live search passed
- Release DMG 构建成功

没有重新安装或卸载 Go，没有修改 PATH/Shell/Homebrew 环境。
