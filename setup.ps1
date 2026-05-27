# IkaVision XP - Dev environment setup
# Run as Administrator:
#   Set-ExecutionPolicy Bypass -Scope Process -Force; .\setup.ps1

$ErrorActionPreference = "Stop"

function Step  { param($m) Write-Host "`n==> $m" -ForegroundColor Cyan }
function Ok    { param($m) Write-Host "  [OK]   $m" -ForegroundColor Green }
function Skip  { param($m) Write-Host "  [SKIP] $m (already installed)" -ForegroundColor DarkGray }
function Fail  { param($m) Write-Host "  [ERR]  $m" -ForegroundColor Red; exit 1 }

Write-Host ""
Write-Host "  IkaVision XP - Setup" -ForegroundColor White
Write-Host "  Installing: Rust, Node.js, VS Build Tools, WebView2" -ForegroundColor DarkGray
Write-Host ""

# ------------------------------------------------------------------
# 1. winget check
# ------------------------------------------------------------------
Step "Checking winget..."
if (-not (Get-Command winget -ErrorAction SilentlyContinue)) {
    Fail "winget not found. Update 'App Installer' from Microsoft Store: https://aka.ms/getwinget"
}
Ok "winget found"

# ------------------------------------------------------------------
# 2. Rust
# ------------------------------------------------------------------
Step "Checking Rust..."
$cargoPath = Join-Path $env:USERPROFILE ".cargo\bin\cargo.exe"
if ((Get-Command cargo -ErrorAction SilentlyContinue) -or (Test-Path $cargoPath)) {
    Skip "Rust / cargo"
} else {
    Write-Host "  Installing Rust..." -ForegroundColor Yellow
    winget install --id Rustlang.Rustup -e --accept-package-agreements --accept-source-agreements
    $env:PATH = $env:PATH + ";" + (Join-Path $env:USERPROFILE ".cargo\bin")
    Ok "Rust installed"
}

# ------------------------------------------------------------------
# 3. Node.js
# ------------------------------------------------------------------
Step "Checking Node.js..."
if (Get-Command node -ErrorAction SilentlyContinue) {
    Skip "Node.js $(node --version)"
} else {
    Write-Host "  Installing Node.js LTS..." -ForegroundColor Yellow
    winget install --id OpenJS.NodeJS.LTS -e --accept-package-agreements --accept-source-agreements
    # Refresh PATH from registry
    $machinePath = [System.Environment]::GetEnvironmentVariable("PATH", "Machine")
    $userPath    = [System.Environment]::GetEnvironmentVariable("PATH", "User")
    $env:PATH    = $machinePath + ";" + $userPath
    Ok "Node.js installed"
}

# ------------------------------------------------------------------
# 4. Visual Studio C++ Build Tools
# ------------------------------------------------------------------
Step "Checking C++ Build Tools..."
$vsWhere = Join-Path ${env:ProgramFiles(x86)} "Microsoft Visual Studio\Installer\vswhere.exe"
$hasCpp  = $false
if (Test-Path $vsWhere) {
    $result = & $vsWhere -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 2>$null
    if ($result) { $hasCpp = $true }
}
if ($hasCpp) {
    Skip "C++ Build Tools"
} else {
    Write-Host "  Installing VS Build Tools 2022 (C++ workload, ~4GB)..." -ForegroundColor Yellow
    $override = "--quiet --wait --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
    winget install --id Microsoft.VisualStudio.2022.BuildTools -e `
        --override $override `
        --accept-package-agreements --accept-source-agreements
    Ok "C++ Build Tools installed"
}

# ------------------------------------------------------------------
# 5. WebView2 Runtime
# ------------------------------------------------------------------
Step "Checking WebView2..."
$wv2Key = "HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}"
if (Test-Path $wv2Key) {
    Skip "WebView2 Runtime"
} else {
    Write-Host "  Installing WebView2 Runtime..." -ForegroundColor Yellow
    winget install --id Microsoft.EdgeWebView2Runtime -e --accept-package-agreements --accept-source-agreements
    Ok "WebView2 installed"
}

# ------------------------------------------------------------------
# 6. npm install
# ------------------------------------------------------------------
Step "Running npm install..."
npm install
Ok "npm install done"

# ------------------------------------------------------------------
# Done
# ------------------------------------------------------------------
Write-Host ""
Write-Host "  Setup complete!" -ForegroundColor Green
Write-Host ""
Write-Host "  Next steps:" -ForegroundColor White
Write-Host "    1. Close this terminal and open a new one (to reload PATH)"
Write-Host "    2. Run:  npm run tauri:dev"
Write-Host "    3. First build takes 5-10 min. When the window opens,"
Write-Host "       use the OCR Debug Panel (bottom-right) to test with"
Write-Host "       a Splatoon 3 result screen screenshot."
Write-Host ""
