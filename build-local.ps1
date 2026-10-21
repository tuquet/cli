[CmdletBinding()]
param(
    [switch]$DebugBuild,
    [switch]$Clean
)

$PSScript = Join-Path $PSScriptRoot "..\scripts\build-local.ps1"
if (Test-Path $PSScript) {
    & $PSScript @PSBoundParameters
} else {
    Write-Error "Could not find root build script at $PSScript"
}
