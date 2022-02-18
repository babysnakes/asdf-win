# Poor Man's ASDF for Windows.

This tool aims to be a way of managing multiple versions of standalone and
(later) portable applications for windows.

## Current Status

A very early stage. It's starting to be usable (in an alpha quality and features
way :slightly_smiling_face:). You can manually place standalone binaries in a
specific path (see [usage](#usage) below), generate shims and configure global,
local and custom version using tools.

## Installation (Windows only)

Once the archive is downloaded and extracted, cd into the directory using
*powershell* terminal and run (alternatively right-click the `install.ps1` file
and select *Run with PowerShell*):


```powershell
# Optionally use -Debug to see exactly what the script is doing
.\install.ps1
```

This will create (if required) a few directories under `$HOME/.asdfw` and
copy/update the executables. Nothing is modified outside of `$HOME/.asdfw`.

Once it completes you can optionally install the *powershell* module (*highly
recommended*) - it includes temporary helpers until they are implemented in
`asdfw.exe`:

* Get your module path: `$env:PSModulePath`
* Copy the `ASDFHelpers` directory into one of the directories in the module
  path (preferably in your `$HOME`). This should be enough to enable the
  provided helpers.
* If you want basic tab-completion, add the following to your *powershell*
  initialization script: `asdfw.exe completion | Out-String | Invoke-Expression`.

Finally, as a one-time step (You only need to perform this on first install) you
need to add the `$HOME\.asdfw\shims` and `$HOME\.asdfw\bin` directories to your
path. This is best done [using system properties][addenv]. It might require you
to logout, restart shell, etc...

[addenv]: https://www.architectryan.com/2018/03/17/add-to-the-path-on-windows-10/

## Usage

The idea behind this utility is to be able to install multiple versions of the
same CLI application and configure the version to use globally, per directory or
using environment variable. This is achieved by installing all the tools in
specific hierarchy and using shims to launch the configured version of the
application. Note that only standalone executables (and later specific portable
applications) are supported.

The following features are currently supported:

* Only supports `*.exe` executables, Currently no other types are supported
  (more types will be added shortly).
* Manually install / uninstall stand alone binaries (see
  [installations](#installation-windows-only)).
* Create shims for all installed applications/versions.
* Configure Local / Global / Ad Hoc version for each tool.
* Query the full path of the configured tool / version for current directory.
* Basic tab completion.
* Descriptive errors.

All operations are performed using either `asdfw.exe` or using one of the
*Powershell* helpers (if installed). To get help on `asdfw.exe` run `asdfw.exe
--help`. For help on the various *powershell* helpers run `Get-Help <Command>`.

### Install New Tool/Version

Currently tool installation is manual. All executables has to installed under
`$HOME/.asdfw/installs/<TOOLNAME>/<VERSION>/bin`. The provided *powershell*
module contains a helper to create this hierarchy. If you installed it you can
run:

```powershell
New-ASDFInstallDir <TOOLNAME> <VERSION>
```

This will create the correct hierarchy and print instructions to the screen.
Don't forget to run `asdfw reshim` after each tool you install. You might also
want to configure the global version (see below).

### Uninstall Tools

To uninstall tool you can delete either the specific version folder (e.g.
`$HOME\.asdfw\installs\<TOOL>\<VERSION>`). To delete all versions of the tool
just delete the tool directory.

### Creating Shims

After each new tool you install you should run:

```powershell
asdfw reshim
```

You can optionally add `--cleanup` flag to delete invalid shims (e.g. if you
deleted the tool).

### Configure Versions

There are three types of variables:

#### *Global* Version

This is the default version to use when calling the tool. You should always
define global version:

```powershell
asdfw.exe global <TOOL> <VERSION>
```

#### *Directory Local* Version

This is the version to use when running the tool inside a specific directory (no
matter how deep inside this directory you are when you call the tool):

```powershell
asdfw.exe local <TOOL> <VERSION>
```

#### *Current Shell* Version

Sometimes you want to temporarily try a different version. For that you need to
define an environment variable in the form of `ASDFW_<TOOL_UPPER_CASE>_VERSION`. So in order to temporarily use [hugo][] version 1.2 use the following command:

```powershell
$Env:ASDFW_HUGO_VERSION = 1.2
```

However, we provide a *Powershell* helper to configure it:

```powershell
Set-ASDFToolVersion <TOOL> <VERSION>

# so the example above would be
Set-ASDFToolVersion hugo 1.2
```

You can also unset the custom version using the helper:

```powershell
Remove-ASDFToolVersion <TOOL>

# or without the helper
Remove-Item $Env.ASDFW_<TOOL_UPPER_CASE>_VERSION
```

### Query the Configured Version

You can always get the configured version for you current working directory using:

```powershell
asdfw.exe which <COMMAND>
```

[hugo]: https://gohugo.io
