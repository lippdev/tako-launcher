# Run with pwsh -NoProfile -File crates/tako-platform/tests/fse-state.ps1.
# All registry, AppX and elevation calls are fakes; CLIXML persistence uses a temporary directory.
$ErrorActionPreference = 'Stop'
$source = Get-Content (Join-Path $PSScriptRoot '../src/fse.ps1') -Raw
$source = $source.Replace('[Environment]::OSVersion.Version.Build', '$script:Build').Replace('exit 0', 'return').Replace('exit 1', 'throw ''setup failed''')
$script:AppRoot = Join-Path ([IO.Path]::GetTempPath()) 'Tako FSE app'
$script:Layout = Join-Path ([IO.Path]::GetTempPath()) ([guid]::NewGuid().ToString())
[IO.Directory]::CreateDirectory($script:Layout) | Out-Null
function Assert($condition, $message) { if (-not $condition) { throw $message } }
function Get-AppxPackage { param($Name) if ($script:Package) { [pscustomobject]@{PackageFamilyName='Tako.GamingHome_test'} } }
function Add-AppxPackage {
    param($Register,$ExternalLocation,[switch]$ForceUpdateFromAnyVersion)
    if ($script:FailRegister) { throw 'registration rejected' }
    Assert ($ExternalLocation -eq $script:AppRoot) 'external location'
    $script:Package = $true
}
function Remove-AppxPackage { param([Parameter(ValueFromPipeline)]$InputObject) process { if ($script:FailRemove) { throw 'remove failed' }; $script:Package = $false } }
function Test-Path { param($LiteralPath) if ($LiteralPath -like 'HKCU:*') { return $script:Supported }; Microsoft.PowerShell.Management\Test-Path -LiteralPath $LiteralPath }
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
    $script:Elevations++
    if ($code -match '-Value (\d+)') { $script:Dev = [int]$Matches[1] } else { $script:Dev = $null }
    [pscustomobject]@{ExitCode=0}
}
function Run($action) { & ([scriptblock]::Create($source)) -Action $action -Layout $script:Layout -Executable (Join-Path $script:AppRoot 'Tako.exe') }
function Reset($gamingHome,$dev) {
    Microsoft.PowerShell.Management\Remove-Item "$script:Layout/*" -Force -ErrorAction SilentlyContinue
    $script:OtherValue='preserved'; $script:GamingHome=$gamingHome; $script:Dev=$dev; $script:Package=$false; $script:Build=26100
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
            Assert (-not (Test-Path "$script:Layout/previous.clixml")) 'backup cleaned'
            if ($oldDev -eq 1) { Assert ($script:Elevations -eq 0) 'existing developer mode requires no UAC' }
        }
    }
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
    Microsoft.PowerShell.Management\Remove-Item -LiteralPath $script:Layout -Recurse -Force
}
