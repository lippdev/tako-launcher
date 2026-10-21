# Execute com pwsh -NoProfile -File crates/tako-platform/tests/fse-state.ps1.
# Registro, AppX e elevação são fakes; CLIXML persiste numa pasta temporária.
$ErrorActionPreference = 'Stop'
$source = Get-Content (Join-Path $PSScriptRoot '../src/fse.ps1') -Raw
$source = $source.Replace('[Security.Cryptography.X509Certificates.X509Certificate2]::new($path)', "[pscustomobject]@{Subject='CN=Tako Dev';Thumbprint=`$script:Thumbprint}")
$source = $source.Replace('[Environment]::OSVersion.Version.Build', '$script:Build').Replace('exit 0', 'return').Replace('exit 1', 'throw ''setup failed''')
$script:AppRoot = Join-Path ([IO.Path]::GetTempPath()) ('Tako FSE app ' + [guid]::NewGuid())
[IO.Directory]::CreateDirectory($script:AppRoot) | Out-Null
$script:Layout = Join-Path ([IO.Path]::GetTempPath()) ([guid]::NewGuid().ToString())
[IO.Directory]::CreateDirectory($script:Layout) | Out-Null
$fixture = Join-Path $script:AppRoot 'fixture.msix'
Add-Type -AssemblyName System.IO.Compression.FileSystem
$zip = [IO.Compression.ZipFile]::Open($fixture, [IO.Compression.ZipArchiveMode]::Create)
try { [IO.Compression.ZipFileExtensions]::CreateEntryFromFile($zip, (Join-Path $PSScriptRoot '../../../packaging/fse/CustomCapability.SCCD'), 'CustomCapability.SCCD') | Out-Null } finally { $zip.Dispose() }
$fixtureBytes = [IO.File]::ReadAllBytes($fixture)
function Assert($condition, $message) { if (-not $condition) { throw $message } }
function Get-AppxPackage { param($Name) if ($script:Package) { [pscustomobject]@{PackageFamilyName='Tako.GamingHome_test'} } }
function Add-AppxPackage {
    param($Path,$ExternalLocation,[switch]$ForceUpdateFromAnyVersion,[switch]$ForceApplicationShutdown,[switch]$ForceTargetApplicationShutdown)
    if ($script:FailRegister) { throw 'registration rejected' }
    Assert (Microsoft.PowerShell.Management\Test-Path -LiteralPath (Join-Path $script:AppRoot 'CustomCapability.SCCD')) 'SCCD must exist in ExternalLocation before registration'
    Assert ($Path -eq (Join-Path $script:Layout 'Tako-GamingHome.msix')) 'signed package path'
    Assert ($ExternalLocation -eq $script:AppRoot) 'external location'
    Assert ($ForceUpdateFromAnyVersion -and $ForceApplicationShutdown -and $ForceTargetApplicationShutdown) 'AnyFSE deployment options'
    $script:Package = $true
}
function Remove-AppxPackage { param([Parameter(ValueFromPipeline)]$InputObject) process { if ($script:FailRemove) { throw 'remove failed' }; $script:Package = $false } }
function Test-Path { param($LiteralPath) if ($LiteralPath -like 'Cert:*') { return $script:Trusted.Contains(($LiteralPath -split '\\')[-1]) }; if ($LiteralPath -like 'HKCU:*') { return $script:Supported }; Microsoft.PowerShell.Management\Test-Path -LiteralPath $LiteralPath }
function Get-ItemProperty {
    param($LiteralPath,$ErrorAction)
    if ($LiteralPath -like 'HKCU:*') { [pscustomobject]@{GamingHomeApp=$script:GamingHome} }
    else { [pscustomobject]@{AllowDevelopmentWithoutDevLicense=$script:Dev} }
}
function Get-Item {
    param($LiteralPath,$ErrorAction)
    $o = [pscustomobject]@{}
    $o | Add-Member ScriptMethod GetValueKind { param($name) 'String' }
    $o | Add-Member ScriptMethod GetValue { param($name,$fallback,$options) $script:GamingHome }
    $o
}
function New-Item { param($Path,[switch]$Force) if ($Force) { $script:OtherValue = $null } }
function New-ItemProperty {
    param($LiteralPath,$Name,$Value,$PropertyType,[switch]$Force)
    Assert ($LiteralPath -like 'HKCU:*') 'HKLM must only be elevated'
    $script:GamingHome = $Value
}
function Remove-ItemProperty { param($LiteralPath,$Name) $script:GamingHome = $null }
function Start-Process {
    param($FilePath,$Verb,$ArgumentList,[switch]$Wait,[switch]$PassThru)
    Assert ($Verb -eq 'RunAs') 'elevation verb'
    if ($script:Cancel) { throw 'UAC cancelled' }
    $code = [Text.Encoding]::Unicode.GetString([Convert]::FromBase64String($ArgumentList[-1]))
    Assert ($code -notmatch 'GamingHomeApp|Add-AppxPackage') 'user operation elevated'
    Assert ($code -notmatch 'New-Item -Path [^;]+ -Force') 'HKLM key must preserve other values'
    if ($code -match 'Import-Certificate') { $script:Trusted.Add($script:Thumbprint) | Out-Null }
    foreach ($match in [regex]::Matches($code, "Remove-Item 'Cert:.*?([A-F0-9]{40})'")) { $script:Trusted.Remove($match.Groups[1].Value) | Out-Null }
    $descriptorStart = $code.IndexOf('; function Set-DescriptorFiles')
    if ($descriptorStart -ge 0) {
        # Executar o trecho real de arquivos; somente registro/certificado/UAC sao fakes.
        & ([scriptblock]::Create($code.Substring($descriptorStart + 2, $code.LastIndexOf('; return') - $descriptorStart - 2)))
    }
    $script:Elevations++
    if ($code -match '-Value (\d+)') { $script:Dev = [int]$Matches[1] } else { $script:Dev = $null }
    [pscustomobject]@{ExitCode=0}
}
function Run($action) { & ([scriptblock]::Create($source)) -Action $action -Layout $script:Layout -Executable (Join-Path $script:AppRoot 'Tako.exe') }
function Reset($gamingHome,$dev) {
    Microsoft.PowerShell.Management\Remove-Item "$script:AppRoot/*" -Force -ErrorAction SilentlyContinue
    Microsoft.PowerShell.Management\Remove-Item "$script:Layout/*" -Force -ErrorAction SilentlyContinue
    [IO.File]::WriteAllBytes((Join-Path $script:Layout 'Tako-GamingHome.msix'), $fixtureBytes)
    $script:Thumbprint='A'*40; $script:Trusted=[Collections.Generic.HashSet[string]]::new(); $script:OtherValue='preserved'; $script:GamingHome=$gamingHome; $script:Dev=$dev; $script:Package=$false; $script:Build=26100
    $script:Supported=$true; $script:FailRegister=$false; $script:FailRemove=$false; $script:Cancel=$false; $script:Elevations=0
}
try {
    $appx = [xml](Get-Content (Join-Path $PSScriptRoot '../../../packaging/fse/AppxManifest.xml') -Raw)
    $win32 = [xml](Get-Content (Join-Path $PSScriptRoot '../../../packaging/fse/Tako.manifest') -Raw)
    Assert ($appx.Package.Identity.Name -eq $win32.assembly.msix.packageName) 'package identity mismatch'
    Assert ($appx.Package.Identity.Publisher -eq $win32.assembly.msix.publisher) 'publisher mismatch'
    Assert ($appx.Package.Applications.Application.Id -eq $win32.assembly.msix.applicationId) 'application mismatch'
    foreach ($oldDev in @($null,0,1)) {
        foreach ($oldHome in @($null,'Original!App')) {
            Reset $oldHome $oldDev
            Assert ((Run status) -eq 'disabled') 'initial status'
            Run enable
            Assert ((Run status) -eq 'enabled') 'enabled status'
            $before = Get-Content "$script:Layout/previous.clixml" -Raw
            Run enable
            Assert ((Get-Content "$script:Layout/previous.clixml" -Raw) -eq $before) 'backup overwritten'
            Assert ($script:Dev -eq 1) 'developer mode enabled'
            Run disable
            Assert ($script:GamingHome -eq $oldHome) 'home restored'
            Assert ($script:OtherValue -eq 'preserved') 'other registry values survive'
            Assert ($script:Dev -eq $oldDev) 'developer mode restored'
            Assert (-not $script:Package) 'package removed'
            Assert (-not (Test-Path -LiteralPath (Join-Path $script:AppRoot 'CustomCapability.SCCD'))) 'owned external SCCD removed'
            Assert (-not (Test-Path "$script:Layout/previous.clixml")) 'backup cleaned'
            Assert ($script:Elevations -eq 2) 'one elevation each for enable and disable'
            Assert ($script:Trusted.Count -eq 0) 'imported certificate removed'
        }
    }
    Reset 'Original!App' 1
    $descriptorPath = Join-Path $script:AppRoot 'CustomCapability.SCCD'
    $descriptorSource = Join-Path $PSScriptRoot '../../../packaging/fse/CustomCapability.SCCD'
    Copy-Item -LiteralPath $descriptorSource -Destination $descriptorPath
    $script:Trusted.Add($script:Thumbprint) | Out-Null
    Run enable
    Run disable
    Assert (Test-Path -LiteralPath $descriptorPath) 'preexisting external SCCD retained'
    Assert ($script:Elevations -eq 0) 'preexisting trust, descriptor and Dev Mode need no elevation'
    Reset 'Original!App' 0
    [IO.File]::WriteAllText($descriptorPath, 'another app')
    try { Run enable; throw 'expected failure' } catch { Assert ($_.ToString() -ne 'expected failure') 'conflicting SCCD must fail' }
    Assert ([IO.File]::ReadAllText($descriptorPath) -eq 'another app') 'conflicting SCCD preserved'
    Assert ($script:Elevations -eq 0 -and -not $script:Package) 'conflict detected before mutation'
    Reset 'Original!App' 0
    Run enable
    [IO.File]::WriteAllText($descriptorPath, 'later user replacement')
    Run disable
    Assert ([IO.File]::ReadAllText($descriptorPath) -eq 'later user replacement') 'later SCCD replacement preserved'
    Reset 'Original!App' 1
    $script:Trusted.Add($script:Thumbprint) | Out-Null
    Run enable
    Run disable
    Assert ($script:Trusted.Contains($script:Thumbprint)) 'preexisting certificate retained'
    Assert ($script:Elevations -eq 2) 'external SCCD setup and removal use elevation'
    Reset 'Original!App' 0
    Run enable
    $oldThumbprint = $script:Thumbprint
    $script:Thumbprint='B'*40
    Run enable
    Assert (-not $script:Trusted.Contains($oldThumbprint)) 'old build trust removed'
    Assert ($script:Trusted.Contains($script:Thumbprint)) 'new build trusted'
    Assert ($script:Elevations -eq 2) 'rotation uses one additional UAC prompt'
    Run disable
    Assert ($script:GamingHome -eq 'Original!App' -and $script:Dev -eq 0) 'rotation baseline restored'
    Assert ($script:Trusted.Count -eq 0) 'rotation trust cleaned'
    Reset 'Original!App' 0
    Run enable
    $script:Thumbprint='B'*40
    $script:FailRegister=$true
    try { Run enable; throw 'expected rotation failure' } catch { Assert ($_.ToString() -ne 'expected rotation failure') 'rotation should fail' }
    Assert ($script:GamingHome -eq 'Original!App' -and $script:Dev -eq 0) 'rotation failure restores baseline'
    Assert ($script:Trusted.Count -eq 0) 'rotation failure cleans both certificates'
    foreach ($laterHome in @('Later!App', $null)) {
        Reset 'Original!App' 0
        Run enable
        $script:GamingHome=$laterHome
        Run disable
        Assert ($script:GamingHome -eq $laterHome) 'later user choice preserved'
        Assert (-not $script:Package -and $script:Dev -eq 0) 'later choice cleanup'
        Assert (-not (Test-Path "$script:Layout/previous.clixml")) 'later choice backup cleaned'
    }
    Reset 'Original!App' 0
    $script:FailRegister=$true
    try { Run enable; throw 'expected failure' } catch { Assert ($_.ToString() -ne 'expected failure') 'registration should fail' }
    Assert ($script:GamingHome -eq 'Original!App' -and $script:Dev -eq 0) 'registration rollback'
    Reset 'Original!App' 0
    $script:Cancel=$true
    try { Run enable; throw 'expected failure' } catch { Assert ($_.ToString() -ne 'expected failure') 'cancel should fail' }
    Assert ($script:GamingHome -eq 'Original!App' -and $script:Dev -eq 0) 'cancel rollback'
    Reset 'Original!App' 0
    Run enable
    $script:FailRemove=$true
    try { Run disable; throw 'expected failure' } catch { Assert ($_.ToString() -ne 'expected failure') 'remove should fail' }
    Assert (Test-Path "$script:Layout/previous.clixml") 'failed recovery retains backup'
    Assert ((Run status) -eq 'recovery') 'failed recovery offers restoration'
    $script:FailRemove=$false
    $script:Supported=$false
    Run disable
    Assert ($script:GamingHome -eq 'Original!App' -and $script:Dev -eq 0) 'retry recovery'
    Assert ((Run status) -eq 'unavailable') 'unsupported status'
    'FSE: all fake state/restore/elevation tests passed.'
} finally {
    Microsoft.PowerShell.Management\Remove-Item -LiteralPath $script:Layout, $script:AppRoot -Recurse -Force
}
