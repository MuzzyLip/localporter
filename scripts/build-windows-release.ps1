param(
    [string]$TargetTriple = "x86_64-pc-windows-msvc"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$BuildInfo = & (Join-Path $PSScriptRoot "build-windows-app.ps1") -TargetTriple $TargetTriple
$ZipArtifact = & (Join-Path $PSScriptRoot "build-windows-zip.ps1") -TargetTriple $TargetTriple -BuildInfo $BuildInfo
$MsiArtifact = & (Join-Path $PSScriptRoot "build-windows-msi.ps1") -TargetTriple $TargetTriple -BuildInfo $BuildInfo

[pscustomobject]@{
    Version = $BuildInfo.Version
    TargetTriple = $TargetTriple
    ZipPath = $ZipArtifact.ArtifactPath
    MsiPath = $MsiArtifact.ArtifactPath
}
