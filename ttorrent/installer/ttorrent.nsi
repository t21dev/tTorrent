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
; It installs for all users into Program Files (one UAC prompt at the start),
; the same place install-local.ps1 uses. The program and its uninstall entry
; are per machine (HKLM); what the person chooses (opening magnet links and
; .torrent files, starting with Windows) stays per user (HKCU), in the same
; keys the app itself writes. Nothing here touches qBittorrent's registry
; keys, files or settings (dist\windows\qbittorrent.nsi is upstream's
; installer and stays as it is). The registry names match
; src\gui\dose\doseassoc.cpp.
;
; Up to 0.6.4 tTorrent installed per user into %LOCALAPPDATA%\Programs\tTorrent.
; An install finds such a copy and moves it here (see SecMove): the old files
; go, what the person had set up carries over to the new path, and torrents
; and settings (the profile) are never touched.

Unicode true
ManifestDPIAware true
ManifestDPIAwareness "PerMonitorV2,System"
SetCompressor /SOLID LZMA
SetCompressorDictSize 64
RequestExecutionLevel admin

!include "MUI2.nsh"
!include "x64.nsh"
!include "FileFunc.nsh"
!include "WordFunc.nsh"
!include "LogicLib.nsh"
!include "nsDialogs.nsh"

!define APP "tTorrent"
!define UNINST_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\tTorrent"
!define ICON "..\..\src\icons\qbittorrent.ico"

Name "${APP} ${TT_VERSION}"
OutFile "${TT_OUT}"
; An upgrade goes where the last install went (see .onInit). InstallDirRegKey
; can't do that: it reads the 32-bit view of the registry.
InstallDir "$PROGRAMFILES64\tTorrent"
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
Page custom AssocPage AssocPageLeave
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

; The "Open with tTorrent" page: both choices start ticked. /ASSOC=0 on the
; command line unticks both (silent test installs, see VERIFY.md › Releases).
Var AssocMagnet
Var AssocTorrent
Var CheckMagnet
Var CheckTorrent
Var OldDir ; a per-user copy from 0.6.4 or earlier, empty if none
Var UserDir ; where 0.6.4 and earlier installed, for this user
Var NoAdmin ; 1 with /ADMIN=0

Function .onInit
  ${IfNot} ${RunningX64}
    MessageBox MB_OK|MB_ICONSTOP "tTorrent needs 64-bit Windows."
    Abort
  ${EndIf}
  ; A silent install shows no pages, so CheckRunning never runs: refuse here
  ; instead of replacing (and moving) files a running tTorrent holds open.
  ${If} ${Silent}
    nsExec::ExecToStack 'cmd /c tasklist /FI "IMAGENAME eq ttorrent.exe" /NH | find /I "ttorrent.exe"'
    Pop $0
    Pop $1
    ${If} $0 == "0"
      SetErrorLevel 3
      Abort
    ${EndIf}
  ${EndIf}
  ; Read before SetShellVarContext all, which turns $LOCALAPPDATA into
  ; ProgramData.
  SetShellVarContext current
  StrCpy $UserDir "$LOCALAPPDATA\Programs\tTorrent"
  SetRegView 64
  SetShellVarContext all
  ; Upgrade in place, unless /D= chose another folder.
  ${If} $INSTDIR == "$PROGRAMFILES64\tTorrent"
    ReadRegStr $0 HKLM "${UNINST_KEY}" "InstallLocation"
    ${If} $0 != ""
      StrCpy $INSTDIR $0
    ${EndIf}
  ${EndIf}
  ; The old per-user copy: from the old installer, or from install-local.ps1,
  ; which used the same folder and wrote no uninstaller.
  ReadRegStr $OldDir HKCU "${UNINST_KEY}" "InstallLocation"
  ${If} $OldDir == ""
  ${AndIf} ${FileExists} "$UserDir\ttorrent.exe"
    StrCpy $OldDir $UserDir
  ${EndIf}
  ; /MOVE=0 leaves that copy alone (test installs, see VERIFY.md › Releases).
  ${GetParameters} $0
  ClearErrors
  ${GetOptions} $0 "/MOVE=" $1
  ${IfNot} ${Errors}
  ${AndIf} $1 == "0"
    StrCpy $OldDir ""
  ${EndIf}
  ; /ADMIN=0 skips the firewall rule and long file paths (test installs).
  StrCpy $NoAdmin 0
  ClearErrors
  ${GetOptions} $0 "/ADMIN=" $1
  ${IfNot} ${Errors}
  ${AndIf} $1 == "0"
    StrCpy $NoAdmin 1
  ${EndIf}
  StrCpy $AssocMagnet ${BST_CHECKED}
  StrCpy $AssocTorrent ${BST_CHECKED}
  ${GetParameters} $0
  ClearErrors
  ${GetOptions} $0 "/ASSOC=" $1
  ${IfNot} ${Errors}
  ${AndIf} $1 == "0"
    StrCpy $AssocMagnet ${BST_UNCHECKED}
    StrCpy $AssocTorrent ${BST_UNCHECKED}
  ${EndIf}
FunctionEnd

Function AssocPage
  !insertmacro MUI_HEADER_TEXT "Open with tTorrent" "Choose what tTorrent opens for you."
  nsDialogs::Create 1018
  Pop $0
  ${NSD_CreateCheckbox} 0 0 100% 12u "Open &magnet links with tTorrent"
  Pop $CheckMagnet
  ${NSD_SetState} $CheckMagnet $AssocMagnet
  ${NSD_CreateCheckbox} 0 18u 100% 12u "Open &.torrent files with tTorrent"
  Pop $CheckTorrent
  ${NSD_SetState} $CheckTorrent $AssocTorrent
  ${NSD_CreateLabel} 0 42u 100% 40u "If you already chose another app for these in Windows Settings, Windows asks you to confirm the change. Default apps then opens after installing, on tTorrent's page."
  Pop $0
  nsDialogs::Show
FunctionEnd

Function AssocPageLeave
  ${NSD_GetState} $CheckMagnet $AssocMagnet
  ${NSD_GetState} $CheckTorrent $AssocTorrent
FunctionEnd

; Moves a per-user copy from 0.6.4 or earlier out of the way. Its own
; uninstaller removes exactly its files, shortcuts and keys, but also the
; person's choices, so those are read first and written again for the new
; path: starting with Windows here, magnet and .torrent handling in SecAssoc
; (even with /ASSOC=0, which means "leave my handlers as they are").
; CheckRunning has already made sure no tTorrent is running.
Section "-Move the per-user copy" SecMove
  StrCmp $OldDir "" move_done
  StrCmp $OldDir $INSTDIR move_done
  DetailPrint "Moving tTorrent from $OldDir"
  StrCpy $R8 0 ; 1 when it started with Windows
  ClearErrors
  ReadRegStr $0 HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "tTorrent"
  ${WordFind} "$0" "$OldDir\ttorrent.exe" "E#" $1
  ${IfNot} ${Errors}
    StrCpy $R8 1
  ${EndIf}
  ClearErrors
  ReadRegStr $0 HKCU "Software\Classes\tTorrent.torrent\shell\open\command" ""
  ${WordFind} "$0" "$OldDir\ttorrent.exe" "E#" $1
  ${IfNot} ${Errors}
    StrCpy $AssocTorrent ${BST_CHECKED}
  ${EndIf}
  ClearErrors
  ReadRegStr $0 HKCU "Software\Classes\magnet\shell\open\command" ""
  ${WordFind} "$0" "$OldDir\ttorrent.exe" "E#" $1
  ${IfNot} ${Errors}
    StrCpy $AssocMagnet ${BST_CHECKED}
  ${EndIf}

  ${If} ${FileExists} "$OldDir\uninstall.exe"
    ; With _? it runs in place and waits, and leaves itself behind.
    ExecWait '"$OldDir\uninstall.exe" /S _?=$OldDir'
    Delete "$OldDir\uninstall.exe"
  ${EndIf}
  ; install-local.ps1 kept no file list, but its folder held only tTorrent.
  ${If} $OldDir == $UserDir
    RMDir /r "$OldDir"
  ${Else}
    RMDir "$OldDir"
  ${EndIf}
  ; The old shortcuts were per user; the new ones are for all users.
  SetShellVarContext current
  Delete "$SMPROGRAMS\tTorrent.lnk"
  Delete "$DESKTOP\tTorrent.lnk"
  SetShellVarContext all
  DeleteRegKey HKCU "${UNINST_KEY}"

  ${If} $R8 == 1
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "tTorrent" '"$INSTDIR\ttorrent.exe"'
  ${EndIf}
  move_done:
SectionEnd

Section "tTorrent (required)" SecApp
  SectionIn RO
  SetOutPath "$INSTDIR"
  ; Leftovers from install-local.ps1, which renames in-use files aside.
  Delete "$INSTDIR\ttorrent-old*.exe"
  !include "${TT_FILES}"
  WriteUninstaller "$INSTDIR\uninstall.exe"

  WriteRegStr HKLM "${UNINST_KEY}" "DisplayName" "${APP}"
  WriteRegStr HKLM "${UNINST_KEY}" "DisplayVersion" "${TT_VERSION}"
  WriteRegStr HKLM "${UNINST_KEY}" "DisplayIcon" '"$INSTDIR\ttorrent.exe",0'
  WriteRegStr HKLM "${UNINST_KEY}" "Publisher" "t21 dev"
  WriteRegStr HKLM "${UNINST_KEY}" "URLInfoAbout" "https://ttorrent.t21.dev"
  WriteRegStr HKLM "${UNINST_KEY}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKLM "${UNINST_KEY}" "UninstallString" '"$INSTDIR\uninstall.exe"'
  WriteRegStr HKLM "${UNINST_KEY}" "QuietUninstallString" '"$INSTDIR\uninstall.exe" /S'
  WriteRegDWORD HKLM "${UNINST_KEY}" "NoModify" 1
  WriteRegDWORD HKLM "${UNINST_KEY}" "NoRepair" 1
  ${GetSize} "$INSTDIR" "/S=0K" $0 $1 $2
  IntFmt $0 "0x%08X" $0
  WriteRegDWORD HKLM "${UNINST_KEY}" "EstimatedSize" "$0"
SectionEnd

Section "Desktop shortcut" SecDesktop
  CreateShortCut "$DESKTOP\tTorrent.lnk" "$INSTDIR\ttorrent.exe"
SectionEnd

Section "Start menu shortcut" SecStartMenu
  CreateShortCut "$SMPROGRAMS\tTorrent.lnk" "$INSTDIR\ttorrent.exe"
SectionEnd

; The same Run value the app writes for its default profile (Preferences::
; setWinStartup), so the setting in Options shows it and can turn it off.
Section /o "Start tTorrent when Windows starts" SecStartup
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "tTorrent" '"$INSTDIR\ttorrent.exe"'
SectionEnd

; These two only record the choice; the hidden section below does the work.
Section "Add a Windows Firewall rule" SecFirewall
SectionEnd

Section "Allow long file paths" SecLongPaths
SectionEnd

; The installer already runs as admin, so neither needs a prompt of its own.
Section "-Admin settings" SecAdmin
  StrCmp $NoAdmin 1 admin_done
  ${If} ${SectionIsSelected} ${SecFirewall}
    nsExec::ExecToStack 'netsh advfirewall firewall delete rule name=tTorrent'
    Pop $0
    Pop $1
    nsExec::ExecToStack 'netsh advfirewall firewall add rule name=tTorrent dir=in action=allow program="$INSTDIR\ttorrent.exe" enable=yes profile=any'
    Pop $0
    Pop $1
    ${If} $0 != "0"
      DetailPrint "Couldn't add the Windows Firewall rule: $1"
    ${EndIf}
  ${EndIf}
  ${If} ${SectionIsSelected} ${SecLongPaths}
    WriteRegDWORD HKLM "SYSTEM\CurrentControlSet\Control\FileSystem" "LongPathsEnabled" 1
  ${EndIf}
  admin_done:
SectionEnd

; Points a value that names the old per-user exe at the new one. Sets $R7 to
; 1 when it changed something.
!macro REPOINT SUBKEY VALUE
  ClearErrors
  ReadRegStr $0 HKCU "${SUBKEY}" "${VALUE}"
  ${If} $0 != ""
    ClearErrors
    ${WordReplace} "$0" "$UserDir\ttorrent.exe" "$INSTDIR\ttorrent.exe" "E+" $1
    ${IfNot} ${Errors}
      WriteRegStr HKCU "${SUBKEY}" "${VALUE}" "$1"
      StrCpy $R7 1
    ${EndIf}
  ${EndIf}
!macroend

; Fixes .torrent and magnet handling that still names the old per-user exe
; after it is gone: what 0.6.5 and 0.6.6 left behind when they moved it, and
; the keys Windows writes itself when the person picks ttorrent.exe in "Open
; with" (torrent_auto_file, Applications\ttorrent.exe). Default apps' choice
; (UserChoice) often names one of those, so double-clicking a .torrent file
; said the app could not be found. Runs on every install, not only on a move,
; so an upgrade from a broken install is fixed too. What it finds counts as
; the person's choice, and SecAssoc registers tTorrent again for it.
Section "-Repoint the old path" SecRepoint
  ${If} ${FileExists} "$UserDir\ttorrent.exe" ; /MOVE=0 kept the old copy
  ${OrIf} $UserDir == $INSTDIR
    Goto repoint_done
  ${EndIf}
  StrCpy $R7 0
  !insertmacro REPOINT "Software\Classes\tTorrent.torrent\shell\open\command" ""
  !insertmacro REPOINT "Software\Classes\tTorrent.torrent\DefaultIcon" ""
  !insertmacro REPOINT "Software\Classes\torrent_auto_file\shell\open\command" ""
  !insertmacro REPOINT "Software\Classes\torrent_auto_file\DefaultIcon" ""
  !insertmacro REPOINT "Software\Classes\Applications\ttorrent.exe\shell\open\command" ""
  !insertmacro REPOINT "Software\Classes\Applications\ttorrent.exe\DefaultIcon" ""
  ${If} $R7 == 1
    StrCpy $AssocTorrent ${BST_CHECKED}
  ${EndIf}
  StrCpy $R7 0
  !insertmacro REPOINT "Software\Classes\tTorrent.magnet\shell\open\command" ""
  !insertmacro REPOINT "Software\Classes\tTorrent.magnet\DefaultIcon" ""
  !insertmacro REPOINT "Software\Classes\magnet\shell\open\command" ""
  !insertmacro REPOINT "Software\Classes\magnet\DefaultIcon" ""
  !insertmacro REPOINT "Software\Classes\magnet_auto_file\shell\open\command" ""
  !insertmacro REPOINT "Software\Classes\magnet_auto_file\DefaultIcon" ""
  ${If} $R7 == 1
    StrCpy $AssocMagnet ${BST_CHECKED}
  ${EndIf}
  !insertmacro REPOINT "Software\tTorrent\Capabilities" "ApplicationIcon"
  !insertmacro REPOINT "Software\Microsoft\Windows\CurrentVersion\Run" "tTorrent"
  repoint_done:
SectionEnd

; Makes tTorrent the handler for what the person ticked, for this user only
; (HKCU; qBittorrent's keys under HKLM are never touched). Windows keeps a
; default the person chose in Settings (UserChoice) and won't let an
; installer change it, so then Default apps opens on tTorrent's page.
Section "-Open with tTorrent" SecAssoc
  StrCpy $R9 0 ; 1 when Windows has to confirm a change in Default apps
  ${If} $AssocTorrent == ${BST_CHECKED}
    WriteRegStr HKCU "Software\Classes\tTorrent.torrent" "" "Torrent file"
    WriteRegStr HKCU "Software\Classes\tTorrent.torrent\DefaultIcon" "" '"$INSTDIR\ttorrent.exe",1'
    WriteRegStr HKCU "Software\Classes\tTorrent.torrent\shell\open\command" "" '"$INSTDIR\ttorrent.exe" "%1"'
    WriteRegStr HKCU "Software\Classes\.torrent\OpenWithProgids" "tTorrent.torrent" ""
    WriteRegStr HKCU "Software\Classes\.torrent" "" "tTorrent.torrent"
    WriteRegStr HKCU "Software\tTorrent\Capabilities\FileAssociations" ".torrent" "tTorrent.torrent"
    ReadRegStr $0 HKCU "Software\Microsoft\Windows\CurrentVersion\Explorer\FileExts\.torrent\UserChoice" "ProgId"
    ${If} $0 != ""
    ${AndIf} $0 != "tTorrent.torrent"
      StrCpy $R9 1
    ${EndIf}
  ${EndIf}
  ${If} $AssocMagnet == ${BST_CHECKED}
    WriteRegStr HKCU "Software\Classes\tTorrent.magnet" "" "Magnet link"
    WriteRegStr HKCU "Software\Classes\tTorrent.magnet" "URL Protocol" ""
    WriteRegStr HKCU "Software\Classes\tTorrent.magnet\DefaultIcon" "" '"$INSTDIR\ttorrent.exe",0'
    WriteRegStr HKCU "Software\Classes\tTorrent.magnet\shell\open\command" "" '"$INSTDIR\ttorrent.exe" "%1"'
    ; Browsers only open magnet: links when a "magnet" key exists.
    WriteRegStr HKCU "Software\Classes\magnet" "" "URL:Magnet link"
    WriteRegStr HKCU "Software\Classes\magnet" "URL Protocol" ""
    WriteRegStr HKCU "Software\Classes\magnet\DefaultIcon" "" '"$INSTDIR\ttorrent.exe",0'
    WriteRegStr HKCU "Software\Classes\magnet\shell\open\command" "" '"$INSTDIR\ttorrent.exe" "%1"'
    WriteRegStr HKCU "Software\tTorrent\Capabilities\URLAssociations" "magnet" "tTorrent.magnet"
    ; "magnet" itself now resolves to the key above, so it counts as ours.
    ReadRegStr $0 HKCU "Software\Microsoft\Windows\Shell\Associations\UrlAssociations\magnet\UserChoice" "ProgId"
    ${If} $0 != ""
    ${AndIf} $0 != "tTorrent.magnet"
    ${AndIf} $0 != "magnet"
      StrCpy $R9 1
    ${EndIf}
  ${EndIf}
  ${If} $AssocTorrent == ${BST_CHECKED}
  ${OrIf} $AssocMagnet == ${BST_CHECKED}
    WriteRegStr HKCU "Software\tTorrent\Capabilities" "ApplicationName" "tTorrent"
    WriteRegStr HKCU "Software\tTorrent\Capabilities" "ApplicationDescription" "A fast, modern torrent client"
    WriteRegStr HKCU "Software\tTorrent\Capabilities" "ApplicationIcon" '"$INSTDIR\ttorrent.exe",0'
    WriteRegStr HKCU "Software\RegisteredApplications" "tTorrent" "Software\tTorrent\Capabilities"
    System::Call 'shell32::SHChangeNotify(i 0x8000000, i 0, p 0, p 0)'
    ${If} $R9 == 1
    ${AndIfNot} ${Silent}
      ExecShell "open" "ms-settings:defaultapps?registeredAppUser=tTorrent"
    ${EndIf}
  ${EndIf}
SectionEnd

Function un.onInit
  SetRegView 64
  SetShellVarContext all
FunctionEnd

; Removes only what the installer put there. Torrents, settings and downloads
; (the profile in %APPDATA% and %LOCALAPPDATA%) stay. The per-user keys are
; the current user's: normally the person who installed tTorrent removes it.
Section "Uninstall"
  !include "${TT_UNFILES}"
  Delete "$INSTDIR\ttorrent-old*.exe"
  Delete "$INSTDIR\uninstall.exe"
  RMDir "$INSTDIR"

  Delete "$SMPROGRAMS\tTorrent.lnk"
  Delete "$DESKTOP\tTorrent.lnk"

  ; Start-up entry: the default profile's value, and only if it points here.
  ClearErrors
  ReadRegStr $0 HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "tTorrent"
  ${un.WordFind} "$0" "ttorrent.exe" "E#" $1
  IfErrors +2
    DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "tTorrent"

  ; The firewall rule only if it is for this copy's exe. Long file paths
  ; stay on: other apps may rely on that Windows setting.
  nsExec::ExecToStack 'netsh advfirewall firewall show rule name=tTorrent verbose'
  Pop $0
  Pop $1
  ClearErrors
  ${un.WordFind} "$1" "$INSTDIR\ttorrent.exe" "E#" $2
  ${IfNot} ${Errors}
    nsExec::ExecToStack 'netsh advfirewall firewall delete rule name=tTorrent'
    Pop $0
    Pop $1
  ${EndIf}

  DeleteRegKey HKCU "Software\Classes\tTorrent.torrent"
  DeleteRegKey HKCU "Software\Classes\tTorrent.magnet"
  ; The magnet scheme key only if it still points at tTorrent.
  ClearErrors
  ReadRegStr $0 HKCU "Software\Classes\magnet\shell\open\command" ""
  ${un.WordFind} "$0" "ttorrent.exe" "E#" $1
  IfErrors +2
    DeleteRegKey HKCU "Software\Classes\magnet"
  DeleteRegValue HKCU "Software\Classes\.torrent\OpenWithProgids" "tTorrent.torrent"
  ReadRegStr $0 HKCU "Software\Classes\.torrent" ""
  StrCmp $0 "tTorrent.torrent" 0 +2
    DeleteRegValue HKCU "Software\Classes\.torrent" ""
  DeleteRegKey HKCU "Software\tTorrent\Capabilities"
  DeleteRegKey /ifempty HKCU "Software\tTorrent"
  DeleteRegValue HKCU "Software\RegisteredApplications" "tTorrent"
  ; The uninstall entry only if it is this copy's (a test install elsewhere
  ; must not remove the real one's).
  ReadRegStr $0 HKLM "${UNINST_KEY}" "InstallLocation"
  StrCmp $0 $INSTDIR 0 +2
    DeleteRegKey HKLM "${UNINST_KEY}"
  System::Call 'shell32::SHChangeNotify(i 0x8000000, i 0, p 0, p 0)'
SectionEnd
