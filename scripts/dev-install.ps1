# Fast Local Dev Installation (Debug Mode)
$ErrorActionPreference = "Stop"

$sw = [System.Diagnostics.Stopwatch]::StartNew()
Write-Host "[1/2] Compiling debug binary (fast)..." -ForegroundColor Cyan

$cliDir = Resolve-Path (Join-Path $PSScriptRoot "..")
cargo build --manifest-path (Join-Path $cliDir "Cargo.toml") --bin tq

$binPath = Join-Path $cliDir "target\x86_64-pc-windows-gnu\debug\tq.exe"
if (-not (Test-Path $binPath)) {
    $binPath = Join-Path $cliDir "target\debug\tq.exe"
}

if (-not (Test-Path $binPath)) {
    throw "Build artifact not found at $binPath"
}

Write-Host "[2/2] Updating PATH locations..." -ForegroundColor Cyan

$destinations = @(
    "C:\Users\ndtu6\.tq\bin\tq.exe",
    "C:\Users\ndtu6\.tuquet\bin\tq.exe",
    "C:\Users\ndtu6\scoop\apps\rustup-msvc\current\.cargo\bin\tq.exe"
)

foreach ($dest in $destinations) {
    $parent = Split-Path $dest
    if (Test-Path $parent) {
        Copy-Item -Force $binPath $dest
        Write-Host "  -> Updated $dest" -ForegroundColor Green
    }
}

$sw.Stop()
Write-Host "Successfully installed local debug tq.exe in $([math]::Round($sw.Elapsed.TotalSeconds, 2))s!" -ForegroundColor Green
