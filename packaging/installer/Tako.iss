; Instalador do Tako (Inno Setup 6). Compilado pelo workflow de release:
;   iscc /DAppVersion=0.1.0-alpha.5 /DNumericVersion=0.1.0.0 /DSourceDir=<publish> /DOutputDir=<saída> Tako.iss
; Instala por usuário sem pedir administrador (ou para todos, se o usuário escolher). Os dados do usuário em
; %LOCALAPPDATA%\Tako (biblioteca, caches) são preservados na desinstalação.

#ifndef AppVersion
  #define AppVersion "0.0.0-dev"
#endif
#ifndef NumericVersion
  #define NumericVersion "0.0.0.0"
#endif
#ifndef SourceDir
  #define SourceDir "..\..\artifacts\Tako"
#endif
#ifndef OutputDir
  #define OutputDir "..\..\artifacts"
#endif

[Setup]
AppId={{6F0B8E2C-4D7A-4C1E-9B5F-7A3D2E1C9B84}
AppName=Tako
AppVersion={#AppVersion}
AppVerName=Tako {#AppVersion}
AppPublisher=Tako
AppPublisherURL=https://github.com/lippdev/ultralauncher
AppSupportURL=https://github.com/lippdev/ultralauncher/issues
VersionInfoVersion={#NumericVersion}
DefaultDirName={autopf}\Tako
DefaultGroupName=Tako
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0.19041
OutputDir={#OutputDir}
OutputBaseFilename=Tako-{#AppVersion}-Setup-x64
SetupIconFile=..\..\src\Launcher.UI.Wpf\Assets\Brand\tako.ico
UninstallDisplayIcon={app}\Tako.exe
UninstallDisplayName=Tako
LicenseFile=..\..\LICENSE
WizardStyle=modern
Compression=lzma2/ultra64
SolidCompression=yes
CloseApplications=yes

[Languages]
Name: "brazilianportuguese"; MessagesFile: "compiler:Languages\BrazilianPortuguese.isl"
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "{#SourceDir}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs; Excludes: "portable.txt"

[Icons]
Name: "{autoprograms}\Tako"; Filename: "{app}\Tako.exe"
Name: "{autodesktop}\Tako"; Filename: "{app}\Tako.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\Tako.exe"; Description: "{cm:LaunchProgram,Tako}"; Flags: nowait postinstall skipifsilent
