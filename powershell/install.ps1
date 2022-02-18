<#
.Description
Installer for ASDF for windows.
#>

param (
    # Enable debugging output
    [switch] $debug
)

$requiredDirs = @(
    "bin",
    "installs",
    "lib",
    "logs",
    "shims"
)

$ASDFWDir = Join-Path $HOME ".asdfw"

function conditionalyCreateDir {
    param (
        [string] $directory
    )
    
    if (Test-Path -Path $directory) {
        # nothing needs to be done
        Write-Debug "$directory exists, no need to create"
    } else {
        Write-Debug "Creating directory $directory"
        mkdir $directory
    }
}

function createOrUpdateDirs {
    conditionalyCreateDir $ASDFWDir
    $requiredDirs.ForEach({
        $f = Join-Path $ASDFWDir $PSItem
        conditionalyCreateDir $f
    })
}

function installBinaries {
    $binDir = Join-Path $ASDFWDir "bin"
    $libDir = Join-Path $ASDFWDir "lib"
    $asdfwFile = Join-Path $PSScriptRoot "asdfw.exe"
    $shimFile = Join-Path $PSScriptRoot "shim.exe"
    Write-Debug "copying .\asdfw.exe $binDir"
    Copy-Item $asdfwFile $binDir
    Write-Debug "copying .\shim.exe $libDir"
    Copy-Item $shimFile $libDir
}

function run {
    if ($debug) {
        $DebugPreference = "Continue"
    }

    Write-Host "Installing to $HOME/.asdfw ..."

    createOrUpdateDirs
    installBinaries
    
    Write-Host " " -NoNewline -ForegroundColor Green
    Write-Host "Installation Complete"
    Write-Host " " -NoNewline -ForegroundColor Green
    Write-Host "Please checkout the readme for further installation and usage instructions:"
    Write-Host "  -> " -NoNewline -ForegroundColor Green
    Write-Host "https://github.com/babysnakes/asdf-win"
    Write-Host ""
    Read-Host "Press ENTER to Finish"
}

run
