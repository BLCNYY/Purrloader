Unicode true
!include "MUI2.nsh"
!include "x64.nsh"
Name "Purrloader"
OutFile "${OUTPUT}"
InstallDir "$LOCALAPPDATA\Programs\Purrloader"
RequestExecutionLevel user
SetCompressor /SOLID lzma
VIProductVersion "${NUMVERSION}"
VIAddVersionKey "ProductName" "Purrloader"
VIAddVersionKey "FileDescription" "Purrloader installer"
VIAddVersionKey "FileVersion" "${VERSION}"
VIAddVersionKey "LegalCopyright" "Copyright 2026 Ömer Balkan"
!define MUI_ICON "${ICON}"
!define MUI_UNICON "${ICON}"
!define MUI_FINISHPAGE_RUN "$INSTDIR\purrloader.exe"
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"

Section "Install"
  SetShellVarContext current
  SetOutPath "$INSTDIR"
  File "${BINARY}"
  File "${LICENSE}"
  File "${NOTICES}"
  File /oname=THIRD_PARTY_LICENSES.html "${DEPENDENCY_LICENSES}"
  WriteUninstaller "$INSTDIR\Uninstall.exe"
  CreateShortcut "$SMPROGRAMS\Purrloader.lnk" "$INSTDIR\purrloader.exe"
  CreateShortcut "$DESKTOP\Purrloader.lnk" "$INSTDIR\purrloader.exe"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Purrloader" "DisplayName" "Purrloader"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Purrloader" "DisplayVersion" "${VERSION}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Purrloader" "Publisher" "BLCNYY"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Purrloader" "DisplayIcon" "$INSTDIR\purrloader.exe"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Purrloader" "UninstallString" '$\"$INSTDIR\Uninstall.exe$\"'
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Purrloader" "NoModify" 1
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Purrloader" "NoRepair" 1
SectionEnd

Section "Uninstall"
  SetShellVarContext current
  Delete "$DESKTOP\Purrloader.lnk"
  Delete "$SMPROGRAMS\Purrloader.lnk"
  Delete "$INSTDIR\purrloader.exe"
  Delete "$INSTDIR\LICENSE"
  Delete "$INSTDIR\THIRD_PARTY_NOTICES.md"
  Delete "$INSTDIR\THIRD_PARTY_LICENSES.html"
  Delete "$INSTDIR\Uninstall.exe"
  RMDir "$INSTDIR"
  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Purrloader"
  ; Preserve downloads and %APPDATA%\Purrloader, including the resumable queue.
SectionEnd
