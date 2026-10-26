#Requires -Version 5.1
param([Parameter(Mandatory=$true)][string]$OutputDirectory)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
New-Item -ItemType Directory -Path $OutputDirectory -Force | Out-Null
$OutputDirectory = (Resolve-Path $OutputDirectory).Path
$sdk = Get-ChildItem "${env:ProgramFiles(x86)}\Windows Kits\10\bin" -Directory |
    Sort-Object Name -Descending | Where-Object { Test-Path (Join-Path $_.FullName 'x64\MakeAppx.exe') } | Select-Object -First 1
if (-not $sdk) { throw 'Windows SDK com MakeAppx não encontrado.' }
$makeappx = Join-Path $sdk.FullName 'x64\MakeAppx.exe'
$signtool = Join-Path $sdk.FullName 'x64\signtool.exe'
$layout = Join-Path $OutputDirectory 'layout'
$certificate = $null
try {
    New-Item -ItemType Directory -Force -Path "$layout\Assets", "$layout\Public" | Out-Null
    # Pasta pública precisa existir também depois de empacotar (ZIP não guarda diretórios vazios).
    Set-Content "$layout\Public\identity.txt" 'Tako Gaming Home' -Encoding UTF8
    Copy-Item "$root\packaging\fse\CustomCapability.SCCD" $layout
    Add-Type -AssemblyName System.Drawing
    $logo = [Drawing.Image]::FromFile("$root\crates\tako-ui\theme\tako-logo.png")
    try {
        foreach ($asset in @(@('StoreLogo',50), @('Square150x150Logo',150), @('Square44x44Logo',44))) {
            $bitmap = [Drawing.Bitmap]::new($logo, [int]$asset[1], [int]$asset[1])
            try { $bitmap.Save("$layout\Assets\$($asset[0]).png", [Drawing.Imaging.ImageFormat]::Png) } finally { $bitmap.Dispose() }
        }
    } finally { $logo.Dispose() }
    $template = Get-Content "$root\packaging\fse\AppxManifest.xml" -Raw
    [xml]$manifest = $template
    $publisher = $manifest.Package.Identity.Publisher
    $baseVersion = if ($env:TAKO_VERSION -match '^(\d+\.\d+\.\d+)') { $Matches[1] } else { '0.0.0' }
    $revision = if ($env:GITHUB_RUN_NUMBER) { [int]$env:GITHUB_RUN_NUMBER } else { 0 }
    if ($revision -gt 32767) { throw 'Revisão MSIX excedeu o limite; atualize a estratégia de versão.' }
    $revision *= 2
    $manifest.Package.Identity.Version = "$baseVersion.$revision"
    $template = $manifest.OuterXml
    $certificate = New-SelfSignedCertificate -Type Custom -Subject $publisher -KeyUsage DigitalSignature -FriendlyName 'Tako FSE build' -CertStoreLocation Cert:\CurrentUser\My -KeyExportPolicy NonExportable -HashAlgorithm SHA256 -NotAfter (Get-Date).AddYears(5) -TextExtension @('2.5.29.37={text}1.3.6.1.5.5.7.3.3','2.5.29.19={text}')
    Export-Certificate -Cert $certificate -FilePath "$OutputDirectory\Tako-GamingHome.cer" | Out-Null
    $variantIndex = 0
    foreach ($variant in @(@('Tako.exe','Tako-GamingHome.msix'), @('Tako-Portable-x64.exe','Tako-GamingHome-Portable.msix'))) {
        [xml]$variantManifest = $template.Replace('__EXECUTABLE__', $variant[0])
        $variantManifest.Package.Identity.Version = "$baseVersion.$($revision + $variantIndex)"
        $variantIndex++
        $variantManifest.OuterXml | Set-Content "$layout\AppxManifest.xml" -Encoding UTF8
        $package = Join-Path $OutputDirectory $variant[1]
        # /nv é necessário para pacote esparso: o executável vive em ExternalLocation.
        & $makeappx pack /o /nv /d $layout /p $package
        if ($LASTEXITCODE -ne 0) { throw 'MakeAppx falhou.' }
        Add-Type -AssemblyName System.IO.Compression.FileSystem
        $zip = [IO.Compression.ZipFile]::OpenRead($package)
        try {
            foreach ($required in @('AppxManifest.xml','CustomCapability.SCCD','Public/identity.txt','Assets/StoreLogo.png','Assets/Square150x150Logo.png','Assets/Square44x44Logo.png')) {
                if (-not $zip.GetEntry($required)) { throw "Recurso ausente no MSIX: $required" }
            }
        } finally { $zip.Dispose() }
        & $signtool sign /sha1 $certificate.Thumbprint /fd SHA256 $package
        if ($LASTEXITCODE -ne 0) { throw 'SignTool falhou.' }
    }
} finally {
    if ($certificate) { Remove-Item "Cert:\CurrentUser\My\$($certificate.Thumbprint)" -DeleteKey }
    if (Test-Path $layout) { Remove-Item $layout -Recurse -Force }
}
