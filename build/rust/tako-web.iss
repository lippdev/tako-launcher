; Tako Web (Rust + Chromium/CEF) — instalador de prévia por usuário (sem administrador).
; Gerado por build\rust\Package-Web.ps1:
;   ISCC /DAppVersion=0.2.0-preview /DSourceDir=<pasta de estágio> /DOutputDir=<dist> /DArch=x64 tako-web.iss
; Instalador independente da versão Slint existente: pasta própria e atalhos com `--web`.

#ifndef AppVersion
  #define AppVersion "0.0.0-preview"
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

#define AppName "Tako Web (prévia)"
#define AppExe "tako.exe"

[Setup]
; AppId próprio para a prévia da versão web (não sobrescreve o Tako Slint nem alphas antigas)
AppId={{8B19D986-7786-4F13-A5DE-8B895C9A43E1}
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
OutputBaseFilename=Tako-Web-Preview-Setup-{#Arch}
SetupIconFile=..\..\packaging\tako.ico
UninstallDisplayIcon={app}\{#AppExe}
UninstallDisplayName={#AppName}
Compression=lzma2/ultra64
SolidCompression=yes
WizardStyle=modern
ShowLanguageDialog=yes
LanguageDetectionMethod=uilanguage
UsePreviousLanguage=yes
CloseApplications=force
RestartApplications=no

[Languages]
Name: "ptbr"; MessagesFile: "compiler:Languages\BrazilianPortuguese.isl"
Name: "en"; MessagesFile: "compiler:Default.isl"
Name: "es"; MessagesFile: "compiler:Languages\Spanish.isl"

[CustomMessages]
ptbr.DesktopShortcut=Atalho do Tako Web (prévia) na Área de Trabalho
en.DesktopShortcut=Tako Web (preview) desktop shortcut
es.DesktopShortcut=Acceso directo de Tako Web (previa) en el escritorio
ptbr.LaunchApp=Abrir o Tako Web (prévia)
en.LaunchApp=Open Tako Web (preview)
es.LaunchApp=Abrir Tako Web (previa)

[Tasks]
Name: "desktopicon"; Description: "{cm:DesktopShortcut}"; Flags: unchecked

[Files]
Source: "{#SourceDir}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "..\Tako.installed"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\{#AppName}"; Filename: "{app}\{#AppExe}"; Parameters: "--web"; IconFilename: "{app}\{#AppExe}"
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\{#AppExe}"; Parameters: "--web"; Tasks: desktopicon; IconFilename: "{app}\{#AppExe}"

[Run]
Filename: "{app}\{#AppExe}"; Parameters: "--web"; Description: "{cm:LaunchApp}"; Flags: nowait postinstall skipifsilent

[UninstallDelete]
Type: files; Name: "{app}\Tako.installed"
