; Clipframes adds itself to the programs that start at login (the app does it, so the setting
; can be changed later). Uninstalling has to take that entry away again.
!macro NSIS_HOOK_POSTUNINSTALL
  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "Clipframes"
!macroend
