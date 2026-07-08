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
$BundleDir = $BuildInfo.BundleDir
$StageDir = $BuildInfo.StageDir
$BinaryName = $BuildInfo.BinaryName
$ArtifactVersion = $BuildInfo.ArtifactVersion
$ArchivePath = Join-Path $BundleDir ("LocalPorter-{0}-windows-x64.zip" -f $ArtifactVersion)
$TemporaryArchivePath = Get-TemporaryArtifactPath -FinalPath $ArchivePath

New-Item -ItemType Directory -Path $BundleDir -Force | Out-Null
Remove-PathIfExists -Path $ArchivePath
Remove-PathIfExists -Path $TemporaryArchivePath

$locationPushed = $false

try {
    Push-Location $StageDir
    $locationPushed = $true
    Compress-Archive -Path $BinaryName -DestinationPath $TemporaryArchivePath -CompressionLevel Optimal
}
catch {
    Remove-PathIfExists -Path $TemporaryArchivePath
    throw
}
finally {
    if ($locationPushed) {
        Pop-Location
    }
}

Assert-PathExists -Path $TemporaryArchivePath -Description "Temporary ZIP archive"
Move-Item -LiteralPath $TemporaryArchivePath -Destination $ArchivePath -Force
Assert-PathExists -Path $ArchivePath -Description "Windows ZIP archive"

Write-Host "Built Windows ZIP:"
Write-Host "  $ArchivePath"

[pscustomobject]@{
    ArtifactType = "zip"
    ArtifactPath = $ArchivePath
    Version = $ArtifactVersion
    TargetTriple = $TargetTriple
}
