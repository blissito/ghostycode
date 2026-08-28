[CmdletBinding()]
param(
    [switch] $AllowUserPathMutation
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

if ([System.Environment]::OSVersion.Platform -ne [System.PlatformID]::Win32NT) {
    throw 'The installer PATH regression only supports Windows.'
}
if (-not $AllowUserPathMutation) {
    throw 'Pass -AllowUserPathMutation to confirm this test may temporarily replace the current-user PATH.'
}

function Get-RawUserPath {
    param(
        [Parameter(Mandatory = $true)]
        [Microsoft.Win32.RegistryKey] $EnvironmentKey
    )

    if (-not (@($EnvironmentKey.GetValueNames()) -contains 'Path')) {
        return $null
    }

    return [string] $EnvironmentKey.GetValue(
        'Path',
        $null,
        [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames
    )
}

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$testRoot = Join-Path `
    ([System.IO.Path]::GetTempPath()) `
    "ghosty-installer-path-$PID-$([guid]::NewGuid().ToString('N'))"
$stageRoot = Join-Path $testRoot 'source'
$stageInstallerDir = Join-Path $stageRoot 'scripts\installer'
$installDir = Join-Path $testRoot 'installed'
$programsDir = [Environment]::GetFolderPath([Environment+SpecialFolder]::Programs)
$shortcutDir = Join-Path $programsDir 'GhostyCode'
$shortcut = Join-Path $shortcutDir 'GhostyCode.lnk'
$environmentKey = [Microsoft.Win32.Registry]::CurrentUser.CreateSubKey('Environment')
if ($null -eq $environmentKey) {
    throw 'Could not open or create the current-user Environment registry key.'
}

$originalPathExists = @($environmentKey.GetValueNames()) -contains 'Path'
if ($originalPathExists) {
    $originalPath = Get-RawUserPath -EnvironmentKey $environmentKey
    $originalPathKind = $environmentKey.GetValueKind('Path')
}
else {
    $originalPath = $null
    $originalPathKind = [Microsoft.Win32.RegistryValueKind]::ExpandString
}

try {
    [void] (New-Item -ItemType Directory -Path $stageInstallerDir -Force)
    Copy-Item -LiteralPath (Join-Path $repoRoot 'LICENSE') -Destination (Join-Path $stageRoot 'LICENSE')
    Copy-Item `
        -LiteralPath (Join-Path $PSScriptRoot 'ghosty.nsi') `
        -Destination (Join-Path $stageInstallerDir 'ghosty.nsi')
    Copy-Item `
        -LiteralPath (Join-Path $PSScriptRoot 'update-user-path.ps1') `
        -Destination (Join-Path $stageInstallerDir 'update-user-path.ps1')
    Copy-Item `
        -LiteralPath (Join-Path $PSScriptRoot 'ghosty.bat') `
        -Destination (Join-Path $stageInstallerDir 'ghosty.bat')

    foreach ($binary in @('ghosty.exe', 'ghosty-tui.exe', 'ghosty-tui.exe')) {
        [System.IO.File]::WriteAllBytes(
            (Join-Path $stageInstallerDir $binary),
            [byte[]] @(0x4d, 0x5a)
        )
    }

    $makensisCandidates = @(
        "${env:ProgramFiles(x86)}\NSIS\makensis.exe",
        "$env:ProgramFiles\NSIS\makensis.exe"
    )
    $makensis = $makensisCandidates |
        Where-Object { Test-Path -LiteralPath $_ -PathType Leaf } |
        Select-Object -First 1
    if ([string]::IsNullOrWhiteSpace($makensis)) {
        throw 'makensis.exe was not found after installing NSIS.'
    }

    Push-Location $stageInstallerDir
    try {
        & $makensis '/DVERSION=0.0.0-test' 'ghosty.nsi'
        if ($LASTEXITCODE -ne 0) {
            throw "makensis.exe exited with code $LASTEXITCODE."
        }
    }
    finally {
        Pop-Location
    }

    $installer = Join-Path $stageInstallerDir 'GhostyCodeSetup.exe'
    if (-not (Test-Path -LiteralPath $installer -PathType Leaf)) {
        throw 'GhostyCodeSetup.exe was not produced.'
    }

    $canaries = [System.Collections.Generic.List[string]]::new()
    $index = 1
    while ((($canaries -join ';').Length) -lt 1800) {
        $canaries.Add(('C:\CWCanary\DevelopmentTool{0:D4}\bin' -f $index))
        $index++
    }
    $seedPath = $canaries -join ';'
    $environmentKey.SetValue(
        'Path',
        $seedPath,
        [Microsoft.Win32.RegistryValueKind]::ExpandString
    )

    $installArguments = [System.Collections.Generic.List[string]]::new()
    $installArguments.Add('/S')
    $installArguments.Add("/D=$installDir")
    $installProcess = Start-Process `
        -FilePath $installer `
        -ArgumentList @($installArguments) `
        -Wait `
        -PassThru
    if ($installProcess.ExitCode -ne 0) {
        throw "GhostyCodeSetup.exe exited with code $($installProcess.ExitCode)."
    }

    $expectedBin = Join-Path $installDir 'bin'
    $afterInstall = Get-RawUserPath -EnvironmentKey $environmentKey
    if (([string] $afterInstall) -cne "$seedPath;$expectedBin") {
        throw "The installer did not preserve the seeded long PATH exactly. Before=$($seedPath.Length), after=$(([string] $afterInstall).Length)."
    }
    if ($environmentKey.GetValueKind('Path') -ne [Microsoft.Win32.RegistryValueKind]::ExpandString) {
        throw 'The installer changed the user PATH registry value kind.'
    }

    $launcher = Join-Path $expectedBin 'ghosty.bat'
    if (-not (Test-Path -LiteralPath $launcher -PathType Leaf)) {
        throw 'The installer did not install ghosty.bat.'
    }
    $launcherText = [System.IO.File]::ReadAllText($launcher)
    if ($launcherText -notmatch 'where wt') {
        throw 'Installed ghosty.bat does not prefer Windows Terminal.'
    }
    if ($launcherText -notmatch 'ghosty\.exe') {
        throw 'Installed ghosty.bat does not launch ghosty.exe.'
    }
    if (-not (Test-Path -LiteralPath $shortcut -PathType Leaf)) {
        throw 'The installer did not create the Start Menu shortcut.'
    }
    $shell = New-Object -ComObject WScript.Shell
    try {
        $lnk = $shell.CreateShortcut($shortcut)
        $expectedTarget = [System.IO.Path]::GetFullPath($launcher)
        $actualTarget = [System.IO.Path]::GetFullPath($lnk.TargetPath)
        if ($actualTarget -cne $expectedTarget) {
            throw "Start Menu shortcut target was '$($lnk.TargetPath)', expected '$launcher'."
        }
    }
    finally {
        [void] [System.Runtime.InteropServices.Marshal]::ReleaseComObject($shell)
    }

    $uninstaller = Join-Path $installDir 'Uninstall.exe'
    if (-not (Test-Path -LiteralPath $uninstaller -PathType Leaf)) {
        throw 'Uninstall.exe was not produced.'
    }
    $uninstallProcess = Start-Process `
        -FilePath $uninstaller `
        -ArgumentList '/S' `
        -Wait `
        -PassThru
    if ($uninstallProcess.ExitCode -ne 0) {
        throw "Uninstall.exe exited with code $($uninstallProcess.ExitCode)."
    }

    $afterUninstall = Get-RawUserPath -EnvironmentKey $environmentKey
    if (([string] $afterUninstall) -cne $seedPath) {
        throw 'Install followed by uninstall did not restore the seeded long PATH exactly.'
    }
    if (Test-Path -LiteralPath $launcher -PathType Leaf) {
        throw 'Uninstall did not remove ghosty.bat.'
    }
    if (Test-Path -LiteralPath $shortcut -PathType Leaf) {
        throw 'Uninstall did not remove the Start Menu shortcut.'
    }
    if (Test-Path -LiteralPath $shortcutDir) {
        throw 'Uninstall did not remove the Start Menu folder.'
    }

    Write-Host "Full NSIS installer PATH regression passed with $($seedPath.Length) characters."
}
finally {
    $uninstaller = Join-Path $installDir 'Uninstall.exe'
    if (Test-Path -LiteralPath $uninstaller -PathType Leaf) {
        try {
            [void] (Start-Process -FilePath $uninstaller -ArgumentList '/S' -Wait -PassThru)
        }
        catch {
            Write-Warning "Best-effort test uninstallation failed: $_"
        }
    }

    if ($originalPathExists) {
        $environmentKey.SetValue('Path', $originalPath, $originalPathKind)
    }
    else {
        $environmentKey.DeleteValue('Path', $false)
    }
    $environmentKey.Close()

    if (Test-Path -LiteralPath $shortcut -PathType Leaf) {
        Remove-Item -LiteralPath $shortcut -Force -ErrorAction SilentlyContinue
    }
    if (Test-Path -LiteralPath $shortcutDir) {
        Remove-Item -LiteralPath $shortcutDir -Recurse -Force -ErrorAction SilentlyContinue
    }

    if (Test-Path -LiteralPath $testRoot) {
        Remove-Item -LiteralPath $testRoot -Recurse -Force -ErrorAction SilentlyContinue
    }
}
