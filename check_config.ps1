Stop-Process -Name qterm -Force -ErrorAction SilentlyContinue
Start-Sleep -Seconds 2
$cfgPath = Join-Path $env:APPDATA "qterm\config.ini"
if (Test-Path $cfgPath) {
    Write-Host "Config file found:"
    Get-Content $cfgPath
} else {
    Write-Host "CONFIG FILE NOT FOUND at: $cfgPath"
    $dir = Join-Path $env:APPDATA "qterm"
    if (Test-Path $dir) {
        Write-Host "Directory exists: $dir"
        Get-ChildItem $dir
    } else {
        Write-Host "Directory does NOT exist: $dir"
    }
}
