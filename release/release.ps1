
param (
    [switch] $Debug,
    [switch] $Clean,
    [string] $ReleaseVersion
)

# Exit on all posh errors
$ErrorActionPreference = 'Stop'

$rootDir = (Get-Item $PSScriptRoot).Parent
$releaseDir = Join-Path $rootDir target release
$moduleDir = Join-Path $rootDir powershell

function failIfCommandFailed {
    param (
        [Parameter(Mandatory = $true)]
        [string] $error
    )

    if ($LASTEXITCODE -ne 0) {
        Write-Error $error -ErrorAction Stop
    }
}

function buildRelease {
    if ($Clean) {
        cargo clean
    }
    Write-Debug "Running tests"
    cargo test
    failIfCommandFailed "Tests failed"
    Write-Debug "Building release"
    cargo build --release
    failIfCommandFailed "Build failed"
}

function copyFiles {
    param (
        [Parameter(Mandatory = $true)]
        [string] $targetDir
    )
    if (Test-Path $targetDir) {
        Remove-Item -Recurse $targetDir
    }
    mkdir $targetDir | Out-Null

    Write-Debug "Copying executables to release dir"
    Copy-Item ${releaseDir}\*.exe $targetDir
    Write-Debug "Copying powershell files to release dir"
    Copy-Item -Recurse ${moduleDir}\* $targetDir
}

function run {
    if ($debug) {
        $DebugPreference = "Continue"
        Write-Debug "Setting DEBUG mode"
    }

    if ([string]::IsNullOrEmpty($ReleaseVersion)) {
        Write-Debug "No release version provided. Checking if we're on exact git tag ..."
        $ReleaseVersion = git describe --exact-match
        failIfCommandFailed "Release version not supplied nor can be extracted from git tag"
    }

    $targetDir = Join-Path $rootDir target "asdf-win-$ReleaseVersion"
    $targetZip = "${targetDir}.zip"

    if (Test-Path $targetZip) {
        Write-Debug "Deleting old $targetZip"
        Remove-Item $targetZip
    }

    buildRelease
    copyFiles $targetDir
    Compress-Archive -Path $targetDir -CompressionLevel Fastest -DestinationPath $targetZip

    if ($Clean) {
        Remove-Item -Recurse $targetDir
    }

    Write-Host ""
    Write-Host " " -NoNewline -ForegroundColor Green
    Write-Host "Build Done"
    Write-Host " " -NoNewline -ForegroundColor Green
    Write-Host -NoNewline "Output file is "
    Write-Host "$targetZip" -ForegroundColor Green
}

run
