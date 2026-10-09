; Clipframes adds itself to the programs that start at login (the app does it, so the setting
; can be changed later). Uninstalling has to take that entry away again, together with the
; on/off choice Task Manager keeps beside it.
;
; An update runs the old version's uninstaller too (with /UPDATE). The entry stays then: the
; app only adds it when it is missing, so removing it here would switch a start at login the
; user turned off in Task Manager back on with every update.
!macro NSIS_HOOK_POSTUNINSTALL
  ${If} $UpdateMode <> 1
    DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "Clipframes"
    DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run" "Clipframes"
  ${EndIf}
!macroend
