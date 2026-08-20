; Inno Setup Compiler Script for Raven Notch
; Generates a professional Windows Installer (.exe) for the desktop app.

#define MyAppName "Raven Notch"
#define MyAppVersion "0.1.5"
#define MyAppPublisher "Raven Notch"
#define MyAppURL "https://ravennotch.me"
#define SourceExeName "Raven-Notch.exe"
#define DestExeName "Raven Notch.exe"

[Setup]
; Unique GUID for Raven Notch installer identification
AppId={{D90A382F-1736-4B83-A0A6-7EE94EBE2004}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}

; Install in user's Local AppData (does not require admin privileges, building maximum trust)
DefaultDirName={localappdata}\Programs\{#MyAppName}
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
Uninstallable=yes
UninstallFilesDir={app}\uninstall
UninstallDisplayIcon={app}\{#DestExeName}

; Output file setup
OutputDir=target
OutputBaseFilename=Raven-Notch
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern

; Visual branding & Icons
SetupIconFile=app.ico
CloseApplications=yes

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
; Source binary
Source: "target\release\{#SourceExeName}"; DestDir: "{app}"; DestName: "{#DestExeName}"; Flags: ignoreversion

[Icons]
; Start Menu and Desktop Shortcuts
Name: "{userprograms}\{#MyAppName}"; Filename: "{app}\{#DestExeName}"
Name: "{userdesktop}\{#MyAppName}"; Filename: "{app}\{#DestExeName}"; Tasks: desktopicon

[Registry]
Root: HKCU; Subkey: "Software\Classes\ravennotch"; ValueType: string; ValueName: ""; ValueData: "URL:Raven Notch"; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Classes\ravennotch"; ValueType: string; ValueName: "URL Protocol"; ValueData: ""
Root: HKCU; Subkey: "Software\Classes\ravennotch\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\{#DestExeName},0"
Root: HKCU; Subkey: "Software\Classes\ravennotch\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#DestExeName}"" ""%1"""

[Run]
; Auto-launch the application post-install
Filename: "{app}\{#DestExeName}"; Description: "{cm:LaunchProgram,{#StringChange(MyAppName, '&', '&&')}}"; Flags: nowait postinstall
