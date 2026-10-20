param([ValidateSet('status','enable','disable')][string]$Action, [string]$Layout, [string]$Executable)
$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = [Text.UTF8Encoding]::new($false)
$gaming = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\GamingConfiguration'
$dev = 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\AppModelUnlock'
$backup = Join-Path $Layout 'previous.clixml'
$name = 'Tako.GamingHome'
function Set-DeveloperMode($value) {
    # Only this fixed HKLM operation runs elevated; never the app or user package registration.
    $operation = if ($null -eq $value) {
        "if ((Get-ItemProperty -LiteralPath '$dev').PSObject.Properties.Name -contains 'AllowDevelopmentWithoutDevLicense') { Remove-ItemProperty -LiteralPath '$dev' -Name AllowDevelopmentWithoutDevLicense }"
    } else {
        "if (-not (Test-Path -LiteralPath '$dev')) { New-Item -Path '$dev' | Out-Null }; New-ItemProperty -LiteralPath '$dev' -Name AllowDevelopmentWithoutDevLicense -PropertyType DWord -Value $([int]$value) -Force | Out-Null"
    }
    $code = "try { `$ErrorActionPreference='Stop'; $operation; exit 0 } catch { exit 1 }"
    $encoded = [Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes($code))
    $p = Start-Process -FilePath "$PSHOME\powershell.exe" -Verb RunAs -ArgumentList @('-NoProfile','-NonInteractive','-EncodedCommand',$encoded) -Wait -PassThru
    if ($p.ExitCode -ne 0) { throw 'Developer Mode setup failed.' }
}
function Restore-Previous($state) {
    $package = Get-AppxPackage -Name $name
    $currentHome = (Get-ItemProperty -LiteralPath $gaming -ErrorAction SilentlyContinue).GamingHomeApp
    # Preserve a later user choice, including removing GamingHomeApp.
    if ($package -and $currentHome -eq "$($package.PackageFamilyName)!App") {
        if ($null -eq $state.Home) {
            Remove-ItemProperty -LiteralPath $gaming -Name GamingHomeApp
        } else {
            if (-not (Test-Path -LiteralPath $gaming)) { New-Item -Path $gaming | Out-Null }
            New-ItemProperty -LiteralPath $gaming -Name GamingHomeApp -Value $state.Home -PropertyType $state.HomeKind -Force | Out-Null
        }
    }
    Get-AppxPackage -Name $name | Remove-AppxPackage
    if ($state.Dev -ne 1) {
        $current = (Get-ItemProperty -LiteralPath $dev -ErrorAction SilentlyContinue).AllowDevelopmentWithoutDevLicense
        if ($current -ne $state.Dev) { Set-DeveloperMode $state.Dev }
    }
    Remove-Item -LiteralPath $backup
}
$lock = $null
try {
    $lock = [IO.File]::Open((Join-Path $Layout 'setup.lock'), [IO.FileMode]::OpenOrCreate, [IO.FileAccess]::ReadWrite, [IO.FileShare]::None)
    $package = Get-AppxPackage -Name $name
    $gamingKey = Get-Item -LiteralPath $gaming -ErrorAction SilentlyContinue
    $gamingHome = if ($null -ne $gamingKey) { $gamingKey.GetValue('GamingHomeApp', $null, [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames) } else { $null }
    # Presence of GamingConfiguration is conservative: build alone cannot establish rollout availability.
    $supported = [Environment]::OSVersion.Version.Build -ge 26100 -and (Test-Path -LiteralPath $gaming)
    if ($Action -eq 'status') {
        if ($package -and $gamingHome -eq "$($package.PackageFamilyName)!App") { 'enabled' }
        elseif (Test-Path -LiteralPath $backup) { 'recovery' }
        elseif ($supported) { 'disabled' }
        else { 'unavailable' }
        exit 0
    }
    if ($Action -eq 'disable') {
        if (Test-Path -LiteralPath $backup) { Restore-Previous (Import-Clixml -LiteralPath $backup) }
        elseif ($package) { throw 'Previous configuration backup is missing; automatic restoration is unavailable.' }
        exit 0
    }
    if (-not $supported) { throw 'Full screen experience is unavailable on this Windows.' }
    if (Test-Path -LiteralPath $backup) {
        if ($package -and $gamingHome -eq "$($package.PackageFamilyName)!App") { exit 0 }
        throw 'Previous setup requires recovery: turn this option off first.'
    }
    if ($package) { throw 'An existing Tako gaming identity has no backup; refusing to overwrite it.' }
    $oldDev = (Get-ItemProperty -LiteralPath $dev -ErrorAction SilentlyContinue).AllowDevelopmentWithoutDevLicense
    $kind = if ($null -ne $gamingHome) { $gamingKey.GetValueKind('GamingHomeApp').ToString() } else { 'String' }
    $state = [pscustomobject]@{ Home = $gamingHome; HomeKind = $kind; Dev = $oldDev }
    $state | Export-Clixml -LiteralPath "$backup.tmp"
    Move-Item -LiteralPath "$backup.tmp" -Destination $backup
    try {
        if ($oldDev -ne 1) { Set-DeveloperMode 1 }
        $manifest = Join-Path $Layout 'AppxManifest.xml'
        Add-AppxPackage -Register $manifest -ExternalLocation (Split-Path -Parent $Executable) -ForceUpdateFromAnyVersion
        $package = Get-AppxPackage -Name $name
        if (-not $package) { throw 'Package registration did not produce an identity.' }
        New-ItemProperty -LiteralPath $gaming -Name GamingHomeApp -Value "$($package.PackageFamilyName)!App" -PropertyType String -Force | Out-Null
    } catch {
        $failure = $_
        try { Restore-Previous $state } catch { throw "$failure; recovery failed: $_. Turn this option off to retry recovery." }
        throw $failure
    }
} catch {
    [Console]::Error.WriteLine($_.ToString())
    exit 1
}
finally {
    if ($null -ne $lock) { $lock.Dispose() }
}
