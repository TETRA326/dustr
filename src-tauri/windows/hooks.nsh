; Dustr installer hooks: adds/removes the classic Explorer context menu
; ("Show more options" on Windows 11). Per-user, no admin rights needed.

!macro DUSTR_MENU ROOT
  WriteRegStr HKCU "Software\Classes\${ROOT}\shell\Dustr" "MUIVerb" "Dustr"
  WriteRegStr HKCU "Software\Classes\${ROOT}\shell\Dustr" "Icon" "$INSTDIR\${MAINBINARYNAME}.exe"
  WriteRegStr HKCU "Software\Classes\${ROOT}\shell\Dustr" "SubCommands" ""
  WriteRegStr HKCU "Software\Classes\${ROOT}\shell\Dustr\shell\1dir" "MUIVerb" "Dust only this directory"
  WriteRegStr HKCU "Software\Classes\${ROOT}\shell\Dustr\shell\1dir\command" "" '"$INSTDIR\${MAINBINARYNAME}.exe" --dir "%V"'
  WriteRegStr HKCU "Software\Classes\${ROOT}\shell\Dustr\shell\2rec" "MUIVerb" "Dust all recursive folders"
  WriteRegStr HKCU "Software\Classes\${ROOT}\shell\Dustr\shell\2rec\command" "" '"$INSTDIR\${MAINBINARYNAME}.exe" --recursive "%V"'
!macroend

!macro NSIS_HOOK_POSTINSTALL
  !insertmacro DUSTR_MENU "Directory"
  !insertmacro DUSTR_MENU "Directory\Background"
  !insertmacro DUSTR_MENU "Drive"
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  DeleteRegKey HKCU "Software\Classes\Directory\shell\Dustr"
  DeleteRegKey HKCU "Software\Classes\Directory\Background\shell\Dustr"
  DeleteRegKey HKCU "Software\Classes\Drive\shell\Dustr"
!macroend
