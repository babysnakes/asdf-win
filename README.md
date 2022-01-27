# Poor Man's ASDF for Windows.

This tool aims to be a way of managing multiple versions of standalone and
(later) portable applications for windows.

## Current Status

A very early stage. It is possible to run standalone tools but you have to
download them manually and put them in the correct path.

## Installation (Windows only)

Everything is currently manual. After building the project (`cargo build
--release`) perform the following steps:

### Create Required Directories (if it's the first time)

```powershell
mkdir $HOME\.asdfw\shims
mkdir $HOME\.asdfw\bin
mkdir $HOME\.asdfw\lib
mkdir $HOME\.asdfw\installs
mkdir $HOME\.asdfw\logs
```

### Copy Files

```powershell
cp target\release\shim.exe $HOME\.asdfw\lib\
cp target\release\asdfw.exe $HOME\.asdfw\bin\
```

Optionally install the *powershell* module - it will have temporary helpers
until they are implemented in `asdfw.exe`:

* Get your module path: `$env:PSModulePath`
* Copy `powershell\ASDFHelpers` into one of the directories in the module path
  (preferably in your `$HOME`)
* run `Import-Module ASDFHelpers` (either manually or add it to your powershell initialization)

Finally, you need to add the `shims` and `bin` directories to your path. This is
best done [using system properties][addenv]. It might require you to logout, restart shell, etc...

[addenv]: https://www.architectryan.com/2018/03/17/add-to-the-path-on-windows-10/

## Usage

Every tool (one or more standalone executables) needs to be installed under
`$HOME\.asdfw\installs\<TOOLS_NAME>\<TOOLS_VERSION>\bin`. To install the current
version of [hugo][] create a `$HOME\.asdfw\installs\hugo\0.92.1\bin` directory
and copy the downloaded `hugo.exe` file into this directory. You can probably
also download *hugo-extended* and copy it as `hugo-extended.exe` into the same
directory. The `New-ASDFInstallDir <TOOLS_NAME> <TOOLS_VERSION>` powershell
function (included in the module) will help you create the required installation
path.

After each tool you install do the following:

* For each of the executables installed in the previous step copy
  `$HOME\.asdfw\lib\shims.exe` to `$HOME\.asdfw\shims\<EXECUTABLE>`. The
  name should be identical to the executable.
* Run `asdfw reshim`.
* Manually edit `$HOME/.tool-versions` and add a line specifying the global
  version for this tool. The format must match "`<TOOL> <VERSION>`" (exactly one
  space between and no trailing spaces).
* If you want to use a different version of the tool when running in specific
  directory, create a `.tool-versions` file in set directory and configure the
  required version using the same format as above.
* When deleting a tool, remember to delete the shim as well.

[hugo]: https://gohugo.io
