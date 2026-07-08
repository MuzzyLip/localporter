Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Invoke-NativeCommand {
    param(
        [Parameter(Mandatory = $true)]
        [string]$FilePath,

        [string[]]$ArgumentList = @(),

        [switch]$CaptureOutput
    )

    if ($CaptureOutput) {
        $output = & $FilePath @ArgumentList 2>&1
        $exitCode = $LASTEXITCODE

        if ($exitCode -ne 0) {
            $message = "Command failed with exit code {0}: {1} {2}" -f $exitCode, $FilePath, ($ArgumentList -join ' ')
            $details = (($output | Out-String).TrimEnd())

            if ([string]::IsNullOrWhiteSpace($details)) {
                throw $message
            }

            throw "$message`n$details"
        }

        return $output
    }

    & $FilePath @ArgumentList
    $exitCode = $LASTEXITCODE

    if ($exitCode -ne 0) {
        throw ("Command failed with exit code {0}: {1} {2}" -f $exitCode, $FilePath, ($ArgumentList -join ' '))
    }
}

function Assert-CommandAvailable {
    param(
        [Parameter(Mandatory = $true)]
        [string]$CommandName,

        [Parameter(Mandatory = $true)]
        [string]$InstallHint
    )

    if (-not (Get-Command $CommandName -ErrorAction SilentlyContinue)) {
        throw "$CommandName was not found in PATH. $InstallHint"
    }
}

function Assert-WixExtensionInstalled {
    param(
        [Parameter(Mandatory = $true)]
        [string]$ExtensionId,

        [Parameter(Mandatory = $true)]
        [string]$InstallHint
    )

    $extensions = Invoke-NativeCommand -FilePath "wix" -ArgumentList @(
        "extension",
        "list"
    ) -CaptureOutput

    $normalizedExtensionId = $ExtensionId.Trim()
    $isInstalled = $extensions | Where-Object {
        ([string]$_).Trim().StartsWith($normalizedExtensionId, [System.StringComparison]::OrdinalIgnoreCase)
    }

    if (-not $isInstalled) {
        throw "$ExtensionId is not installed in the WiX extension cache. $InstallHint"
    }
}

function Assert-PathExists {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path,

        [Parameter(Mandatory = $true)]
        [string]$Description
    )

    if (-not (Test-Path -LiteralPath $Path)) {
        throw "$Description was not found: $Path"
    }
}

function Remove-PathIfExists {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path
    )

    if (Test-Path -LiteralPath $Path) {
        Remove-Item -LiteralPath $Path -Recurse -Force
    }
}

function New-CleanDirectory {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path
    )

    Remove-PathIfExists -Path $Path
    New-Item -ItemType Directory -Path $Path -Force | Out-Null
}

function Get-TemporaryArtifactPath {
    param(
        [Parameter(Mandatory = $true)]
        [string]$FinalPath
    )

    $directory = Split-Path -Parent $FinalPath
    $fileNameWithoutExtension = [System.IO.Path]::GetFileNameWithoutExtension($FinalPath)
    $extension = [System.IO.Path]::GetExtension($FinalPath)
    $temporaryName = "{0}-{1}.tmp{2}" -f $fileNameWithoutExtension, ([guid]::NewGuid().ToString("N")), $extension

    return Join-Path $directory $temporaryName
}

function Get-CargoMetadata {
    param(
        [Parameter(Mandatory = $true)]
        [string]$CargoTomlPath
    )

    $rawMetadata = Invoke-NativeCommand -FilePath "cargo" -ArgumentList @(
        "metadata",
        "--format-version",
        "1",
        "--no-deps",
        "--manifest-path",
        $CargoTomlPath
    ) -CaptureOutput

    $json = ($rawMetadata | Out-String)
    return $json | ConvertFrom-Json
}

function Get-PackageVersion {
    param(
        [Parameter(Mandatory = $true)]
        $Metadata,

        [Parameter(Mandatory = $true)]
        [string]$PackageName
    )

    $package = $Metadata.packages | Where-Object { $_.name -eq $PackageName } | Select-Object -First 1

    if (-not $package) {
        throw "Package '$PackageName' was not found in cargo metadata."
    }

    return [string]$package.version
}

function ConvertTo-MsiVersion {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Version
    )

    $match = [regex]::Match($Version, '^(?<major>\d+)\.(?<minor>\d+)\.(?<patch>\d+)')

    if (-not $match.Success) {
        throw "Version '$Version' is not compatible with MSI version requirements. Expected a SemVer prefix like 1.2.3."
    }

    $major = [int]$match.Groups["major"].Value
    $minor = [int]$match.Groups["minor"].Value
    $patch = [int]$match.Groups["patch"].Value

    if ($major -gt 255 -or $minor -gt 255 -or $patch -gt 65535) {
        throw "Version '$Version' exceeds MSI numeric limits (major/minor <= 255, patch <= 65535)."
    }

    return "{0}.{1}.{2}" -f $major, $minor, $patch
}

function Assert-SupportedWindowsTargetTriple {
    param(
        [Parameter(Mandatory = $true)]
        [string]$TargetTriple
    )

    if ($TargetTriple -ne "x86_64-pc-windows-msvc") {
        throw "Unsupported Windows target triple '$TargetTriple'. These packaging scripts currently support only x86_64-pc-windows-msvc."
    }
}
