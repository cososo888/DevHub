param(
  [string]$InputPath = "$PSScriptRoot\winget-candidates.json",
  [string]$OutputPath = "$PSScriptRoot\winget-verified.json"
)

$ErrorActionPreference = "Stop"
if (-not (Get-Command winget -ErrorAction SilentlyContinue)) {
  throw "winget 不可用。请先在 Windows 上安装 App Installer / Windows Package Manager。"
}

$items = Get-Content -Raw -Encoding UTF8 $InputPath | ConvertFrom-Json
$valid = @()
$invalid = @()

foreach ($item in $items) {
  if ($item.id -notmatch '^[A-Za-z0-9][A-Za-z0-9._-]{0,127}$') {
    $invalid += [pscustomobject]@{ id = $item.id; reason = "invalid-id" }
    continue
  }
  winget show --id $item.id --source winget --accept-source-agreements 2>$null | Out-Null
  if ($LASTEXITCODE -eq 0) {
    $valid += $item
  } else {
    $invalid += [pscustomobject]@{ id = $item.id; reason = "not-found-or-show-failed" }
  }
}

$valid | ConvertTo-Json -Depth 4 | Set-Content -Encoding UTF8 $OutputPath
Write-Host ("valid={0} invalid={1}" -f $valid.Count, $invalid.Count)
$invalid | Format-Table -AutoSize
if ($valid.Count -lt 100) {
  Write-Warning "有效候选少于 100；不要删除已有有效项，补充候选后重新运行。"
}
