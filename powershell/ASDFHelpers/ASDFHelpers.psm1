$ASDFDir = Join-Path $HOME ".asdfw"
$ASDFInstallDir = Join-Path $ASDFDir "installs"

function New-ASDFInstallDir {
    param (
        [Parameter(Mandatory,Position=0)]
        [ValidateNotNullOrEmpty()]
        [string]$ToolName,

        [Parameter(Mandatory,Position=1)]
        [ValidateNotNullOrEmpty()]
        [string]$ToolVersion
    )

    $dirName = Join-Path $ASDFInstallDir $ToolName $ToolVersion "bin"
    mkdir $dirName | Out-Null
    Write-Host " " -NoNewline -ForegroundColor Green
    Write-Host "Successfully created directory. Download and save the executables into '${dirName}'."
    Write-Host ""
    Write-Host "* " -NoNewline -ForegroundColor Yellow
    Write-Host "Don't forget to:"
    Write-Host "  - " -NoNewline -ForegroundColor Yellow
    Write-Host "ran 'asdfw reshim'"
    Write-Host "  - " -NoNewline -ForegroundColor Yellow
    Write-Host "update ${HOME}\.tool-versions ..."
}

Export-ModuleMember -Function New-ASDFInstallDir
