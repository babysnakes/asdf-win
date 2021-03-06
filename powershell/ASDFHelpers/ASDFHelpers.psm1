$ASDFDir = Join-Path $HOME ".asdfw"
$ASDFInstallDir = Join-Path $ASDFDir "installs"

<#
.Description
Set-ASDFToolVersion Configures asdfw custom version for the requested tool. Use
it for temporarily override global or local tool version. Note, this does not
validate whether the tool / version is actually installed.
#>
function Set-ASDFToolVersion {
    param (
        # The tool to define version for
        [Parameter(Mandatory, Position = 0)]
        [ValidateNotNullOrEmpty()]
        [string] $ToolName,

        # The custom version to set
        [Parameter(Mandatory, Position = 1)]
        [ValidateNotNullOrEmpty()]
        [string] $Version
    )

    $tool = $ToolName.ToUpper()
    $envName = "ASDFW_${tool}_VERSION"
    Set-Item -Path Env:$envName -Value $Version
}

<#
.Description
Remove-ASDFToolVersion unsets the custom version set by Set-ASDFToolVersion.
#>
function Remove-ASDFToolVersion {
    param (
        # The tool to unset custom version for (it has no effect if not defined)
        [Parameter(Mandatory, Position = 0)]
        [ValidateNotNullOrEmpty()]
        [string] $ToolName
    )

    $tool = $ToolName.ToUpper()
    $envName = "ASDFW_${tool}_VERSION"
    Remove-Item -Path Env:$envName
}

<#
.Description
New-ASDFInstallDir creates the hierarchy for installing new asdfw tools. Follow
the output for where to copy the executables to.
#>
function New-ASDFInstallDir {
    param (
        # The tool you intend to install in this directory
        [Parameter(Mandatory, Position = 0)]
        [ValidateNotNullOrEmpty()]
        [string]$ToolName,

        # The version you intend to install in this directory
        [Parameter(Mandatory, Position = 1)]
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

Export-ModuleMember -Function New-ASDFInstallDir, Set-ASDFToolVersion
