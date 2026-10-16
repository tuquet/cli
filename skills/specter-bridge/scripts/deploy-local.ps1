# Specter Bridge Local Deploy Script
# Strict ASCII encoding - no diacritics to avoid Windows PowerShell parsing issues

$ErrorActionPreference = 'Stop'

Write-Host "==========================================" -ForegroundColor Cyan
Write-Host " [SPECTER-BRIDGE] Local Deploy Pipeline" -ForegroundColor Cyan
Write-Host "==========================================" -ForegroundColor Cyan

# 1. Stop active bridge daemons to release file locks on Windows
Write-Host "`n[1/4] Stopping active bridge daemons..." -ForegroundColor Yellow
try {
    $ErrorActionPreference = 'SilentlyContinue'
    specter bridge stop
    $ErrorActionPreference = 'Stop'
    Write-Host "      Bridge daemons stopped successfully." -ForegroundColor Green
} catch {
    Write-Host "      Notice: specter bridge stop returned non-zero (may not have been running)." -ForegroundColor Gray
}

# 2. Recompile and install the CLI crate dynamically
Write-Host "`n[2/4] Building and installing latest specter CLI binary..." -ForegroundColor Yellow

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path

# Dynamically locate CLI workspace without machine-specific hardcoded paths
$CandidateRoots = @(
    $env:SPECTER_CLI_DIR,
    $env:SPECTER_WORKSPACE,
    (Resolve-Path (Join-Path $ScriptDir "..\..\..\..\cli") -ErrorAction SilentlyContinue),
    (Resolve-Path (Join-Path $ScriptDir "..\..\..\cli") -ErrorAction SilentlyContinue),
    (Resolve-Path (Join-Path (Get-Location) "cli") -ErrorAction SilentlyContinue),
    (Resolve-Path (Get-Location) -ErrorAction SilentlyContinue)
) | Where-Object { $_ -and (Test-Path (Join-Path $_ "Cargo.toml")) }

$CliWorkspace = $null
if ($CandidateRoots.Count -gt 0) {
    $CliWorkspace = $CandidateRoots[0].Path
}

if (-not $CliWorkspace) {
    Write-Error "Could not locate specter/cli workspace containing Cargo.toml."
    exit 1
}

Write-Host "      Found CLI workspace: $CliWorkspace" -ForegroundColor Gray

Push-Location $CliWorkspace
try {
    cargo install --path .
    if ($LASTEXITCODE -ne 0) {
        throw "cargo install failed with exit code $LASTEXITCODE"
    }
    Write-Host "      CLI binary compiled and installed successfully." -ForegroundColor Green
} finally {
    Pop-Location
}

# 3. Relaunch Bridge with SSH tunnel (port 2222)
Write-Host "`n[3/4] Relaunching bridge daemon with SSH tunnel (--ssh)..." -ForegroundColor Yellow
$ErrorActionPreference = 'SilentlyContinue'
specter bridge start --ssh
$ErrorActionPreference = 'Stop'

# Allow a moment for background socket bindings
Start-Sleep -Seconds 1

# 4. Smoke Verification
Write-Host "`n[4/4] Verifying bridge status and active listeners..." -ForegroundColor Yellow
$ErrorActionPreference = 'SilentlyContinue'
specter bridge status
$ErrorActionPreference = 'Stop'

Write-Host "`n[SUCCESS] Specter Bridge local deployment complete!" -ForegroundColor Green
