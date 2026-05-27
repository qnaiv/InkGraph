# IkaVision XP — 開発環境セットアップスクリプト
# 使い方: PowerShell を「管理者として実行」して以下を実行
#   Set-ExecutionPolicy Bypass -Scope Process -Force
#   .\setup.ps1

$ErrorActionPreference = "Stop"

function Write-Step($msg) { Write-Host "`n==> $msg" -ForegroundColor Cyan }
function Write-Ok($msg)   { Write-Host "  ✓ $msg"  -ForegroundColor Green }
function Write-Skip($msg) { Write-Host "  - $msg (スキップ)" -ForegroundColor DarkGray }

Write-Host @"

  🦑  IkaVision XP — セットアップ開始
  ──────────────────────────────────────
  所要時間: 約 10〜20 分 (回線速度による)
  インストール内容:
    • Rust + cargo
    • Node.js 20 LTS
    • Visual Studio C++ Build Tools
    • WebView2 Runtime

"@ -ForegroundColor White

# ──────────────────────────────────────────────────────────────
# 1. winget の存在確認
# ──────────────────────────────────────────────────────────────
Write-Step "winget を確認中..."
if (-not (Get-Command winget -ErrorAction SilentlyContinue)) {
    Write-Host "  winget が見つかりません。" -ForegroundColor Red
    Write-Host "  Microsoft Store から 'アプリ インストーラー' を更新してください。" -ForegroundColor Yellow
    Write-Host "  URL: https://aka.ms/getwinget"
    exit 1
}
Write-Ok "winget OK"

# ──────────────────────────────────────────────────────────────
# 2. Rust (rustup)
# ──────────────────────────────────────────────────────────────
Write-Step "Rust を確認中..."
if (Get-Command cargo -ErrorAction SilentlyContinue) {
    $v = cargo --version
    Write-Skip "cargo 既にインストール済み ($v)"
} else {
    Write-Host "  Rust をインストール中..." -ForegroundColor Yellow
    winget install --id Rustlang.Rustup -e --accept-package-agreements --accept-source-agreements
    # PATH を現セッションに反映
    $env:PATH += ";$env:USERPROFILE\.cargo\bin"
    Write-Ok "Rust インストール完了"
}

# ──────────────────────────────────────────────────────────────
# 3. Node.js
# ──────────────────────────────────────────────────────────────
Write-Step "Node.js を確認中..."
if (Get-Command node -ErrorAction SilentlyContinue) {
    $v = node --version
    Write-Skip "Node.js 既にインストール済み ($v)"
} else {
    Write-Host "  Node.js 20 LTS をインストール中..." -ForegroundColor Yellow
    winget install --id OpenJS.NodeJS.LTS -e --accept-package-agreements --accept-source-agreements
    # PATH を現セッションに反映
    $env:PATH += ";$env:PROGRAMFILES\nodejs"
    Write-Ok "Node.js インストール完了"
}

# ──────────────────────────────────────────────────────────────
# 4. Visual Studio Build Tools (C++ コンパイラ)
# ──────────────────────────────────────────────────────────────
Write-Step "C++ Build Tools を確認中..."
$vsWhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
$hasCpp = $false
if (Test-Path $vsWhere) {
    $installs = & $vsWhere -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -format json 2>$null | ConvertFrom-Json
    if ($installs.Count -gt 0) { $hasCpp = $true }
}
if ($hasCpp) {
    Write-Skip "C++ Build Tools 既にインストール済み"
} else {
    Write-Host "  Visual Studio Build Tools 2022 をインストール中..." -ForegroundColor Yellow
    Write-Host "  (C++ ワークロード込み、約 4GB)" -ForegroundColor DarkGray
    winget install --id Microsoft.VisualStudio.2022.BuildTools -e `
        --override "--quiet --wait --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended" `
        --accept-package-agreements --accept-source-agreements
    Write-Ok "C++ Build Tools インストール完了"
}

# ──────────────────────────────────────────────────────────────
# 5. WebView2 Runtime
# ──────────────────────────────────────────────────────────────
Write-Step "WebView2 を確認中..."
$wv2 = Get-Package -Name "Microsoft Edge WebView2 Runtime" -ErrorAction SilentlyContinue
if ($wv2) {
    Write-Skip "WebView2 既にインストール済み"
} else {
    Write-Host "  WebView2 Runtime をインストール中..." -ForegroundColor Yellow
    winget install --id Microsoft.EdgeWebView2Runtime -e --accept-package-agreements --accept-source-agreements
    Write-Ok "WebView2 インストール完了"
}

# ──────────────────────────────────────────────────────────────
# 6. npm install
# ──────────────────────────────────────────────────────────────
Write-Step "npm パッケージをインストール中..."
npm install
Write-Ok "npm install 完了"

# ──────────────────────────────────────────────────────────────
# 完了メッセージ
# ──────────────────────────────────────────────────────────────
Write-Host @"

  ✅  セットアップ完了！
  ──────────────────────────────────────
  次のステップ:

  1. ターミナルを一度閉じて開き直す（PATH 反映のため）

  2. 開発サーバー起動:
       npm run tauri:dev

  3. 初回ビルドは 5〜10 分かかります。
     ウィンドウが開いたら右下の OCR デバッグパネルで
     スプラトゥーン3のリザルト画像を読み込んでみてください。

"@ -ForegroundColor Green
