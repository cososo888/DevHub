/** Heuristic guidance only; the original output remains available as the technical log. */
export function failureFeedback(message: string, output = ""): { summary: string; advice: string } {
  const text = `${message}\n${output}`.toLowerCase();
  if (/already an app|已有应用|应用已存在/.test(text)) return { summary: "已有同名应用，安装未覆盖原文件", advice: "请查看已有应用；它未登记为 Homebrew 管理，DevHub 不会自动接管或删除。" };
  if (/无法确认.*状态|未确认.*已安装/.test(message)) return { summary: "安装命令已结束，但安装结果尚未确认", advice: "先刷新检测并查看日志，不要立即重复安装或卸载。" };
  if (/checksum|sha256|hash mismatch/.test(text)) return { summary: "下载文件校验未通过", advice: "不要绕过校验。查看日志中的下载来源及校验错误。" };
  if (/no space left|disk full/.test(text)) return { summary: "磁盘可用空间不足", advice: "检查剩余空间；DevHub 不会自动清理你的文件。" };
  if (/permission denied|not writable|password|a terminal is required|sudo:/.test(text)) return { summary: "安装可能需要权限或终端交互", advice: "查看日志确认具体要求；DevHub 不会自动提权或代输密码。" };
  if (/could not resolve|failed to connect|connection (reset|refused)|curl:|网络不可用/.test(text)) return { summary: "下载或网络连接出现问题", advice: "检查网络后再决定是否重试，具体原因以技术日志为准。" };
  if (/timed out|timeout|超时/.test(text)) return { summary: "操作超时，结果需要重新确认", advice: "先刷新检测；超时并不保证所有安装步骤都已撤销。" };
  return { summary: message.length <= 120 && !message.includes("\n") ? message : "操作未完成，请查看技术日志", advice: "先查看日志并刷新状态，确认原因后再操作。" };
}
