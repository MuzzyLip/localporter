param(
    [string]$TargetTriple = "x86_64-pc-windows-msvc"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

. (Join-Path $PSScriptRoot "windows-build-common.ps1")

$RootDir = [System.IO.Path]::GetFullPath((Split-Path -Parent $PSScriptRoot))
$CargoToml = Join-Path $RootDir "Cargo.toml"
$BinaryName = "localporter-app.exe"
$PackageName = "localporter-app"
$IconPath = Join-Path $RootDir "crates\localporter-ui\assets\app-icon.ico"

Assert-CommandAvailable -CommandName "cargo" -InstallHint "Install Rust and ensure cargo is available before running this script."
Assert-PathExists -Path $CargoToml -Description "Workspace Cargo.toml"
Assert-PathExists -Path $IconPath -Description "Application icon"
Assert-SupportedWindowsTargetTriple -TargetTriple $TargetTriple

$Metadata = Get-CargoMetadata -CargoTomlPath $CargoToml
$Version = Get-PackageVersion -Metadata $Metadata -PackageName $PackageName
$ArtifactVersion = if ([string]::IsNullOrWhiteSpace($env:LOCALPORTER_RELEASE_VERSION)) {
    $Version
}
else {
    [string]$env:LOCALPORTER_RELEASE_VERSION
}
$MsiVersion = ConvertTo-MsiVersion -Version $Version
$TargetRootDir = [System.IO.Path]::GetFullPath([string]$Metadata.target_directory)
$TargetDir = Join-Path $TargetRootDir $TargetTriple
$ReleaseDir = Join-Path $TargetDir "release"
$BundleDir = Join-Path $ReleaseDir "bundle\windows"
$StageDir = Join-Path $BundleDir "app"
$BuiltBinaryPath = Join-Path $ReleaseDir $BinaryName
$StagedBinaryPath = Join-Path $StageDir $BinaryName

Push-Location $RootDir
try {
    Invoke-NativeCommand -FilePath "cargo" -ArgumentList @(
        "build",
        "--locked",
        "--release",
        "--package",
        $PackageName,
        "--target",
        $TargetTriple,
        "--manifest-path",
        $CargoToml
    )
}
finally {
    Pop-Location
}

Assert-PathExists -Path $BuiltBinaryPath -Description "Built application binary"
New-CleanDirectory -Path $StageDir
Copy-Item -LiteralPath $BuiltBinaryPath -Destination $StagedBinaryPath -Force
Assert-PathExists -Path $StagedBinaryPath -Description "Staged application binary"

[pscustomobject]@{
    RootDir = $RootDir
    CargoToml = $CargoToml
    TargetTriple = $TargetTriple
    TargetRootDir = $TargetRootDir
    TargetDir = $TargetDir
    ReleaseDir = $ReleaseDir
    BundleDir = $BundleDir
    StageDir = $StageDir
    BinaryName = $BinaryName
    BinaryPath = $StagedBinaryPath
    BuiltBinaryPath = $BuiltBinaryPath
    IconPath = $IconPath
    Version = $Version
    ArtifactVersion = $ArtifactVersion
    MsiVersion = $MsiVersion
}
