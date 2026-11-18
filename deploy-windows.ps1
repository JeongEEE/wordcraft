<#
.SYNOPSIS
  Build Word locally on Windows and upload to the shared GitHub release draft. No Actions runner.
.EXAMPLE
  .\deploy-windows.ps1 -Version 0.18.2
  .\deploy-windows.ps1 -Version 0.18.2 -Draft
  .\deploy-windows.ps1 -DryRun
#>
param(
    [string] $Version,
    [switch] $Draft,
    [switch] $DryRun,
    [switch] $Finalize,
    [string] $KeyFile
)
$ErrorActionPreference = 'Stop'
$python = Get-Command python -ErrorAction SilentlyContinue
$launcher = Get-Command py -ErrorAction SilentlyContinue
$pythonArgs = @()
if ($launcher) {
    $executable = $launcher.Source
    $pythonArgs += '-3'
} elseif ($python) {
    $executable = $python.Source
} else {
    throw 'Python 3.11+ is required. Install Python and enable its PATH or py launcher.'
}
& $executable @pythonArgs -c 'import sys; sys.exit(0 if sys.version_info >= (3, 11) else 1)'
if ($LASTEXITCODE -ne 0) { throw 'Python 3.11+ is required.' }
$pythonArgs += @((Join-Path $PSScriptRoot 'packaging/deploy.py'), '--platform', 'windows')
if ($Version) { $pythonArgs += $Version }
if ($Draft) { $pythonArgs += '--draft' }
if ($DryRun) { $pythonArgs += '--dry-run' }
if ($Finalize) { $pythonArgs += '--finalize' }
if ($KeyFile) { $pythonArgs += @('--key-file', $KeyFile) }
& $executable @pythonArgs
exit $LASTEXITCODE
