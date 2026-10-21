# ==============================================================================
# CLI - Modern One-Liner Web & Local Installer (Windows x64)
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
Write-Host " ___/ / /_/ /  __/ /__/ /_/  __/ /     v1.2.1" -ForegroundColor Cyan
Write-Host "/____/ .___/\___/\___/\__/\___/_/      " -ForegroundColor Cyan
Write-Host "    /_/                                " -ForegroundColor Cyan
Write-Host ""
Write-Host "  Unified Zero-Dependency Installer" -ForegroundColor White
Write-Host "  Autonomous Browser Automation & Distributed Mesh Runtime" -ForegroundColor Gray
Write-Host ""

# 1. Resolve Canonical SSOT Paths (~/.tq)
$UserHome = if ($env:USERPROFILE) { $env:USERPROFILE } else { $env:HOME }
$TqRoot = if ($env:TQ_HOME) { $env:TQ_HOME } elseif ($env:SPECTER_HOME) { $env:SPECTER_HOME } elseif ($env:TUQUET_HOME) { $env:TUQUET_HOME } else { Join-Path $UserHome ".tq" }
$TqBinDir = Join-Path $TqRoot "bin"
$TargetExe = Join-Path $TqBinDir "tq.exe"

Write-Step "Preparing canonical SSOT directory: $TqBinDir"
if (-not (Test-Path $TqBinDir)) {
    New-Item -ItemType Directory -Path $TqBinDir -Force | Out-Null
}

# 2. Acquire tq.exe Binary
# If running locally from a built repository, use local compiled binary
$LocalBuildCandidates = @()
$CheckDirs = @($PSScriptRoot, (Get-Location).Path)
foreach ($Dir in $CheckDirs) {
    if (-not [string]::IsNullOrWhiteSpace($Dir)) {
        $LocalBuildCandidates += @(
            (Join-Path $Dir "target\x86_64-pc-windows-gnu\release\tq.exe"),
            (Join-Path $Dir "target\release\tq.exe"),
            (Join-Path $Dir "cli\target\x86_64-pc-windows-gnu\release\tq.exe"),
            (Join-Path $Dir "cli\target\release\tq.exe"),
            (Join-Path $Dir "..\cli\target\x86_64-pc-windows-gnu\release\tq.exe"),
            (Join-Path $Dir "target\x86_64-pc-windows-gnu\release\tuquet.exe"),
            (Join-Path $Dir "cli\target\x86_64-pc-windows-gnu\release\tuquet.exe")
        )
    }
}

$FoundLocal = $false
foreach ($Cand in $LocalBuildCandidates) {
    if ($Cand -and (Test-Path $Cand)) {
        Write-Step "Installing local compiled binary from: $Cand"
        Copy-Item -Path $Cand -Destination $TargetExe -Force
        Copy-Item -Path $Cand -Destination (Join-Path $TqBinDir "spc.exe") -Force
        Copy-Item -Path $Cand -Destination (Join-Path $TqBinDir "tuquet.exe") -Force
        $FoundLocal = $true
        break
    }
}

if (-not $FoundLocal) {
    Write-Step "Discovering and downloading latest release for Windows x64..."
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
        "$ReleaseBase/windows-x64.zip"
    )
    $ExeUrl = "$ReleaseBase/tq.exe"

    # Try downloading and extracting release ZIP asset
    $TempZip = Join-Path $env:TEMP "dl_$([guid]::NewGuid().ToString('N')).zip"
    $TempDir = Join-Path $env:TEMP "ext_$([guid]::NewGuid().ToString('N'))"

    foreach ($ZipUrl in $ZipUrls) {
        try {
            Invoke-WebRequest -Uri $ZipUrl -OutFile $TempZip -UserAgent "curl/8.0" -UseBasicParsing -ErrorAction Stop
            if (Test-Path $TempZip) {
                Expand-Archive -Path $TempZip -DestinationPath $TempDir -Force
                $ExtractedExe = Get-ChildItem -Path $TempDir -Filter "*.exe" -Recurse | Where-Object { $_.Name -match '^(tuquet|tq|spc)\.exe$' } | Select-Object -First 1
                if ($ExtractedExe) {
                    Copy-Item -Path $ExtractedExe.FullName -Destination $TargetExe -Force
                    Copy-Item -Path $ExtractedExe.FullName -Destination (Join-Path $TqBinDir "spc.exe") -Force
                    Copy-Item -Path $ExtractedExe.FullName -Destination (Join-Path $TqBinDir "tuquet.exe") -Force
                    $Downloaded = $true
                    Write-Success "Downloaded and installed $($ExtractedExe.Name) from $ZipUrl"
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
            Copy-Item -Path $TargetExe -Destination (Join-Path $TqBinDir "spc.exe") -Force
            Copy-Item -Path $TargetExe -Destination (Join-Path $TqBinDir "tuquet.exe") -Force
            $Downloaded = $true
            Write-Success "Downloaded tq.exe directly from $ExeUrl"
        } catch {
            Write-Notice "Could not download from release assets. Checking local fallback..."
            if (-not (Test-Path $TargetExe)) {
                Write-Error "Failed to acquire tq.exe binary. Error: $_"
                exit 1
            }
        }
    }
} else {
    Write-Success "Placed tq.exe in $TargetExe"
}

# 3. Synchronize Scoop Shims if Scoop is installed
$ScoopShims = Join-Path $UserHome "scoop\shims"
if (Test-Path $ScoopShims) {
    Write-Step "Synchronizing Scoop shims in $ScoopShims..."
    $ShimContent = "@echo off`r`n`"$TargetExe`" %*`r`n"
    Set-Content -Path (Join-Path $ScoopShims "tq.cmd") -Value $ShimContent -Force
    Set-Content -Path (Join-Path $ScoopShims "tq.bat") -Value $ShimContent -Force
    Set-Content -Path (Join-Path $ScoopShims "spc.cmd") -Value $ShimContent -Force
    Set-Content -Path (Join-Path $ScoopShims "spc.bat") -Value $ShimContent -Force
    Write-Success "Synchronized scoop shims: tq.cmd, tq.bat, spc.cmd, spc.bat"
}

# 4. Add to User Environment PATH (No Administrator privileges required)
Write-Step "Configuring User PATH environment variable..."
$CurrentPath = [Environment]::GetEnvironmentVariable("Path", "User")
if (-not $CurrentPath) { $CurrentPath = "" }

$PathEntries = $CurrentPath -split ';' | Where-Object { $_ -ne "" }
if ($PathEntries -notcontains $TqBinDir) {
    $NewPath = if ($CurrentPath) { "$TqBinDir;$CurrentPath" } else { $TqBinDir }
    [Environment]::SetEnvironmentVariable("Path", $NewPath, "User")
    Write-Success "Added $TqBinDir to User PATH registry"
} else {
    Write-Success "$TqBinDir is already in User PATH registry"
}

# Unblock files and attempt exclusion registration
try {
    Get-ChildItem -Path $TqBinDir -Filter "*.exe" | ForEach-Object { Unblock-File $_.FullName -ErrorAction SilentlyContinue }
    Add-MpPreference -ExclusionPath $TqBinDir -ErrorAction SilentlyContinue
} catch {
    # Silently continue
}

# Update current session PATH immediately
if ($env:PATH -split ';' -notcontains $TqBinDir) {
    $env:PATH = "$TqBinDir;$env:PATH"
}

# 5. Run One-Command Bootstrap
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
Write-Host "  TQ CLI INSTALLATION COMPLETED SUCCESSFULLY" -ForegroundColor Green
Write-Host "==============================================================================" -ForegroundColor Green
Write-Host ""
Write-Host "  To get started, open a new shell or run immediately:" -ForegroundColor White
Write-Host "    tq tui              Launch Mission Control terminal dashboard" -ForegroundColor Cyan
Write-Host "    tq browser launch   Launch Antidetect Chromium browser sandbox" -ForegroundColor Cyan
Write-Host "    tq doctor           Check environment and system shims" -ForegroundColor Cyan
Write-Host "    tq status           Inspect unified services dashboard" -ForegroundColor Cyan
Write-Host "    tq bridge start     Start secure SOCKS5 mesh tunnel" -ForegroundColor Cyan
Write-Host ""
Write-Host "  Single Source of Truth Root: $TqRoot" -ForegroundColor Gray
Write-Host ""
