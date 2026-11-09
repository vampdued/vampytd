# VampYTD Installer & Setup Script (Rust)
param(
    [switch]$Uninstall,
    [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"

$HostName = "com.vampytd.bridge"
$InstallDir = Join-Path $env:LOCALAPPDATA "VampYTD"
$ManifestPath = Join-Path $InstallDir "$HostName.json"
$ExePath = Join-Path $InstallDir "vampytd.exe"

$BrowserRegKeys = @(
    "HKCU:\Software\Google\Chrome\NativeMessagingHosts\$HostName",
    "HKCU:\Software\Microsoft\Edge\NativeMessagingHosts\$HostName",
    "HKCU:\Software\Chromium\NativeMessagingHosts\$HostName",
    "HKCU:\Software\BraveSoftware\Brave-Browser\NativeMessagingHosts\$HostName",
    "HKCU:\Software\Vivaldi\NativeMessagingHosts\$HostName"
)

if ($Uninstall) {
    Write-Host "Uninstalling VampYTD..." -ForegroundColor Yellow

    foreach ($key in $BrowserRegKeys) {
        if (Test-Path $key) {
            Remove-Item -Path $key -Recurse -Force -ErrorAction SilentlyContinue
            Write-Host "  Removed registry key: $key" -ForegroundColor DarkGray
        }
    }

    $aumidReg = "HKCU:\Software\Classes\AppUserModelId\VampYTD"
    if (Test-Path $aumidReg) {
        Remove-Item -Path $aumidReg -Recurse -Force -ErrorAction SilentlyContinue
        Write-Host "  Removed notification registry key: $aumidReg" -ForegroundColor DarkGray
    }

    Write-Host "VampYTD has been successfully uninstalled." -ForegroundColor Green
    exit 0
}

Write-Host "==> Installing VampYTD (Rust Unified Binary)..." -ForegroundColor Cyan

$LocalExe = Join-Path $PSScriptRoot "vampytd.exe"
$TargetExe = Join-Path $PSScriptRoot "target\release\vampytd.exe"

# If pre-built vampytd.exe is present (release package), skip cargo build automatically
$isPrebuilt = Test-Path $LocalExe
if ($isPrebuilt) {
    $BuiltExe = $LocalExe
} else {
    $BuiltExe = $TargetExe
}

# 1. Build release binary if not skipped and not prebuilt
if (-not $SkipBuild -and -not $isPrebuilt) {
    $cargoFound = Get-Command cargo -ErrorAction SilentlyContinue
    if ($cargoFound) {
        $cargoCmd = $cargoFound.Source
    } elseif (Test-Path "$env:USERPROFILE\.cargo\bin\cargo.exe") {
        $cargoCmd = "$env:USERPROFILE\.cargo\bin\cargo.exe"
    } else {
        $cargoCmd = $null
    }

    if (-not $cargoCmd) {
        Write-Error "Cargo not found. Please install Rust from https://rustup.rs"
    }

    Write-Host "Building release binary..." -ForegroundColor Cyan
    & $cargoCmd build --release
    if ($LASTEXITCODE -ne 0) {
        Write-Error "Cargo build failed"
    }
}

if (-not (Test-Path $BuiltExe)) {
    Write-Error "Built binary not found at $BuiltExe"
}

# 2. Deploy binary to LOCALAPPDATA
if (-not (Test-Path $InstallDir)) {
    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
}

Copy-Item -Path $BuiltExe -Destination $ExePath -Force
Write-Host "  Copied binary to: $ExePath" -ForegroundColor Green

# 2b. Deploy extension to LOCALAPPDATA for permanent hosting
$SourceExtDir = Join-Path $PSScriptRoot "extension"
$InstalledExtDir = Join-Path $InstallDir "extension"
if (Test-Path $SourceExtDir) {
    Copy-Item -Path $SourceExtDir -Destination $InstallDir -Recurse -Force
    Write-Host "  Copied extension to: $InstalledExtDir" -ForegroundColor Green
}

# 2c. Deploy brand assets (icons) for notifications and shortcuts
$SourceAssetsDir = Join-Path $PSScriptRoot "assets"
if (Test-Path $SourceAssetsDir) {
    Copy-Item -Path (Join-Path $SourceAssetsDir "*") -Destination $InstallDir -Force
    Write-Host "  Copied brand assets to: $InstallDir" -ForegroundColor Green
}

# 3. Write Chrome Native Messaging Manifest
$ManifestContent = @{
    name = $HostName
    description = "VampYTD Native Messaging Host (Rust)"
    path = $ExePath
    type = "stdio"
    allowed_origins = @(
        "chrome-extension://jjacbochmpbgpfpbfclmileocddkncgd/"
    )
} | ConvertTo-Json -Depth 4

Set-Content -Path $ManifestPath -Value $ManifestContent -Encoding UTF8
Write-Host "  Generated manifest: $ManifestPath" -ForegroundColor Green

# 4. Register in Windows Registry for browsers
foreach ($key in $BrowserRegKeys) {
    $parent = Split-Path $key -Parent
    if (-not (Test-Path $parent)) {
        New-Item -ItemType Directory -Path $parent -Force | Out-Null
    }
    New-Item -Path $key -Force | Out-Null
    Set-ItemProperty -Path $key -Name "(default)" -Value $ManifestPath -Force
    Write-Host "  Registered in: $key" -ForegroundColor DarkGray
}

# 5. Register AppUserModelId for Windows Notifications
$aumidReg = "HKCU:\Software\Classes\AppUserModelId\VampYTD"
if (-not (Test-Path $aumidReg)) {
    New-Item -Path $aumidReg -Force | Out-Null
}
Set-ItemProperty -Path $aumidReg -Name "DisplayName" -Value "VampYTD" -Force
Set-ItemProperty -Path $aumidReg -Name "IconUri" -Value $ExePath -Force
Write-Host "  Registered AppUserModelId for Windows Notifications: $aumidReg" -ForegroundColor Green

# 6. Add to User PATH if missing
$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($userPath -notlike "*$InstallDir*") {
    [Environment]::SetEnvironmentVariable("Path", "$userPath;$InstallDir", "User")
    Write-Host "  Added $InstallDir to User PATH." -ForegroundColor Green
}

Write-Host "==> VampYTD installation complete! Restart your browser to connect." -ForegroundColor Green

$TargetExtPath = if ($InstalledExtDir -and (Test-Path $InstalledExtDir)) { $InstalledExtDir } elseif ($SourceExtDir -and (Test-Path $SourceExtDir)) { $SourceExtDir } else { $null }
if ($TargetExtPath) {
    Write-Host "`nChrome Extension:" -ForegroundColor Cyan
    Write-Host "  To load in Chrome / Brave / Edge:" -ForegroundColor White
    Write-Host "  1. Navigate to chrome://extensions (or edge://extensions)" -ForegroundColor DarkGray
    Write-Host "  2. Enable 'Developer mode' (top-right toggle)" -ForegroundColor DarkGray
    Write-Host "  3. Click 'Load unpacked' and select:" -ForegroundColor DarkGray
    Write-Host "     $TargetExtPath" -ForegroundColor Yellow
}

