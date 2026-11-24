; Dustr installer (Inno Setup 6). Build: ISCC /DAppVersion=x.y.z installer\dustr.iss
; Per-user install, no admin rights. Start menu shortcut always; desktop shortcut optional (off).

#ifndef AppVersion
  #define AppVersion "0.0.0"
#endif
#define AppExe "Dustr.exe"

[Setup]
AppId={{8E3C1A52-6B4F-4C2E-9D7A-2F1B5C9E7A31}
AppName=Dustr
AppVersion={#AppVersion}
AppVerName=Dustr {#AppVersion}
AppPublisher=Dustr contributors
AppPublisherURL=https://github.com/TETRA326/dustr
AppSupportURL=https://github.com/TETRA326/dustr/issues
DefaultDirName={autopf}\Dustr
DefaultGroupName=Dustr
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
WizardStyle=modern
WizardSmallImageFile=small-55.bmp,small-110.bmp
WizardImageFile=large-100.bmp,large-200.bmp
SetupIconFile=..\src-tauri\icons\icon.ico
UninstallDisplayIcon={app}\{#AppExe}
UninstallDisplayName=Dustr
; Keep it short: shortcuts choice -> install -> finish.
DisableWelcomePage=yes
DisableDirPage=yes
DisableProgramGroupPage=yes
DisableReadyPage=yes
CloseApplications=yes
Compression=lzma2/max
SolidCompression=yes
OutputDir=..\dist
; Fixed name so releases/latest/download/Dustr-Setup.exe always works.
OutputBaseFilename=Dustr-Setup

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "Create a desktop shortcut"; Flags: unchecked

[Files]
Source: "..\src-tauri\target\release\{#AppExe}"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\Dustr"; Filename: "{app}\{#AppExe}"
Name: "{autodesktop}\Dustr"; Filename: "{app}\{#AppExe}"; Tasks: desktopicon

[Registry]
; Classic Explorer context menu ("Show more options" on Windows 11), for folders,
; folder backgrounds and drives.
Root: HKCU; Subkey: "Software\Classes\Directory\shell\Dustr"; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Classes\Directory\shell\Dustr"; ValueType: string; ValueName: "MUIVerb"; ValueData: "Dustr"
Root: HKCU; Subkey: "Software\Classes\Directory\shell\Dustr"; ValueType: string; ValueName: "Icon"; ValueData: "{app}\{#AppExe}"
Root: HKCU; Subkey: "Software\Classes\Directory\shell\Dustr"; ValueType: string; ValueName: "SubCommands"; ValueData: ""
Root: HKCU; Subkey: "Software\Classes\Directory\shell\Dustr\shell\1dir"; ValueType: string; ValueName: "MUIVerb"; ValueData: "Dust only this directory"
Root: HKCU; Subkey: "Software\Classes\Directory\shell\Dustr\shell\1dir\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#AppExe}"" --dir ""%V"""
Root: HKCU; Subkey: "Software\Classes\Directory\shell\Dustr\shell\2rec"; ValueType: string; ValueName: "MUIVerb"; ValueData: "Dust all recursive folders"
Root: HKCU; Subkey: "Software\Classes\Directory\shell\Dustr\shell\2rec\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#AppExe}"" --recursive ""%V"""
Root: HKCU; Subkey: "Software\Classes\Directory\Background\shell\Dustr"; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Classes\Directory\Background\shell\Dustr"; ValueType: string; ValueName: "MUIVerb"; ValueData: "Dustr"
Root: HKCU; Subkey: "Software\Classes\Directory\Background\shell\Dustr"; ValueType: string; ValueName: "Icon"; ValueData: "{app}\{#AppExe}"
Root: HKCU; Subkey: "Software\Classes\Directory\Background\shell\Dustr"; ValueType: string; ValueName: "SubCommands"; ValueData: ""
Root: HKCU; Subkey: "Software\Classes\Directory\Background\shell\Dustr\shell\1dir"; ValueType: string; ValueName: "MUIVerb"; ValueData: "Dust only this directory"
Root: HKCU; Subkey: "Software\Classes\Directory\Background\shell\Dustr\shell\1dir\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#AppExe}"" --dir ""%V"""
Root: HKCU; Subkey: "Software\Classes\Directory\Background\shell\Dustr\shell\2rec"; ValueType: string; ValueName: "MUIVerb"; ValueData: "Dust all recursive folders"
Root: HKCU; Subkey: "Software\Classes\Directory\Background\shell\Dustr\shell\2rec\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#AppExe}"" --recursive ""%V"""
Root: HKCU; Subkey: "Software\Classes\Drive\shell\Dustr"; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Classes\Drive\shell\Dustr"; ValueType: string; ValueName: "MUIVerb"; ValueData: "Dustr"
Root: HKCU; Subkey: "Software\Classes\Drive\shell\Dustr"; ValueType: string; ValueName: "Icon"; ValueData: "{app}\{#AppExe}"
Root: HKCU; Subkey: "Software\Classes\Drive\shell\Dustr"; ValueType: string; ValueName: "SubCommands"; ValueData: ""
Root: HKCU; Subkey: "Software\Classes\Drive\shell\Dustr\shell\1dir"; ValueType: string; ValueName: "MUIVerb"; ValueData: "Dust only this directory"
Root: HKCU; Subkey: "Software\Classes\Drive\shell\Dustr\shell\1dir\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#AppExe}"" --dir ""%V"""
Root: HKCU; Subkey: "Software\Classes\Drive\shell\Dustr\shell\2rec"; ValueType: string; ValueName: "MUIVerb"; ValueData: "Dust all recursive folders"
Root: HKCU; Subkey: "Software\Classes\Drive\shell\Dustr\shell\2rec\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#AppExe}"" --recursive ""%V"""

[Run]
Filename: "{app}\{#AppExe}"; Description: "Open Dustr"; Flags: nowait postinstall skipifsilent

[Code]
// Remove a previous NSIS-based install (Dustr 0.1.x) so the two don't overlap.
procedure RemoveOldNsisInstall();
var
  Uninst, Dir: String;
  Code: Integer;
begin
  if RegQueryStringValue(HKCU, 'Software\Microsoft\Windows\CurrentVersion\Uninstall\Dustr', 'UninstallString', Uninst) then
  begin
    Uninst := RemoveQuotes(Uninst);
    Dir := ExtractFileDir(Uninst);
    if FileExists(Uninst) then
    begin
      // _?= runs the NSIS uninstaller in place and waits for it to finish.
      Exec(Uninst, '/S _?=' + Dir, '', SW_HIDE, ewWaitUntilTerminated, Code);
      DeleteFile(Uninst);
      RemoveDir(Dir);
    end;
  end;
end;

procedure CurStepChanged(CurStep: TSetupStep);
begin
  if CurStep = ssInstall then
    RemoveOldNsisInstall();
end;
