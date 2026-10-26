# Dot-source este arquivo para preparar o cache compartilhado e configurar a sessão atual.
# Uso: . .\scripts\cef-setup.ps1
[CmdletBinding()]
param()

& {
    $ErrorActionPreference = 'Stop'
    $cefLockfile = Get-Content -LiteralPath (Join-Path $PSScriptRoot '../Cargo.lock') -Raw
    $cefVersionMatch = [regex]::Match($cefLockfile, '(?m)^name = "cef"\r?\nversion = "([^"+]+)\+([^"+]+)"')
    if (-not $cefVersionMatch.Success) { throw 'Versão da crate cef ausente ou inválida no Cargo.lock.' }
    $cefBindingVersion = $cefVersionMatch.Groups[1].Value + '+' + $cefVersionMatch.Groups[2].Value
    $cefRuntimeVersion = $cefVersionMatch.Groups[2].Value
    $cefCache = Join-Path $env:LOCALAPPDATA "tako-dev\cef\$cefRuntimeVersion"
    $cefTools = Join-Path $env:LOCALAPPDATA "tako-dev\cef-tools\$cefBindingVersion"
    $cefExporter = Join-Path $cefTools 'bin\export-cef-dir.exe'
    $cefRuntime = Join-Path $cefCache 'cef_windows_x86_64'

    New-Item -ItemType Directory -Force -Path $cefCache | Out-Null
    $cefSetupLock = $null
    $cefLockDeadline = [DateTime]::UtcNow.AddSeconds(60)
    while ($null -eq $cefSetupLock) {
        try {
            $cefSetupLock = [IO.File]::Open((Join-Path $cefCache '.setup.lock'), [IO.FileMode]::OpenOrCreate, [IO.FileAccess]::ReadWrite, [IO.FileShare]::None)
        } catch [IO.IOException] {
            if ([DateTime]::UtcNow -ge $cefLockDeadline) { throw 'Outro setup CEF está em andamento; tente novamente depois dele.' }
            Start-Sleep -Milliseconds 250
        }
    }
    try {
        if (-not (Test-Path -LiteralPath (Join-Path $cefRuntime 'archive.json'))) {
            if (-not (Test-Path -LiteralPath $cefExporter)) {
                & cargo install export-cef-dir --version "=$cefBindingVersion" --locked --root $cefTools
                if ($LASTEXITCODE -ne 0) { throw 'Falha ao instalar export-cef-dir.' }
            }
            & $cefExporter --target x86_64-pc-windows-msvc $cefRuntime
            if ($LASTEXITCODE -ne 0) { throw 'Falha ao baixar ou conferir o CEF.' }
        }
    } finally { $cefSetupLock.Dispose() }

    $env:CEF_PATH = $cefRuntime
    $cmakeBin = 'C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\Common7\IDE\CommonExtensions\Microsoft\CMake\CMake\bin'
    $ninjaBin = 'C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\Common7\IDE\CommonExtensions\Microsoft\CMake\Ninja'
    $env:PATH = "$cmakeBin;$ninjaBin;$cefRuntime;$env:PATH"
    Write-Host "CEF_PATH=$env:CEF_PATH"
}
