; Tako — instalador por usuário (sem administrador). Gerado por build\Publish-Tako.ps1:
;   ISCC /DAppVersion=0.1.0 /DSourceDir=<pasta do publish> /DOutputDir=<dist> /DArch=x64 Tako.iss

#ifndef AppVersion
  #define AppVersion "0.0.0"
#endif
#ifndef SourceDir
  #error SourceDir (the dotnet publish folder) is required
#endif
#ifndef OutputDir
  #define OutputDir "..\dist"
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
AppPublisherURL=https://github.com/lippdev/ultralauncher
AppSupportURL=https://github.com/lippdev/ultralauncher/issues
AppUpdatesURL=https://github.com/lippdev/ultralauncher/releases
DefaultDirName={localappdata}\Programs\{#AppName}
DefaultGroupName={#AppName}
DisableProgramGroupPage=yes
DisableDirPage=auto
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir={#OutputDir}
OutputBaseFilename=Tako-Setup-{#Arch}
SetupIconFile=..\src\Launcher.UI.Wpf\Assets\Brand\tako.ico
UninstallDisplayIcon={app}\{#AppExe}
UninstallDisplayName={#AppName}
Compression=lzma2/ultra64
SolidCompression=yes
WizardStyle=modern
; Always ask for the installer language, preselecting the Windows display language.
ShowLanguageDialog=yes
LanguageDetectionMethod=uilanguage
UsePreviousLanguage=yes
; Close the app through Restart Manager if it is still up. "force": the windowless supervisor (ADR-0012) cannot be closed gracefully.
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

[Files]
Source: "{#SourceDir}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs
; Tells the app it is installed: data goes to %LOCALAPPDATA%\Tako instead of next to the exe.
Source: "Tako.installed"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\{#AppName}"; Filename: "{app}\{#AppExe}"
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\{#AppExe}"; Tasks: desktopicon

[Run]
Filename: "{app}\{#AppExe}"; Description: "{cm:LaunchApp}"; Flags: nowait postinstall skipifsilent
; Silent installs (in-app updates): bring the app back afterwards, unless the update runs as the app exits (/NOLAUNCH=1).
Filename: "{app}\{#AppExe}"; Flags: nowait; Check: RelaunchAfterSilentUpdate

[Code]
function RelaunchAfterSilentUpdate: Boolean;
begin
  Result := WizardSilent and (ExpandConstant('{param:NOLAUNCH|0}') = '0');
end;

[UninstallDelete]
Type: files; Name: "{app}\Tako.installed"
; User data in %LOCALAPPDATA%\Tako (library, caches) is kept on purpose.
