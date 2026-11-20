; tTorrent installer for Windows x64.
; Copyright (C) 2026  t21 dev
;
; This program is free software; you can redistribute it and/or
; modify it under the terms of the GNU General Public License
; as published by the Free Software Foundation; either version 2
; of the License, or (at your option) any later version.
;
; Built by ttorrent\scripts\make-installer.ps1, which passes:
;   TT_VERSION   the version, e.g. 0.5.1
;   TT_DIST      the staged release folder (build\Release plus COPYING.txt)
;   TT_FILES     an include with a File line for every staged file
;   TT_UNFILES   an include with a Delete line for every staged file
;   TT_OUT       the setup .exe to write
;
; It installs for the current user only (no admin prompt) into
; %LOCALAPPDATA%\Programs\tTorrent, the same place install-local.ps1 uses.
; Nothing here touches qBittorrent's registry keys, files or settings
; (dist\windows\qbittorrent.nsi is upstream's installer and stays as it is).
; The registry names match src\gui\dose\doseassoc.cpp.

Unicode true
ManifestDPIAware true
ManifestDPIAwareness "PerMonitorV2,System"
SetCompressor /SOLID LZMA
SetCompressorDictSize 64
RequestExecutionLevel user

!include "MUI2.nsh"
!include "x64.nsh"
!include "FileFunc.nsh"

!define APP "tTorrent"
!define UNINST_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\tTorrent"
!define ICON "..\..\src\icons\qbittorrent.ico"

Name "${APP} ${TT_VERSION}"
OutFile "${TT_OUT}"
InstallDir "$LOCALAPPDATA\Programs\tTorrent"
InstallDirRegKey HKCU "${UNINST_KEY}" "InstallLocation"
BrandingText "${APP} ${TT_VERSION}"

VIProductVersion "${TT_VERSION}.0"
VIAddVersionKey "ProductName" "${APP}"
VIAddVersionKey "ProductVersion" "${TT_VERSION}"
VIAddVersionKey "FileVersion" "${TT_VERSION}"
VIAddVersionKey "FileDescription" "${APP} installer"
VIAddVersionKey "CompanyName" "t21 dev"
VIAddVersionKey "LegalCopyright" "© 2026 t21 dev. GPL."

!define MUI_ICON "${ICON}"
!define MUI_UNICON "${ICON}"
!define MUI_ABORTWARNING
!define MUI_COMPONENTSPAGE_NODESC
!define MUI_FINISHPAGE_RUN "$INSTDIR\ttorrent.exe"
!define MUI_FINISHPAGE_RUN_TEXT "Start tTorrent"

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE "${TT_DIST}\COPYING.txt"
!insertmacro MUI_PAGE_COMPONENTS
!define MUI_PAGE_CUSTOMFUNCTION_LEAVE CheckRunning
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

!define MUI_PAGE_CUSTOMFUNCTION_LEAVE un.CheckRunning
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

!insertmacro MUI_LANGUAGE "English"

; Asks the person to close tTorrent until it isn't running, or to cancel.
!macro CHECK_RUNNING un
Function ${un}CheckRunning
  retry:
    nsExec::ExecToStack 'cmd /c tasklist /FI "IMAGENAME eq ttorrent.exe" /NH | find /I "ttorrent.exe"'
    Pop $0
    Pop $1
    StrCmp $0 "0" 0 done
    MessageBox MB_RETRYCANCEL|MB_ICONEXCLAMATION "tTorrent is running. Close it (File › Exit or the tray menu), then click Retry." IDRETRY retry
    Abort
  done:
FunctionEnd
!macroend
!insertmacro CHECK_RUNNING ""
!insertmacro CHECK_RUNNING "un."

Function .onInit
  ${IfNot} ${RunningX64}
    MessageBox MB_OK|MB_ICONSTOP "tTorrent needs 64-bit Windows."
    Abort
  ${EndIf}
FunctionEnd

Section "tTorrent (required)" SecApp
  SectionIn RO
  SetOutPath "$INSTDIR"
  ; Leftovers from install-local.ps1, which renames in-use files aside.
  Delete "$INSTDIR\ttorrent-old*.exe"
  !include "${TT_FILES}"
  WriteUninstaller "$INSTDIR\uninstall.exe"

  CreateShortCut "$SMPROGRAMS\tTorrent.lnk" "$INSTDIR\ttorrent.exe"

  WriteRegStr HKCU "${UNINST_KEY}" "DisplayName" "${APP}"
  WriteRegStr HKCU "${UNINST_KEY}" "DisplayVersion" "${TT_VERSION}"
  WriteRegStr HKCU "${UNINST_KEY}" "DisplayIcon" '"$INSTDIR\ttorrent.exe",0'
  WriteRegStr HKCU "${UNINST_KEY}" "Publisher" "t21 dev"
  WriteRegStr HKCU "${UNINST_KEY}" "URLInfoAbout" "https://ttorrent.t21.dev"
  WriteRegStr HKCU "${UNINST_KEY}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "${UNINST_KEY}" "UninstallString" '"$INSTDIR\uninstall.exe"'
  WriteRegStr HKCU "${UNINST_KEY}" "QuietUninstallString" '"$INSTDIR\uninstall.exe" /S'
  WriteRegDWORD HKCU "${UNINST_KEY}" "NoModify" 1
  WriteRegDWORD HKCU "${UNINST_KEY}" "NoRepair" 1
  ${GetSize} "$INSTDIR" "/S=0K" $0 $1 $2
  IntFmt $0 "0x%08X" $0
  WriteRegDWORD HKCU "${UNINST_KEY}" "EstimatedSize" "$0"
SectionEnd

Section "Desktop shortcut" SecDesktop
  CreateShortCut "$DESKTOP\tTorrent.lnk" "$INSTDIR\ttorrent.exe"
SectionEnd

; Lists tTorrent in Windows' Default apps for .torrent files and magnet links.
; Windows still asks the person before it becomes the default.
Section "Offer tTorrent for .torrent files and magnet links" SecAssoc
  WriteRegStr HKCU "Software\Classes\tTorrent.torrent" "" "Torrent file"
  WriteRegStr HKCU "Software\Classes\tTorrent.torrent\DefaultIcon" "" '"$INSTDIR\ttorrent.exe",1'
  WriteRegStr HKCU "Software\Classes\tTorrent.torrent\shell\open\command" "" '"$INSTDIR\ttorrent.exe" "%1"'
  WriteRegStr HKCU "Software\Classes\.torrent\OpenWithProgids" "tTorrent.torrent" ""
  WriteRegStr HKCU "Software\Classes\tTorrent.magnet" "" "Magnet link"
  WriteRegStr HKCU "Software\Classes\tTorrent.magnet" "URL Protocol" ""
  WriteRegStr HKCU "Software\Classes\tTorrent.magnet\DefaultIcon" "" '"$INSTDIR\ttorrent.exe",0'
  WriteRegStr HKCU "Software\Classes\tTorrent.magnet\shell\open\command" "" '"$INSTDIR\ttorrent.exe" "%1"'
  WriteRegStr HKCU "Software\tTorrent\Capabilities" "ApplicationName" "tTorrent"
  WriteRegStr HKCU "Software\tTorrent\Capabilities" "ApplicationDescription" "A fast, modern torrent client"
  WriteRegStr HKCU "Software\tTorrent\Capabilities" "ApplicationIcon" '"$INSTDIR\ttorrent.exe",0'
  WriteRegStr HKCU "Software\tTorrent\Capabilities\FileAssociations" ".torrent" "tTorrent.torrent"
  WriteRegStr HKCU "Software\tTorrent\Capabilities\URLAssociations" "magnet" "tTorrent.magnet"
  WriteRegStr HKCU "Software\RegisteredApplications" "tTorrent" "Software\tTorrent\Capabilities"
  System::Call 'shell32::SHChangeNotify(i 0x8000000, i 0, p 0, p 0)'
SectionEnd

; Removes only what the installer put there. Torrents, settings and downloads
; (the profile in %APPDATA% and %LOCALAPPDATA%) stay.
Section "Uninstall"
  !include "${TT_UNFILES}"
  Delete "$INSTDIR\ttorrent-old*.exe"
  Delete "$INSTDIR\uninstall.exe"
  RMDir "$INSTDIR"

  Delete "$SMPROGRAMS\tTorrent.lnk"
  Delete "$DESKTOP\tTorrent.lnk"

  DeleteRegKey HKCU "Software\Classes\tTorrent.torrent"
  DeleteRegKey HKCU "Software\Classes\tTorrent.magnet"
  DeleteRegValue HKCU "Software\Classes\.torrent\OpenWithProgids" "tTorrent.torrent"
  DeleteRegKey HKCU "Software\tTorrent\Capabilities"
  DeleteRegKey /ifempty HKCU "Software\tTorrent"
  DeleteRegValue HKCU "Software\RegisteredApplications" "tTorrent"
  DeleteRegKey HKCU "${UNINST_KEY}"
  System::Call 'shell32::SHChangeNotify(i 0x8000000, i 0, p 0, p 0)'
SectionEnd
