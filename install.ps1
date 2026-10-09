# ==============================================================================
# Specter CLI - Modern One-Liner Web Installer (Windows x64)
# ==============================================================================
# Usage (Clean/Blank Machine):
#   powershell -c "irm https://raw.githubusercontent.com/tuquet/cli/main/install.ps1 | iex"
#   or locally: .\install.ps1
#
# Requirements: Windows 10/11 x64 (Zero external dependencies needed)
# ==============================================================================

$ErrorActionPreference = 'Stop'

function Write-Step {
    param([string]$Message)
    Write-Host "  [+] $Message" -ForegroundColor Cyan
}

function Write-Success {
    param([string]$Message)
    Write-Host "  [OK] $Message" -ForegroundColor Green
}

function Write-Notice {
    param([string]$Message)
    Write-Host "  [!] $Message" -ForegroundColor Yellow
}

Write-Host ""
Write-Host "   _____                 __            " -ForegroundColor Cyan
Write-Host "  / ___/____  ___  _____/ /____  _____ " -ForegroundColor Cyan
Write-Host "  \__ \/ __ \/ _ \/ ___/ __/ _ \/ ___/ " -ForegroundColor Cyan
Write-Host " ___/ / /_/ /  __/ /__/ /_/  __/ /     v1.0.0" -ForegroundColor Cyan
Write-Host "/____/ .___/\___/\___/\__/\___/_/      " -ForegroundColor Cyan
Write-Host "    /_/                                " -ForegroundColor Cyan
Write-Host ""
Write-Host "  Specter Unified Zero-Dependency Installer" -ForegroundColor White
Write-Host "  Autonomous Browser Automation & Distributed Mesh Runtime" -ForegroundColor Gray
Write-Host ""

# 1. Resolve Canonical SSOT Paths
$UserHome = if ($env:USERPROFILE) { $env:USERPROFILE } else { $env:HOME }
$SpecterRoot = if ($env:SPECTER_HOME) { $env:SPECTER_HOME } elseif ($env:SPECTER_DIR) { $env:SPECTER_DIR } elseif ($env:TUQUET_HOME) { $env:TUQUET_HOME } elseif ($env:TUQUET_DIR) { $env:TUQUET_DIR } else { Join-Path $UserHome ".specter" }
$SpecterBinDir = Join-Path $SpecterRoot "bin"
$TargetExe = Join-Path $SpecterBinDir "specter.exe"

# Auto-migration if ~/.tuquet exists and ~/.specter does not
$LegacyTuquet = Join-Path $UserHome ".tuquet"
if ((-not (Test-Path $SpecterRoot)) -and (Test-Path $LegacyTuquet)) {
    try {
        New-Item -ItemType SymbolicLink -Path $SpecterRoot -Target $LegacyTuquet -Force | Out-Null
    } catch {
        # Fallback if unprivileged
    }
}

Write-Step "Preparing canonical SSOT directory: $SpecterBinDir"
if (-not (Test-Path $SpecterBinDir)) {
    New-Item -ItemType Directory -Path $SpecterBinDir -Force | Out-Null
}

# 2. Acquire specter.exe Binary
# If running locally from a built repository, use local release binary
$LocalBuildCandidates = @(
    (Join-Path $PSScriptRoot "target\x86_64-pc-windows-gnu\release\specter.exe"),
    (Join-Path $PSScriptRoot "target\release\specter.exe"),
    (Join-Path $PSScriptRoot "cli\target\x86_64-pc-windows-gnu\release\specter.exe"),
    (Join-Path $PSScriptRoot "cli\target\release\specter.exe"),
    (Join-Path (Get-Location) "target\x86_64-pc-windows-gnu\release\specter.exe"),
    (Join-Path (Get-Location) "target\release\specter.exe"),
    (Join-Path (Get-Location) "cli\target\x86_64-pc-windows-gnu\release\specter.exe"),
    (Join-Path (Get-Location) "cli\target\release\specter.exe")
)

$FoundLocal = $false
foreach ($Cand in $LocalBuildCandidates) {
    if ($Cand -and (Test-Path $Cand)) {
        Write-Step "Installing local compiled binary from: $Cand"
        Copy-Item -Path $Cand -Destination $TargetExe -Force
        $FoundLocal = $true
        break
    }
}

if (-not $FoundLocal) {
    Write-Step "Downloading latest specter release for Windows x64..."
    $ReleaseBase = "https://github.com/tuquet/cli/releases/latest/download"
    $ZipUrls = @(
        "$ReleaseBase/specter-windows-x64.zip",
        "$ReleaseBase/specter-v1.0.0-windows-x64.zip"
    )
    $ExeUrl = "$ReleaseBase/specter.exe"

    [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12 -bor [Net.SecurityProtocolType]::Tls13
    $Downloaded = $false

    # Try downloading and extracting release ZIP asset
    $TempZip = Join-Path $env:TEMP "specter_dl_$([guid]::NewGuid().ToString('N')).zip"
    $TempDir = Join-Path $env:TEMP "specter_ext_$([guid]::NewGuid().ToString('N'))"

    foreach ($ZipUrl in $ZipUrls) {
        try {
            Invoke-WebRequest -Uri $ZipUrl -OutFile $TempZip -UseBasicParsing -ErrorAction Stop
            if (Test-Path $TempZip) {
                Expand-Archive -Path $TempZip -DestinationPath $TempDir -Force
                $ExtractedExe = Get-ChildItem -Path $TempDir -Filter "specter.exe" -Recurse | Select-Object -First 1
                if ($ExtractedExe) {
                    Copy-Item -Path $ExtractedExe.FullName -Destination $TargetExe -Force
                    $Downloaded = $true
                    Write-Success "Downloaded and extracted specter.exe from $ZipUrl"
                    break
                }
            }
        } catch {
            # Try next release candidate
        } finally {
            if (Test-Path $TempZip) { Remove-Item $TempZip -Force -ErrorAction SilentlyContinue }
            if (Test-Path $TempDir) { Remove-Item $TempDir -Recurse -Force -ErrorAction SilentlyContinue }
        }
    }

    # If ZIP not available, try raw EXE fallback
    if (-not $Downloaded) {
        try {
            Invoke-WebRequest -Uri $ExeUrl -OutFile $TargetExe -UseBasicParsing -ErrorAction Stop
            $Downloaded = $true
            Write-Success "Downloaded specter.exe directly from $ExeUrl"
        } catch {
            Write-Notice "Could not download from release assets. Checking local fallback..."
            if (-not (Test-Path $TargetExe)) {
                Write-Error "Failed to acquire specter.exe binary. Error: $_"
                exit 1
            }
        }
    }
} else {
    Write-Success "Placed specter.exe in $TargetExe"
}

# 3. Add to User Environment PATH (No Administrator privileges required)
Write-Step "Configuring User PATH environment variable..."
$CurrentPath = [Environment]::GetEnvironmentVariable("Path", "User")
if (-not $CurrentPath) { $CurrentPath = "" }

$PathEntries = $CurrentPath -split ';' | Where-Object { $_ -ne "" }
if ($PathEntries -notcontains $SpecterBinDir) {
    $NewPath = if ($CurrentPath) { "$CurrentPath;$SpecterBinDir" } else { $SpecterBinDir }
    [Environment]::SetEnvironmentVariable("Path", $NewPath, "User")
    Write-Success "Added $SpecterBinDir to User PATH registry"
} else {
    Write-Success "$SpecterBinDir is already in User PATH registry"
}

# Update current session PATH immediately
if ($env:PATH -split ';' -notcontains $SpecterBinDir) {
    $env:PATH = "$SpecterBinDir;$env:PATH"
}

# 4. Run One-Command Bootstrap
Write-Host ""
Write-Step "Triggering automatic ecosystem bootstrap (SSOT, DB, MCP, Chromium LTS)..."
Write-Host ""

& $TargetExe bootstrap

Write-Host ""
Write-Host "==============================================================================" -ForegroundColor Green
Write-Host "  SPECTER CLI INSTALLATION COMPLETED SUCCESSFULLY" -ForegroundColor Green
Write-Host "==============================================================================" -ForegroundColor Green
Write-Host ""
Write-Host "  To get started, open a new shell or run immediately:" -ForegroundColor White
Write-Host "    specter browser launch   Launch Antidetect Chromium browser sandbox" -ForegroundColor Cyan
Write-Host "    specter doctor           Check environment and system shims" -ForegroundColor Cyan
Write-Host "    specter status           Inspect unified services dashboard" -ForegroundColor Cyan
Write-Host "    specter bridge start     Start secure SOCKS5 mesh tunnel" -ForegroundColor Cyan
Write-Host ""
Write-Host "  Single Source of Truth Root: $SpecterRoot" -ForegroundColor Gray
Write-Host ""
