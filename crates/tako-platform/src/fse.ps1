param([ValidateSet('status','enable','disable')][string]$Action, [string]$Layout, [string]$Executable)
$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = [Text.UTF8Encoding]::new($false)
$gaming = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\GamingConfiguration'
$dev = 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\AppModelUnlock'
$backup = Join-Path $Layout 'previous.clixml'
$name = 'Tako.GamingHome'
function Read-PackageCertificate($path) {
    [Security.Cryptography.X509Certificates.X509Certificate2]::new($path)
}
function Set-MachineState($value, [string]$ImportCertificate = '', [string[]]$RemoveThumbprints = @()) {
    # Uma elevação reúne confiança e Dev Mode; HKCU e AppX ficam no usuário original.
    foreach ($thumbprint in $RemoveThumbprints) {
        if ($thumbprint -notmatch '^[A-Fa-f0-9]{40}$') { throw 'Invalid saved certificate thumbprint.' }
    }
    $operation = if ($null -eq $value) {
        "if ((Get-ItemProperty -LiteralPath '$dev' -ErrorAction SilentlyContinue).PSObject.Properties.Name -contains 'AllowDevelopmentWithoutDevLicense') { Remove-ItemProperty -LiteralPath '$dev' -Name AllowDevelopmentWithoutDevLicense }"
    } else {
        "if (-not (Test-Path -LiteralPath '$dev')) { New-Item -Path '$dev' | Out-Null }; New-ItemProperty -LiteralPath '$dev' -Name AllowDevelopmentWithoutDevLicense -PropertyType DWord -Value $([int]$value) -Force | Out-Null"
    }
    if ($ImportCertificate) {
        $expected = (Read-PackageCertificate $ImportCertificate).Thumbprint
        if ($expected -notmatch '^[A-Fa-f0-9]{40}$') { throw 'Invalid package certificate thumbprint.' }
        $escaped = $ImportCertificate.Replace("'", "''")
        $operation += "; if (([Security.Cryptography.X509Certificates.X509Certificate2]::new('$escaped')).Thumbprint -ne '$expected') { throw 'Certificate changed during elevation' }; Import-Certificate -FilePath '$escaped' -CertStoreLocation Cert:\LocalMachine\TrustedPeople | Out-Null"
    }
    foreach ($thumbprint in $RemoveThumbprints) { $operation += "; if (Test-Path 'Cert:\LocalMachine\TrustedPeople\$thumbprint') { Remove-Item 'Cert:\LocalMachine\TrustedPeople\$thumbprint' }" }
    $code = "try { `$ErrorActionPreference='Stop'; $operation; exit 0 } catch { exit 1 }"
    $encoded = [Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes($code))
    $p = Start-Process -FilePath "$PSHOME\powershell.exe" -Verb RunAs -ArgumentList @('-NoProfile','-NonInteractive','-EncodedCommand',$encoded) -Wait -PassThru
    if ($p.ExitCode -ne 0) { throw 'Developer Mode/certificate setup failed.' }
}
function Save-Previous($state) {
    $state | Export-Clixml -LiteralPath "$backup.tmp"
    Move-Item -LiteralPath "$backup.tmp" -Destination $backup -Force
}
function Owned-Certificates($state) {
    if ($state.CertificateThumbprints) { $state.CertificateThumbprints }
    elseif ($state.ImportedCertificate) { $state.Thumbprint }
}
function Restore-Previous($state) {
    $package = Get-AppxPackage -Name $name
    $currentHome = (Get-ItemProperty -LiteralPath $gaming -ErrorAction SilentlyContinue).GamingHomeApp
    # Preserve a later user choice, including removing GamingHomeApp.
    $assignedHome = if ($state.AssignedHome) { $state.AssignedHome } elseif ($package) { "$($package.PackageFamilyName)!App" } else { $null }
    if ($assignedHome -and $currentHome -eq $assignedHome) {
        if ($null -eq $state.Home) {
            Remove-ItemProperty -LiteralPath $gaming -Name GamingHomeApp
        } else {
            if (-not (Test-Path -LiteralPath $gaming)) { New-Item -Path $gaming | Out-Null }
            New-ItemProperty -LiteralPath $gaming -Name GamingHomeApp -Value $state.Home -PropertyType $state.HomeKind -Force | Out-Null
        }
    }
    Get-AppxPackage -Name $name | Remove-AppxPackage
    $current = (Get-ItemProperty -LiteralPath $dev -ErrorAction SilentlyContinue).AllowDevelopmentWithoutDevLicense
    $remove = @(Owned-Certificates $state | Where-Object { Test-Path -LiteralPath "Cert:\LocalMachine\TrustedPeople\$_" })
    if ($current -ne $state.Dev -or $remove.Count -gt 0) {
        Set-MachineState $state.Dev -RemoveThumbprints $remove
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
    $certificatePath = Join-Path $Layout 'Tako-GamingHome.cer'
    $certificate = Read-PackageCertificate $certificatePath
    if ($certificate.Subject -ne 'CN=Tako Dev') { throw 'Unexpected FSE certificate publisher.' }
    $oldDev = (Get-ItemProperty -LiteralPath $dev -ErrorAction SilentlyContinue).AllowDevelopmentWithoutDevLicense
    $import = -not (Test-Path -LiteralPath "Cert:\LocalMachine\TrustedPeople\$($certificate.Thumbprint)")
    $retired = @()
    if (Test-Path -LiteralPath $backup) {
        $state = Import-Clixml -LiteralPath $backup
        if (-not $package -or $gamingHome -ne "$($package.PackageFamilyName)!App") {
            throw 'Previous setup requires recovery: turn this option off first.'
        }
        if ($state.Thumbprint -eq $certificate.Thumbprint -and $state.Executable -eq $Executable) { exit 0 }
        $retired = @(Owned-Certificates $state | Where-Object { $_ -ne $certificate.Thumbprint })
    } else {
        if ($package) { throw 'An existing Tako gaming identity has no backup; refusing to overwrite it.' }
        $kind = if ($null -ne $gamingHome) { $gamingKey.GetValueKind('GamingHomeApp').ToString() } else { 'String' }
        $state = [pscustomobject]@{ Home = $gamingHome; HomeKind = $kind; Dev = $oldDev }
    }
    $owned = @($retired)
    if ($import -or (@(Owned-Certificates $state) -contains $certificate.Thumbprint)) { $owned += $certificate.Thumbprint }
    # Salvar ambos os certificados antes de remover o pacote: falhas continuam recuperáveis.
    $state | Add-Member -NotePropertyMembers @{ Thumbprint = $certificate.Thumbprint; CertificateThumbprints = $owned; Executable = $Executable; AssignedHome = $(if ($package) { "$($package.PackageFamilyName)!App" } else { $null }) } -Force
    Save-Previous $state
    try {
        if ($package) { $package | Remove-AppxPackage }
        if ($oldDev -ne 1 -or $import -or $retired.Count -gt 0) { Set-MachineState 1 -ImportCertificate $(if ($import) { $certificatePath } else { '' }) -RemoveThumbprints $retired }
        $msix = Join-Path $Layout 'Tako-GamingHome.msix'
        Write-Host "FSE pre-install: exe=$(Test-Path -LiteralPath $Executable); DevMode=$((Get-ItemProperty -LiteralPath $dev).AllowDevelopmentWithoutDevLicense); AllowAllTrustedApps=$((Get-ItemProperty -LiteralPath $dev).AllowAllTrustedApps); TrustedPeople=$(Test-Path -LiteralPath "Cert:\LocalMachine\TrustedPeople\$($certificate.Thumbprint)"); Root=$(Test-Path -LiteralPath "Cert:\LocalMachine\Root\$($certificate.Thumbprint)")"
        Add-AppxPackage -Path $msix -ExternalLocation (Split-Path -Parent $Executable) -ForceUpdateFromAnyVersion
        $package = Get-AppxPackage -Name $name
        if (-not $package) { throw 'Package registration did not produce an identity.' }
        $state.AssignedHome = "$($package.PackageFamilyName)!App"
        Save-Previous $state
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
