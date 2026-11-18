# Test fixture only; used by test-auto-update.ps1 on a disposable Windows machine. Never run it on a personal PC.
# Test fixture only. A separate process releases every Windows Installer COM reference before
# msiexec reads the database; a closed View RCW can otherwise retain the database's file lock.
param([Parameter(Mandatory=$true)][string] $Path)
$ErrorActionPreference = 'Stop'
$installer = New-Object -ComObject WindowsInstaller.Installer
$db = $installer.OpenDatabase((Resolve-Path $Path).Path, 1)
foreach ($statement in @(
    "UPDATE ``Property`` SET ``Value``='0.0.1' WHERE ``Property``='ProductVersion'",
    "UPDATE ``Property`` SET ``Value``='{$([guid]::NewGuid().ToString().ToUpperInvariant())}' WHERE ``Property``='ProductCode'"
)) {
    $view = $db.OpenView($statement)
    try { $view.Execute() } finally {
        $view.Close()
        [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($view)
    }
}
$summary = $db.SummaryInformation(1)
$summary.Property(9) = '{' + [guid]::NewGuid().ToString().ToUpperInvariant() + '}'
$summary.Persist()
$db.Commit()
[void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($summary)
[void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($db)
[void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($installer)
