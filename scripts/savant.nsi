; =============================================================================
;  SAVANT TRADING ENGINE — NSIS Installer Script
;  scripts/savant.nsi
;
;  Build with:
;    makensis -DVERSION="0.5.0" -DSRC_DIR="dist/staging" -DOUT_DIR="dist" savant.nsi
;
;  Requirements:
;    - NSIS 3.x                  https://nsis.sourceforge.io
;    - NSIS Modern UI (MUI2)     Included with NSIS 3.x
;    - LogicLib                  Included with NSIS 3.x
; =============================================================================

; ── Compiler Settings ─────────────────────────────────────────────
Unicode true
ManifestDPIAware true
SetCompressor /SOLID lzma
SetCompressorDictSize 64
BrandingText "Savant Trading Engine ${VERSION}"
ShowInstDetails hide
ShowUnInstDetails hide

; ── Version Info ──────────────────────────────────────────────────
!ifndef VERSION
  !define VERSION "0.5.0"
!endif

!ifndef SRC_DIR
  !define SRC_DIR "dist/staging"
!endif

!ifndef OUT_DIR
  !define OUT_DIR "dist"
!endif

VIProductVersion "${VERSION}.0"
VIAddVersionKey "ProductName" "Savant Trading Engine"
VIAddVersionKey "ProductVersion" "${VERSION}"
VIAddVersionKey "FileVersion" "${VERSION}"
VIAddVersionKey "FileDescription" "AI-native autonomous crypto trading system"
VIAddVersionKey "LegalCopyright" "2026 Spencer Howell"
VIAddVersionKey "CompanyName" "Savant Trading"
VIAddVersionKey "OriginalFilename" "SavantTrading-${VERSION}-Setup.exe"

; ── Modern UI 2 (MUI2) ───────────────────────────────────────────
!include "MUI2.nsh"
!include "LogicLib.nsh"
!include "FileFunc.nsh"
!include "WinVer.nsh"

; ── Installer Attributes ─────────────────────────────────────────
Name "Savant Trading Engine ${VERSION}"
OutFile "${OUT_DIR}\SavantTrading-${VERSION}-x64-Setup.exe"
InstallDir "$PROGRAMFILES64\SavantTrading"
InstallDirRegKey HKLM "Software\SavantTrading" "InstallDir"
RequestExecutionLevel admin

; ── Interface Settings ───────────────────────────────────────────
!define MUI_HEADERIMAGE
!define MUI_HEADERIMAGE_BITMAP ""
!define MUI_HEADERIMAGE_RIGHT
!define MUI_ABORTWARNING
!define MUI_COMPONENTSPAGE_SMALLDESC
!define MUI_FINISHPAGE_RUN "$INSTDIR\savant.exe"
!define MUI_FINISHPAGE_RUN_TEXT "Start Savant Trading Engine"
!define MUI_FINISHPAGE_LINK "View documentation"
!define MUI_FINISHPAGE_LINK_LOCATION "https://github.com/spencer-thompson/savant-trading"
!define MUI_WELCOMEFINISHPAGE_BITMAP ""

; ── Pages ─────────────────────────────────────────────────────────
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE "${SRC_DIR}\docs\LICENSE"
!insertmacro MUI_PAGE_COMPONENTS
!insertmacro MUI_PAGE_DIRECTORY

; Custom data directory page
Page custom DataDirectoryPage DataDirectoryPageLeave

!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

; ── Uninstaller Pages ─────────────────────────────────────────────
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

; ── Language ──────────────────────────────────────────────────────
!insertmacro MUI_LANGUAGE "English"

; ── Reserve Files ─────────────────────────────────────────────────
!insertmacro MUI_RESERVEFILE_INSTALLOPTIONS

; ── Global Variables ─────────────────────────────────────────────
Var DataDir
Var InstallTuiShortcut

; ═══════════════════════════════════════════════════════════════════
;  SECTION: Main Application
; ═══════════════════════════════════════════════════════════════════
Section "Savant Trading Engine" SecMain
  SectionIn RO  ; Required section — always installed
  SetOutPath "$INSTDIR"

  ; ── Binary ──
  File "${SRC_DIR}\savant.exe"

  ; ── Config ──
  SetOutPath "$INSTDIR\config"
  File "${SRC_DIR}\config\default.toml"

  ; ── Knowledge Base ──
  SetOutPath "$INSTDIR\knowledge"
  File /nonfatal "${SRC_DIR}\knowledge\*.json"

  ; ── Documentation ──
  SetOutPath "$INSTDIR\docs"
  File /nonfatal "${SRC_DIR}\docs\*.md"

  ; ── Environment template ──
  SetOutPath "$INSTDIR"
  File "${SRC_DIR}\.env.example"

  ; ── Run scripts ──
  File "${SRC_DIR}\run.bat"
  File "${SRC_DIR}\run-tui.bat"

  ; ── VERSION file ──
  File "${SRC_DIR}\VERSION"

  ; ── Create data directory (user's choice or default) ──
  ${If} $DataDir == ""
    StrCpy $DataDir "$INSTDIR\data"
  ${EndIf}
  CreateDirectory "$DataDir"
  ; Persist data directory to registry so engine can read it at runtime
  WriteRegStr HKLM "Software\SavantTrading" "DataDir" "$DataDir"
  WriteUninstaller "$INSTDIR\Uninstall.exe"

  ; ── Registry: Install path ──
  WriteRegStr HKLM "Software\SavantTrading" "InstallDir" "$INSTDIR"
  WriteRegStr HKLM "Software\SavantTrading" "Version" "${VERSION}"
  WriteRegStr HKLM "Software\SavantTrading" "InstallDate" "$%DATE%"

  ; ── Registry: Add/Remove Programs ──
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\SavantTrading" \
    "DisplayName" "Savant Trading Engine ${VERSION}"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\SavantTrading" \
    "DisplayIcon" "$INSTDIR\savant.exe"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\SavantTrading" \
    "DisplayVersion" "${VERSION}"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\SavantTrading" \
    "Publisher" "Savant Trading"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\SavantTrading" \
    "URLInfoAbout" "https://github.com/spencer-thompson/savant-trading"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\SavantTrading" \
    "HelpLink" "https://github.com/spencer-thompson/savant-trading"
  WriteRegDWORD HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\SavantTrading" \
    "NoModify" 1
  WriteRegDWORD HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\SavantTrading" \
    "NoRepair" 1
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\SavantTrading" \
    "UninstallString" "$INSTDIR\Uninstall.exe"

  ; ── Calculate installed size ──
  ${GetSize} "$INSTDIR" "/S=0K" $0 $1 $2
  IntFmt $0 "0x%08X" $0
  WriteRegDWORD HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\SavantTrading" \
    "EstimatedSize" "$0"

SectionEnd


; ═══════════════════════════════════════════════════════════════════
;  SECTION: Start Menu & Desktop Shortcuts
; ═══════════════════════════════════════════════════════════════════
Section "Start Menu Shortcuts" SecShortcuts
  CreateDirectory "$SMPROGRAMS\Savant Trading Engine"

  ; Engine (default mode)
  CreateShortCut "$SMPROGRAMS\Savant Trading Engine\Savant Engine.lnk" \
    "$INSTDIR\savant.exe" "" "$INSTDIR\savant.exe" 0 \
    SW_SHOWNORMAL "" "Start Savant trading engine + API server"

  ; TUI Mode
  CreateShortCut "$SMPROGRAMS\Savant Trading Engine\Savant TUI (Terminal UI).lnk" \
    "$INSTDIR\savant.exe" "--tui" "$INSTDIR\savant.exe" 0 \
    SW_SHOWNORMAL "" "Start Savant with full-screen multi-tab TUI"

  ; Documentation
  CreateShortCut "$SMPROGRAMS\Savant Trading Engine\Documentation.lnk" \
    "$INSTDIR\docs\README.md" "" "$INSTDIR\docs\README.md" 0

  ; API-KEYS guide
  CreateShortCut "$SMPROGRAMS\Savant Trading Engine\API Keys.lnk" \
    "$INSTDIR\docs\API-KEYS.md" "" "$INSTDIR\docs\API-KEYS.md" 0

  ; Config folder
  CreateShortCut "$SMPROGRAMS\Savant Trading Engine\Configuration.lnk" \
    "$INSTDIR\config" "" "$INSTDIR\config" 0

  ; Data folder
  CreateShortCut "$SMPROGRAMS\Savant Trading Engine\Data (runtime).lnk" \
    "$INSTDIR\data" "" "$INSTDIR\data" 0

  ; Uninstaller
  CreateShortCut "$SMPROGRAMS\Savant Trading Engine\Uninstall.lnk" \
    "$INSTDIR\Uninstall.exe" "" "$INSTDIR\Uninstall.exe" 0

  ; Add to PATH for command-line access
  Push "$INSTDIR"
  Call AddToPath
SectionEnd


; ═══════════════════════════════════════════════════════════════════
;  SECTION: Desktop Shortcut (optional, duplicates for component clarity)
; ═══════════════════════════════════════════════════════════════════
Section /o "Desktop Shortcut" SecDesktop
  ; Desktop shortcut launches TUI mode (most user-facing experience)
  CreateShortCut "$DESKTOP\Savant Trading Engine.lnk" \
    "$INSTDIR\savant.exe" "--tui" "$INSTDIR\savant.exe" 0 \
    SW_SHOWNORMAL "" "Savant Trading Engine (TUI Mode)"
SectionEnd


; ═══════════════════════════════════════════════════════════════════
;  SECTION: .env file association (optional)
; ═══════════════════════════════════════════════════════════════════
Section /o "Associate .env files" SecEnvAssoc
  ; Register .env file type
  WriteRegStr HKCR ".env" "" "SavantEnvFile"
  WriteRegStr HKCR "SavantEnvFile" "" "Environment Configuration File"
  WriteRegStr HKCR "SavantEnvFile\DefaultIcon" "" "$INSTDIR\savant.exe,0"
  WriteRegStr HKCR "SavantEnvFile\shell\open\command" "" "notepad.exe %1"
SectionEnd


; ── Section Descriptions ─────────────────────────────────────────
!insertmacro MUI_FUNCTION_DESCRIPTION_BEGIN
  !insertmacro MUI_DESCRIPTION_TEXT ${SecMain} \
    "Core application: trading engine binary, configuration, knowledge base, and documentation."
  !insertmacro MUI_DESCRIPTION_TEXT ${SecShortcuts} \
    "Start Menu shortcuts for engine, TUI mode, documentation, and configuration folders. Also adds Savant to system PATH."
  !insertmacro MUI_DESCRIPTION_TEXT ${SecDesktop} \
    "Desktop shortcut that launches Savant in full-screen TUI mode."
  !insertmacro MUI_DESCRIPTION_TEXT ${SecEnvAssoc} \
    "Associate .env configuration files with Notepad for easy editing."
!insertmacro MUI_FUNCTION_DESCRIPTION_END


; ═══════════════════════════════════════════════════════════════════
;  CUSTOM PAGE: Data Directory
; ═══════════════════════════════════════════════════════════════════
Function DataDirectoryPage
  !insertmacro MUI_HEADER_TEXT "Data Directory" \
    "Choose where runtime data (logs, databases, trade history) will be stored."

  nsDialogs::Create 1018
  Pop $0

  ${If} $0 == error
    Abort
  ${EndIf}

  ${NSD_CreateLabel} 0u 0u 100% 12u \
    "Runtime data (SQLite databases, logs, cached market data) will be stored in:"
  Pop $0

  ${NSD_CreateText} 0u 16u 80% 12u "$INSTDIR\data"
  Pop $DataDir

  ${NSD_CreateButton} 82% 14u 18% 14u "Browse..."
  Pop $1
  ${NSD_OnClick} $1 DataDirBrowse

  nsDialogs::Show
FunctionEnd

Function DataDirBrowse
  nsDialogs::SelectFolderDialog "Select Data Directory" "$INSTDIR\data"
  Pop $0
  ${If} $0 != "error"
    ${NSD_SetText} $DataDir $0
  ${EndIf}
FunctionEnd

Function DataDirectoryPageLeave
  ${NSD_GetText} $DataDir $0
  ${If} $0 == ""
    MessageBox MB_ICONEXCLAMATION "Please select a data directory."
    Abort
  ${EndIf}
  StrCpy $DataDir $0
FunctionEnd


; ═══════════════════════════════════════════════════════════════════
;  INSTALLER INIT — Pre-install checks
; ═══════════════════════════════════════════════════════════════════
Function .onInit
  ; Require Windows 10 or later
  ${IfNot} ${AtLeastWin10}
    MessageBox MB_ICONSTOP \
      "Savant Trading Engine requires Windows 10 or later."
    Abort
  ${EndIf}

  ; Check if already installed and offer to uninstall first
  ReadRegStr $R0 HKLM \
    "Software\Microsoft\Windows\CurrentVersion\Uninstall\SavantTrading" \
    "UninstallString"
  StrCmp $R0 "" done

  MessageBox MB_ICONQUESTION|MB_YESNO|MB_DEFBUTTON2 \
    "Savant Trading Engine is already installed.$\n$\nWould you like to uninstall the previous version first?" \
    IDYES uninstall
  Goto done

  uninstall:
    ClearErrors
    ExecWait '$R0 _?=$INSTDIR'

  done:
FunctionEnd


; ═══════════════════════════════════════════════════════════════════
;  UNINSTALLER
; ═══════════════════════════════════════════════════════════════════
Section "Uninstall"
  ; ── Ensure DataDir has a default fallback ──
  ${If} $DataDir == ""
    StrCpy $DataDir "$INSTDIR\data"
  ${EndIf}

  ; ── Remove shortcuts ──
  RMDir /r "$SMPROGRAMS\Savant Trading Engine"
  Delete "$DESKTOP\Savant Trading Engine.lnk"

  ; ── Remove PATH entries ──
  Push "$INSTDIR"
  Call un.RemoveFromPath

  ; ── Remove registry keys ──
  DeleteRegKey HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\SavantTrading"
  DeleteRegKey HKLM "Software\SavantTrading"
  DeleteRegKey HKCR ".env"
  DeleteRegKey HKCR "SavantEnvFile"

  ; ── Remove installed files ──
  RMDir /r "$INSTDIR"

  ; ── Ensure data directory is removed (prompt user first) ──
  IfFileExists "$DataDir\*.*" 0 skip_data_removal
    MessageBox MB_ICONQUESTION|MB_YESNO|MB_DEFBUTTON2 \
      "Remove runtime data directory as well?$\n$\n$DataDir$\n$\nThis will delete all databases, logs, and cached data." \
      IDNO skip_data_removal
    RMDir /r "$DataDir"
  skip_data_removal:

SectionEnd


; ═══════════════════════════════════════════════════════════════════
;  HELPER FUNCTIONS: PATH Management
; ═══════════════════════════════════════════════════════════════════
;
;  AddToPath / RemoveFromPath — modifies system PATH via registry
;  Based on NSIS documentation example: PathManipulation.nsi
; ═══════════════════════════════════════════════════════════════════

!define ENV_HKLM "HKLM\SYSTEM\CurrentControlSet\Control\Session Manager\Environment"

Function AddToPath
  Exch $0
  Push $1
  Push $2
  Push $3

  ; Don't add if already in PATH
  ReadRegStr $1 ${ENV_HKLM} "Path"
  Push "$0;"
  Push "$1"
  Call StrStr
  Pop $2
  StrCmp $2 "" add_it
    Pop $0
    Pop $1
    Pop $2
    Pop $3
    Return

  add_it:
    StrCpy $2 "$1;$0"
    WriteRegExpandStr ${ENV_HKLM} "Path" $2
    SendMessage ${HWND_BROADCAST} ${WM_SETTINGCHANGE} 0 "STR:Environment" /TIMEOUT=5000

  Pop $0
  Pop $1
  Pop $2
  Pop $3
FunctionEnd


Function un.RemoveFromPath
  Exch $0
  Push $1
  Push $2
  Push $3
  Push $4
  Push $5
  Push $6

  ReadRegStr $1 ${ENV_HKLM} "Path"
  StrCpy $5 $1 1 0
  StrCmp $5 ";" 0 next_check
    StrCpy $1 $1 "" 1

  next_check:
    StrCpy $5 $1 1 -1
    StrCmp $5 ";" 0 trim_loop
      StrCpy $1 $1 -1
      Goto next_check

  trim_loop:
    Push "$0;"
    Push "$1"
    Call un.StrStr
    Pop $2
    StrCmp $2 "" done
      StrLen $3 "$0"
      StrLen $4 $2
      StrCpy $5 $1 -$4
      IntOp $3 $3 + 1
      StrCpy $6 $2 "" $3
      StrCpy $1 "$5$6"
      Goto trim_loop

  done:
    WriteRegExpandStr ${ENV_HKLM} "Path" $1
    SendMessage ${HWND_BROADCAST} ${WM_SETTINGCHANGE} 0 "STR:Environment" /TIMEOUT=5000

  Pop $6
  Pop $5
  Pop $4
  Pop $3
  Pop $2
  Pop $1
  Pop $0
FunctionEnd


; ── String search helper ──────────────────────────────────────────
Function StrStr
  Exch $1
  Exch
  Exch $0
  Push $2
  Push $3
  Push $4
  Push $5

  StrCpy $2 -1
  StrLen $3 $0
  StrLen $4 $1
  IntOp $4 $4 - $3
  ${If} $4 < 0
    Goto done
  ${EndIf}

  loop:
    IntOp $2 $2 + 1
    ${If} $2 > $4
      Goto done
    ${EndIf}
    StrCpy $5 $1 $3 $2
    ${If} $5 == $0
      Goto found
    ${EndIf}
    Goto loop

  found:
    StrCpy $0 $1 "" $2
    Pop $5
    Pop $4
    Pop $3
    Pop $2
    Pop $1
    Exch $0
    Return

  done:
    StrCpy $0 ""
    Pop $5
    Pop $4
    Pop $3
    Pop $2
    Pop $1
    Pop $1
    Exch $0
FunctionEnd


Function un.StrStr
  Exch $1
  Exch
  Exch $0
  Push $2
  Push $3
  Push $4
  Push $5

  StrCpy $2 -1
  StrLen $3 $0
  StrLen $4 $1
  IntOp $4 $4 - $3
  ${If} $4 < 0
    Goto done
  ${EndIf}

  loop:
    IntOp $2 $2 + 1
    ${If} $2 > $4
      Goto done
    ${EndIf}
    StrCpy $5 $1 $3 $2
    ${If} $5 == $0
      Goto found
    ${EndIf}
    Goto loop

  found:
    StrCpy $0 $1 "" $2
    Pop $5
    Pop $4
    Pop $3
    Pop $2
    Pop $1
    Exch $0
    Return

  done:
    StrCpy $0 ""
    Pop $5
    Pop $4
    Pop $3
    Pop $2
    Pop $1
    Pop $1
    Exch $0
FunctionEnd
