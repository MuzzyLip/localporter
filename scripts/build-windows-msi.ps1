param(
    [string]$TargetTriple = "x86_64-pc-windows-msvc",
    [psobject]$BuildInfo
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

. (Join-Path $PSScriptRoot "windows-build-common.ps1")

if (-not $BuildInfo) {
    $BuildInfo = & (Join-Path $PSScriptRoot "build-windows-app.ps1") -TargetTriple $TargetTriple
}
$RootDir = $BuildInfo.RootDir
$BundleDir = $BuildInfo.BundleDir
$BinaryPath = $BuildInfo.BinaryPath
$IconPath = $BuildInfo.IconPath
$ArtifactVersion = $BuildInfo.ArtifactVersion
$MsiVersion = $BuildInfo.MsiVersion
$WixSource = Join-Path $RootDir "packaging\windows\localporter.wxs"
$MsiPath = Join-Path $BundleDir ("LocalPorter-{0}-windows-x64.msi" -f $ArtifactVersion)
$TemporaryMsiPath = Get-TemporaryArtifactPath -FinalPath $MsiPath

Assert-CommandAvailable -CommandName "wix" -InstallHint "Install WiX v4 and ensure the 'wix' CLI is in PATH before building the MSI."
Assert-PathExists -Path $WixSource -Description "WiX source file"
Assert-PathExists -Path $BinaryPath -Description "Staged application binary"
Assert-PathExists -Path $IconPath -Description "Application icon"

New-Item -ItemType Directory -Force -Path $BundleDir | Out-Null
Remove-PathIfExists -Path $MsiPath
Remove-PathIfExists -Path $TemporaryMsiPath

try {
    Invoke-NativeCommand -FilePath "wix" -ArgumentList @(
        "build",
        $WixSource,
        "-arch",
        "x64",
        "-d",
        "ProductVersion=$MsiVersion",
        "-d",
        "AppExePath=$BinaryPath",
        "-d",
        "AppIconPath=$IconPath",
        "-o",
        $TemporaryMsiPath
    )
}
catch {
    Remove-PathIfExists -Path $TemporaryMsiPath
    throw
}

Assert-PathExists -Path $TemporaryMsiPath -Description "Temporary MSI package"
Move-Item -LiteralPath $TemporaryMsiPath -Destination $MsiPath -Force
Assert-PathExists -Path $MsiPath -Description "Windows MSI package"

Write-Host "Built Windows MSI:"
Write-Host "  $MsiPath"

[pscustomobject]@{
    ArtifactType = "msi"
    ArtifactPath = $MsiPath
    Version = $ArtifactVersion
    MsiVersion = $MsiVersion
    TargetTriple = $TargetTriple
}
