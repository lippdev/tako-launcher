; Tako (Rust) — instalador por usuário (sem administrador). Gerado por build\rust\Package-Tako.ps1:
;   ISCC /DAppVersion=0.2.0 /DSourceDir=<pasta com Tako.exe> /DOutputDir=<dist> /DArch=x64 tako.iss
; Mesmo AppId das alphas C#: a instalação existente é atualizada no lugar e os dados em %LOCALAPPDATA%\Tako ficam.

#ifndef AppVersion
  #define AppVersion "0.0.0"
#endif
#ifndef SourceDir
  #error SourceDir is required
#endif
#ifndef OutputDir
  #define OutputDir "..\..\dist"
#endif
#ifndef Arch
  #define Arch "x64"
#endif

#define AppName "Tako"
#define AppExe "Tako.exe"

[Setup]
; Keep this AppId forever: it is how updates find the existing install.
AppId={{6F0B8E2C-4D7A-4C1E-9B5F-7A3D2E1C9B84}
AppName={#AppName}
AppVersion={#AppVersion}
AppVerName={#AppName} {#AppVersion}
AppPublisher=lippdev
AppCopyright=Copyright (C) 2026 Filipe Moreira (lippdev) e colaboradores. AGPL-3.0-only.
AppPublisherURL=https://github.com/lippdev/tako-launcher
AppSupportURL=https://github.com/lippdev/tako-launcher/issues
AppUpdatesURL=https://github.com/lippdev/tako-launcher/releases
DefaultDirName={localappdata}\Programs\{#AppName}
DefaultGroupName={#AppName}
DisableProgramGroupPage=yes
DisableDirPage=auto
PrivilegesRequired=lowest
#if Arch == "arm64"
ArchitecturesAllowed=arm64
ArchitecturesInstallIn64BitMode=arm64
#else
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
#endif
OutputDir={#OutputDir}
OutputBaseFilename=Tako-Setup-{#Arch}
SetupIconFile=..\..\packaging\tako.ico
UninstallDisplayIcon={app}\{#AppExe}
UninstallDisplayName={#AppName}
Compression=lzma2/ultra64
SolidCompression=yes
WizardStyle=modern
ShowLanguageDialog=yes
LanguageDetectionMethod=uilanguage
UsePreviousLanguage=yes
; The windowless supervisor (ADR-0026) cannot be closed gracefully.
CloseApplications=force
RestartApplications=no

[Languages]
Name: "ptbr"; MessagesFile: "compiler:Languages\BrazilianPortuguese.isl"
Name: "en"; MessagesFile: "compiler:Default.isl"
Name: "es"; MessagesFile: "compiler:Languages\Spanish.isl"

[CustomMessages]
ptbr.DesktopShortcut=Atalho do Tako na Área de Trabalho
en.DesktopShortcut=Tako desktop shortcut
es.DesktopShortcut=Acceso directo de Tako en el escritorio
ptbr.LaunchApp=Abrir o Tako
en.LaunchApp=Open Tako
es.LaunchApp=Abrir Tako

[Tasks]
Name: "desktopicon"; Description: "{cm:DesktopShortcut}"; Flags: unchecked

[InstallDelete]
; Arquivos da versão C#/WPF (vários .dll, temas do Playnite e o PresentMon) que o Tako em Rust não usa mais.
Type: files; Name: "{app}\*.dll"
Type: files; Name: "{app}\*.pdb"
Type: files; Name: "{app}\*.json"
Type: files; Name: "{app}\PresentMon*.exe"
Type: filesandordirs; Name: "{app}\Themes"
Type: filesandordirs; Name: "{app}\runtimes"

[Files]
Source: "{#SourceDir}\{#AppExe}"; DestDir: "{app}"; Flags: ignoreversion
; Tells the app it is installed: data goes to %LOCALAPPDATA%\Tako instead of next to the exe.
Source: "..\Tako.installed"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\{#AppName}"; Filename: "{app}\{#AppExe}"
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\{#AppExe}"; Tasks: desktopicon

[Registry]
Root: HKCU; Subkey: "Software\Classes\tako"; ValueType: string; ValueData: "URL:Tako"; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Classes\tako"; ValueType: string; ValueName: "URL Protocol"; ValueData: ""
Root: HKCU; Subkey: "Software\Classes\tako\shell\open\command"; ValueType: string; ValueData: """{app}\{#AppExe}"" --open-url ""%1"""

[Run]
Filename: "{app}\{#AppExe}"; Description: "{cm:LaunchApp}"; Flags: nowait postinstall skipifsilent
; Silent installs (in-app updates): bring the app back afterwards, unless the update runs as the app exits (/NOLAUNCH=1).
Filename: "{app}\{#AppExe}"; Flags: nowait; Check: RelaunchAfterSilentUpdate

[Code]
function RelaunchAfterSilentUpdate: Boolean;
begin
  Result := WizardSilent and (ExpandConstant('{param:NOLAUNCH|0}') = '0');
end;

[UninstallRun]
; O disable compartilhado restaura HKCU/Dev Mode e remove apenas o certificado salvo.
Filename: "{app}\{#AppExe}"; Parameters: "--fse-setup disable"; Flags: runhidden waituntilterminated skipifdoesntexist; RunOnceId: "RestoreFse"

[UninstallDelete]
Type: files; Name: "{app}\Tako.installed"
; User data in %LOCALAPPDATA%\Tako (library, caches) is kept on purpose.
