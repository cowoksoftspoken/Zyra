; Zyra Programming Language Installer
; Inno Setup Script
; 
; Requirements:
;   - Inno Setup 6.x (https://jrsoftware.org/isdl.php)
;   - Zyra binary (built using cargo build --release)
;
; To build: Open this file in Inno Setup Compiler and click Build > Compile

#define MyAppName "Zyra Programming Language"
#define MyAppVersion "1.0.2"
#define MyAppPublisher "Inggrit Setya Budi"
#define MyAppURL "https://github.com/cowoksoftspoken/Zyra"
#define MyAppExeName "zyra.exe"

[Setup]
; Application Info
AppId={{B7C8D9E0-F1A2-4B3C-5D6E-7F8A9B0C1D2E}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppVerName={#MyAppName} {#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}/releases
DefaultDirName={autopf}\Zyra
DefaultGroupName=Zyra
AllowNoIcons=yes
LicenseFile=..\LICENSE
OutputDir=..\dist
OutputBaseFilename=ZyraSetup-{#MyAppVersion}
SetupIconFile=..\extensions\ZyraFileIcons\icons\zyra.ico
UninstallDisplayIcon={app}\icons\zyra.ico
Compression=lzma2/ultra64
SolidCompression=yes
WizardStyle=modern
PrivilegesRequired=admin
ArchitecturesInstallIn64BitMode=x64compatible
ChangesEnvironment=yes

; Appearance - using modern built-in wizard images (removed deprecated directives)

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"
Name: "addtopath"; Description: "Add Zyra to system PATH"; GroupDescription: "Environment:"; Flags: checkedonce

[Files]
; Main binary (rename to zyra.exe on install)
Source: "..\installer\bin\windows\zyra.exe"; DestDir: "{app}\bin"; DestName: "zyra.exe"; Flags: ignoreversion

; Icons folder
Source: "..\extensions\ZyraFileIcons\icons\*"; DestDir: "{app}\icons"; Flags: ignoreversion recursesubdirs createallsubdirs

; Installer scripts (for manual use)
Source: "..\installer\*"; DestDir: "{app}\installer"; Flags: ignoreversion recursesubdirs createallsubdirs; Excludes: "*.iss"

; Documentation
Source: "..\README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\LICENSE"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\Zyra Command Prompt"; Filename: "{cmd}"; Parameters: "/K ""{app}\bin\zyra.exe"" --version"; WorkingDir: "{app}"; IconFilename: "{app}\icons\zyra.ico"
Name: "{group}\Zyra Documentation"; Filename: "{app}\README.md"
Name: "{group}\{cm:UninstallProgram,{#MyAppName}}"; Filename: "{uninstallexe}"; IconFilename: "{app}\icons\zyra.ico"
Name: "{autodesktop}\Zyra"; Filename: "{cmd}"; Parameters: "/K ""{app}\bin\zyra.exe"" --help"; WorkingDir: "{app}"; IconFilename: "{app}\icons\zyra.ico"; Tasks: desktopicon

[Registry]
; Add to PATH
Root: HKLM; Subkey: "SYSTEM\CurrentControlSet\Control\Session Manager\Environment"; ValueType: expandsz; ValueName: "Path"; ValueData: "{olddata};{app}\bin"; Tasks: addtopath; Check: NeedsAddPath(ExpandConstant('{app}\bin'))

[Run]
Filename: "{app}\bin\zyra.exe"; Parameters: "--version"; Description: "Verify installation"; Flags: postinstall nowait skipifsilent shellexec

[Code]
// Check if path needs to be added
function NeedsAddPath(Param: string): boolean;
var
  OrigPath: string;
begin
  if not RegQueryStringValue(HKLM, 'SYSTEM\CurrentControlSet\Control\Session Manager\Environment', 'Path', OrigPath)
  then begin
    Result := True;
    exit;
  end;
  // Look for the path with leading and trailing semicolon
  Result := Pos(';' + Param + ';', ';' + OrigPath + ';') = 0;
end;

// Remove from PATH on uninstall
procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
var
  Path: string;
  AppPath: string;
  P: Integer;
begin
  if CurUninstallStep = usPostUninstall then
  begin
    if RegQueryStringValue(HKLM, 'SYSTEM\CurrentControlSet\Control\Session Manager\Environment', 'Path', Path) then
    begin
      AppPath := ExpandConstant('{app}\bin');
      P := Pos(';' + AppPath, Path);
      if P = 0 then
        P := Pos(AppPath + ';', Path);
      if P <> 0 then
      begin
        Delete(Path, P, Length(AppPath) + 1);
        RegWriteStringValue(HKLM, 'SYSTEM\CurrentControlSet\Control\Session Manager\Environment', 'Path', Path);
      end;
    end;
  end;
end;

[Messages]
WelcomeLabel1=Welcome to the Zyra Setup Wizard
WelcomeLabel2=This will install [name/ver] on your computer.%n%nZyra is a modern, memory-safe programming language with ownership semantics - built for performance, safety, and simplicity.%n%nIt is recommended that you close all other applications before continuing.
FinishedHeadingLabel=Completing the Zyra Setup Wizard
FinishedLabel=Zyra has been installed on your computer.%n%nOpen a new terminal and type "zyra --version" to verify the installation.%n%nHappy coding!
