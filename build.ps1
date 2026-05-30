<#
.SYNOPSIS
    qterm build & run script
.DESCRIPTION
    Compile and run the qterm terminal emulator.
    Usage:
      .\build.ps1           # debug build + run
      .\build.ps1 -Release  # release build + run
      .\build.ps1 -BuildOnly # build only, don't run
      .\build.ps1 -Clean    # clean build artifacts then build + run
#>

param(
    [switch]$Release,
    [switch]$BuildOnly,
    [switch]$Clean
)

$ErrorActionPreference = "Stop"
$ProjectDir = Split-Path -Parent $MyInvocation.MyCommand.Path

Push-Location $ProjectDir

try {
    if ($Clean) {
        Write-Host "[1/3] Cleaning..." -ForegroundColor Cyan
        cargo clean
    }

    $buildArgs = @("build")
    if ($Release) {
        $buildArgs += "--release"
        Write-Host "[build] Release mode" -ForegroundColor Green
    } else {
        Write-Host "[build] Debug mode" -ForegroundColor Yellow
    }

    Write-Host "[build] Compiling qterm..." -ForegroundColor Cyan
    & cargo @buildArgs
    if ($LASTEXITCODE -ne 0) {
        Write-Host "[error] Build failed." -ForegroundColor Red
        exit $LASTEXITCODE
    }

    if ($BuildOnly) {
        Write-Host "[done] Build succeeded." -ForegroundColor Green
        exit 0
    }

    $binary = if ($Release) { "target/release/qterm.exe" } else { "target/debug/qterm.exe" }

    Write-Host "[run] Launching qterm..." -ForegroundColor Cyan
    & ./$binary
}
finally {
    Pop-Location
}
