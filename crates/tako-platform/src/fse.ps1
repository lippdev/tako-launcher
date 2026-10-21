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
function Read-PackageDescriptor($path) {
    Add-Type -AssemblyName System.IO.Compression, System.IO.Compression.FileSystem
    $zip = [IO.Compression.ZipFile]::OpenRead($path)
    try {
        $entry = $zip.GetEntry('CustomCapability.SCCD')
        if (-not $entry) { throw 'CustomCapability.SCCD missing from signed package.' }
        $stream = $entry.Open()
        $buffer = [IO.MemoryStream]::new()
        try { $stream.CopyTo($buffer); return ,$buffer.ToArray() }
        finally { $stream.Dispose(); $buffer.Dispose() }
    } finally { $zip.Dispose() }
}
function Set-DescriptorFiles($Descriptors, [string]$Content, [switch]$Remove) {
    foreach ($descriptor in $Descriptors) {
        if ([IO.Path]::GetFileName($descriptor.Path) -ne 'CustomCapability.SCCD' -or $descriptor.Hash -notmatch '^[A-Fa-f0-9]{64}$') { throw 'Invalid saved SCCD metadata.' }
        if (Test-Path -LiteralPath $descriptor.Path) {
            # Sem Get-FileHash: o módulo dele some quando o PSModulePath vem do PowerShell 7.
            $sha = [Security.Cryptography.SHA256]::Create()
            try { $current = ([BitConverter]::ToString($sha.ComputeHash([IO.File]::ReadAllBytes($descriptor.Path)))).Replace('-', '') } finally { $sha.Dispose() }
            if ($current -ne $descriptor.Hash) {
                if ($Remove) { continue } # Preserve uma substituicao posterior feita por outro app/usuario.
                throw 'An unrelated CustomCapability.SCCD already exists beside the executable.'
            }
            if ($Remove) { Remove-Item -LiteralPath $descriptor.Path }
        } elseif (-not $Remove) {
            $bytes = [Convert]::FromBase64String($Content)
            $sha = [Security.Cryptography.SHA256]::Create()
            try { $hash = ([BitConverter]::ToString($sha.ComputeHash($bytes))).Replace('-', '') } finally { $sha.Dispose() }
            if ($hash -ne $descriptor.Hash) { throw 'SCCD content changed during elevation.' }
            $file = [IO.File]::Open($descriptor.Path, [IO.FileMode]::CreateNew, [IO.FileAccess]::Write, [IO.FileShare]::None)
            try { $file.Write($bytes, 0, $bytes.Length) } finally { $file.Dispose() }
        }
    }
}
function Set-MachineState($value, [string]$ImportCertificate = '', [string[]]$RemoveThumbprints = @(), [object[]]$Descriptors = @(), [string]$DescriptorContent = '', [switch]$RemoveDescriptors) {
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
    if ($Descriptors.Count -gt 0) {
        $json = [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes((ConvertTo-Json -InputObject @($Descriptors) -Compress)))
        if ($DescriptorContent -notmatch '^[A-Za-z0-9+/]*={0,2}$') { throw 'Invalid SCCD encoding.' }
        $operation += "; function Set-DescriptorFiles { ${function:Set-DescriptorFiles} }; Set-DescriptorFiles -Descriptors ([Text.Encoding]::UTF8.GetString([Convert]::FromBase64String('$json')) | ConvertFrom-Json) -Content '$DescriptorContent' -Remove:`$$([bool]$RemoveDescriptors)"
    }
    $code = "try { `$ErrorActionPreference='Stop'; $operation; exit 0 } catch { exit 1 }"
    $encoded = [Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes($code))
    $p = Start-Process -FilePath "$PSHOME\powershell.exe" -Verb RunAs -ArgumentList @('-NoProfile','-NonInteractive','-EncodedCommand',$encoded) -Wait -PassThru
    if ($p.ExitCode -ne 0) { throw 'Developer Mode/certificate/SCCD setup failed.' }
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
    $descriptors = @($state.Descriptors | Where-Object { $_ -and (Test-Path -LiteralPath $_.Path) })
    if ($current -ne $state.Dev -or $remove.Count -gt 0 -or $descriptors.Count -gt 0) {
        Set-MachineState $state.Dev -RemoveThumbprints $remove -Descriptors $descriptors -RemoveDescriptors
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
    $msix = Join-Path $Layout 'Tako-GamingHome.msix'
    $descriptorBytes = Read-PackageDescriptor $msix
    $descriptorContent = [Convert]::ToBase64String($descriptorBytes)
    $sha = [Security.Cryptography.SHA256]::Create()
    try { $descriptorHash = ([BitConverter]::ToString($sha.ComputeHash($descriptorBytes))).Replace('-', '') } finally { $sha.Dispose() }
    # AppX busca *.sccd em ExternalLocation mesmo quando o SCCD esta no MSIX.
    $descriptorPath = Join-Path (Split-Path -Parent $Executable) 'CustomCapability.SCCD'
    $newDescriptors = @()
    if (Test-Path -LiteralPath $descriptorPath) {
        $sha = [Security.Cryptography.SHA256]::Create()
        try { $current = ([BitConverter]::ToString($sha.ComputeHash([IO.File]::ReadAllBytes($descriptorPath)))).Replace('-', '') } finally { $sha.Dispose() }
        if ($current -ne $descriptorHash) { throw 'An unrelated CustomCapability.SCCD already exists beside the executable.' }
    } else { $newDescriptors = @([pscustomobject]@{ Path = $descriptorPath; Hash = $descriptorHash }) }
    $descriptors = @($state.Descriptors | Where-Object { $_ })
    if (-not @($descriptors | Where-Object { $_.Path -eq $descriptorPath }).Count) { $descriptors += $newDescriptors }
    $state | Add-Member -NotePropertyName Descriptors -NotePropertyValue $descriptors -Force
    $owned = @($retired)
    if ($import -or (@(Owned-Certificates $state) -contains $certificate.Thumbprint)) { $owned += $certificate.Thumbprint }
    # Salvar ambos os certificados antes de remover o pacote: falhas continuam recuperáveis.
    $state | Add-Member -NotePropertyMembers @{ Thumbprint = $certificate.Thumbprint; CertificateThumbprints = $owned; Executable = $Executable; AssignedHome = $(if ($package) { "$($package.PackageFamilyName)!App" } else { $null }) } -Force
    Save-Previous $state
    try {
        if ($package) { $package | Remove-AppxPackage }
        if ($oldDev -ne 1 -or $import -or $retired.Count -gt 0 -or $newDescriptors.Count -gt 0) { Set-MachineState 1 -ImportCertificate $(if ($import) { $certificatePath } else { '' }) -RemoveThumbprints $retired -Descriptors $newDescriptors -DescriptorContent $descriptorContent }
        Write-Host "FSE pre-install: exe=$(Test-Path -LiteralPath $Executable); DevMode=$((Get-ItemProperty -LiteralPath $dev).AllowDevelopmentWithoutDevLicense); AllowAllTrustedApps=$((Get-ItemProperty -LiteralPath $dev).AllowAllTrustedApps); TrustedPeople=$(Test-Path -LiteralPath "Cert:\LocalMachine\TrustedPeople\$($certificate.Thumbprint)"); Root=$(Test-Path -LiteralPath "Cert:\LocalMachine\Root\$($certificate.Thumbprint)")"
        # O cmdlet rejeita ambas as flags: ForceApplicationShutdown ja inclui o pacote e dependencias.
        Add-AppxPackage -Path $msix -ExternalLocation (Split-Path -Parent $Executable) -ForceUpdateFromAnyVersion -ForceApplicationShutdown
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
