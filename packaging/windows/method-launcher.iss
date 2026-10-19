; The Windows installer (Inno Setup 6). `cargo xtask package-windows` runs it as
;   ISCC /DAppVersion=<version> /DAppNumericVersion=<X.Y.Z> /DSourceDir=<repo root> /DOutputDir=<folder> method-launcher.iss
; and it writes Method-Launcher-windows-x64-setup.exe (a version-free name, so the website
; can link releases/latest/download/<file>).
;
; Installs for the current user by default (no administrator prompt); the first page offers
; all users instead. Uninstall from Settings > Apps.

#ifndef AppVersion
  #error Pass /DAppVersion=<version> /DAppNumericVersion=<X.Y.Z>
#endif
#ifndef SourceDir
  #define SourceDir "..\.."
#endif
#ifndef OutputDir
  #define OutputDir SourceDir + "\target\bundle\windows"
#endif

[Setup]
; Never change AppId: Windows uses it to find the installed copy when updating or removing it.
AppId={{A03F8A0C-C2A9-4F7D-8A3A-85521D927141}
AppName=Method Launcher
AppVersion={#AppVersion}
AppVerName=Method Launcher {#AppVersion}
AppPublisher=Method Production Suite
AppPublisherURL=https://methodproduction.com
AppSupportURL=https://github.com/method-production-suite/method-launcher/issues
AppUpdatesURL=https://github.com/method-production-suite/method-launcher/releases
AppCopyright=Copyright (c) 2026 Method Production Suite Team and the Method Launcher contributors
VersionInfoVersion={#AppNumericVersion}
DefaultDirName={autopf}\Method Launcher
DefaultGroupName=Method Launcher
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
OutputDir={#OutputDir}
OutputBaseFilename=Method-Launcher-windows-x64-setup
SetupIconFile={#SourceDir}\packaging\icons\icon.ico
UninstallDisplayIcon={app}\method-launcher-app.exe
UninstallDisplayName=Method Launcher
LicenseFile={#SourceDir}\LICENSE-MIT
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "{#SourceDir}\target\release\method-launcher-app.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceDir}\LICENSE-MIT"; DestDir: "{app}"; DestName: "LICENSE-MIT.txt"; Flags: ignoreversion
Source: "{#SourceDir}\LICENSE-APACHE"; DestDir: "{app}"; DestName: "LICENSE-APACHE.txt"; Flags: ignoreversion
Source: "{#SourceDir}\NOTICE"; DestDir: "{app}"; DestName: "NOTICE.txt"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\Method Launcher"; Filename: "{app}\method-launcher-app.exe"
Name: "{autodesktop}\Method Launcher"; Filename: "{app}\method-launcher-app.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\method-launcher-app.exe"; Description: "{cm:LaunchProgram,Method Launcher}"; Flags: nowait postinstall skipifsilent
