# ==============================================================================
# Tuquet CLI - Modern One-Liner Web Installer (Windows x64)
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
Write-Host "  Tuquet Unified Zero-Dependency Installer" -ForegroundColor White
Write-Host "  Autonomous Browser Automation & Distributed Mesh Runtime" -ForegroundColor Gray
Write-Host ""

# 1. Resolve Canonical SSOT Paths
$UserHome = if ($env:USERPROFILE) { $env:USERPROFILE } else { $env:HOME }
$TuquetRoot = if ($env:TUQUET_HOME) { $env:TUQUET_HOME } elseif ($env:TUQUET_DIR) { $env:TUQUET_DIR } elseif ($env:SPECTER_HOME) { $env:SPECTER_HOME } elseif ($env:SPECTER_DIR) { $env:SPECTER_DIR } else { Join-Path $UserHome ".tuquet" }
$SpecterRoot = $TuquetRoot
$SpecterBinDir = Join-Path $SpecterRoot "bin"
$TargetExe = Join-Path $SpecterBinDir "tuquet.exe"

Write-Step "Preparing canonical SSOT directory: $SpecterBinDir"
if (-not (Test-Path $SpecterBinDir)) {
    New-Item -ItemType Directory -Path $SpecterBinDir -Force | Out-Null
}

# 2. Acquire tuquet.exe Binary
# If running locally from a built repository, use local release binary
$LocalBuildCandidates = @()
if (-not [string]::IsNullOrWhiteSpace($PSScriptRoot)) {
    $LocalBuildCandidates += @(
        (Join-Path $PSScriptRoot "target\x86_64-pc-windows-gnu\release\tuquet.exe"),
        (Join-Path $PSScriptRoot "target\release\tuquet.exe"),
        (Join-Path $PSScriptRoot "cli\target\x86_64-pc-windows-gnu\release\tuquet.exe"),
        (Join-Path $PSScriptRoot "cli\target\release\tuquet.exe")
    )
}
$CurrentLoc = (Get-Location).Path
if (-not [string]::IsNullOrWhiteSpace($CurrentLoc)) {
    $LocalBuildCandidates += @(
        (Join-Path $CurrentLoc "target\x86_64-pc-windows-gnu\release\tuquet.exe"),
        (Join-Path $CurrentLoc "target\release\tuquet.exe"),
        (Join-Path $CurrentLoc "cli\target\x86_64-pc-windows-gnu\release\tuquet.exe"),
        (Join-Path $CurrentLoc "cli\target\release\tuquet.exe")
    )
}

$FoundLocal = $false
foreach ($Cand in $LocalBuildCandidates) {
    if ($Cand -and (Test-Path $Cand)) {
        Write-Step "Installing local compiled binary from: $Cand"
        Copy-Item -Path $Cand -Destination $TargetExe -Force
        # Also ensure tq.exe alias exists
        Copy-Item -Path $Cand -Destination (Join-Path $SpecterBinDir "tq.exe") -Force
        $FoundLocal = $true
        break
    }
}

if (-not $FoundLocal) {
    Write-Step "Discovering and downloading latest tuquet release for Windows x64..."
    [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12 -bor [Net.SecurityProtocolType]::Tls13
    $Downloaded = $false

    # Dynamic GitHub API query to always fetch the latest release asset
    $ZipUrls = @()
    try {
        $ApiHeaders = @{ 'User-Agent' = 'curl/8.0' }
        $ReleaseMeta = Invoke-RestMethod -Uri "https://api.github.com/repos/tuquet/cli/releases/latest" -Headers $ApiHeaders -ErrorAction SilentlyContinue
        if ($ReleaseMeta -and $ReleaseMeta.assets) {
            $Matched = $ReleaseMeta.assets | Where-Object { $_.name -like "*windows-x64*.zip" } | Select-Object -ExpandProperty browser_download_url
            if ($Matched) {
                $ZipUrls += $Matched
            }
        }
    } catch {
        # Fallback to direct latest release asset URLs
    }

    $ReleaseBase = "https://github.com/tuquet/cli/releases/latest/download"
    $ZipUrls += @(
        "$ReleaseBase/tuquet-windows-x64.zip"
    )
    $ExeUrl = "$ReleaseBase/tuquet.exe"

    # Try downloading and extracting release ZIP asset
    $TempZip = Join-Path $env:TEMP "tuquet_dl_$([guid]::NewGuid().ToString('N')).zip"
    $TempDir = Join-Path $env:TEMP "tuquet_ext_$([guid]::NewGuid().ToString('N'))"

    foreach ($ZipUrl in $ZipUrls) {
        try {
            Invoke-WebRequest -Uri $ZipUrl -OutFile $TempZip -UserAgent "curl/8.0" -UseBasicParsing -ErrorAction Stop
            if (Test-Path $TempZip) {
                Expand-Archive -Path $TempZip -DestinationPath $TempDir -Force
                $ExtractedExe = Get-ChildItem -Path $TempDir -Filter "*.exe" -Recurse | Where-Object { $_.Name -match '^(tuquet|tq)\.exe$' } | Select-Object -First 1
                if ($ExtractedExe) {
                    Copy-Item -Path $ExtractedExe.FullName -Destination $TargetExe -Force
                    Copy-Item -Path $ExtractedExe.FullName -Destination (Join-Path $SpecterBinDir "tq.exe") -Force
                    $Downloaded = $true
                    Write-Success "Downloaded and installed $($ExtractedExe.Name) (as tuquet.exe & tq.exe) from $ZipUrl"
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
            Invoke-WebRequest -Uri $ExeUrl -OutFile $TargetExe -UserAgent "curl/8.0" -UseBasicParsing -ErrorAction Stop
            Copy-Item -Path $TargetExe -Destination (Join-Path $SpecterBinDir "tq.exe") -Force
            $Downloaded = $true
            Write-Success "Downloaded tuquet.exe directly from $ExeUrl"
        } catch {
            Write-Notice "Could not download from release assets. Checking local fallback..."
            if (-not (Test-Path $TargetExe)) {
                Write-Error "Failed to acquire tuquet.exe binary. Error: $_"
                exit 1
            }
        }
    }
} else {
    Write-Success "Placed tuquet.exe in $TargetExe"
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

# Unblock files and attempt exclusion registration
try {
    Get-ChildItem -Path $SpecterBinDir -Filter "*.exe" | ForEach-Object { Unblock-File $_.FullName -ErrorAction SilentlyContinue }
    Add-MpPreference -ExclusionPath $SpecterBinDir -ErrorAction SilentlyContinue
} catch {
    # Silently continue
}

# Update current session PATH immediately
if ($env:PATH -split ';' -notcontains $SpecterBinDir) {
    $env:PATH = "$SpecterBinDir;$env:PATH"
}

# 4. Run One-Command Bootstrap
Write-Host ""
Write-Step "Triggering automatic ecosystem bootstrap (SSOT, DB, MCP, Chromium LTS)..."
Write-Host ""

try {
    & $TargetExe bootstrap
} catch {
    Write-Notice "Bootstrap process deferred. You can run 'tq bootstrap' directly."
}

Write-Host ""
Write-Host "==============================================================================" -ForegroundColor Green
Write-Host "  SPECTER CLI INSTALLATION COMPLETED SUCCESSFULLY" -ForegroundColor Green
Write-Host "==============================================================================" -ForegroundColor Green
Write-Host ""
Write-Host "  To get started, open a new shell or run immediately:" -ForegroundColor White
Write-Host "    tuquet browser launch   Launch Antidetect Chromium browser sandbox" -ForegroundColor Cyan
Write-Host "    tuquet doctor           Check environment and system shims" -ForegroundColor Cyan
Write-Host "    tuquet status           Inspect unified services dashboard" -ForegroundColor Cyan
Write-Host "    tuquet bridge start     Start secure SOCKS5 mesh tunnel" -ForegroundColor Cyan
Write-Host ""
Write-Host "  Single Source of Truth Root: $SpecterRoot" -ForegroundColor Gray
Write-Host ""
