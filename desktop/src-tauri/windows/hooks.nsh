; Clipframes adds itself to the programs that start at login (the app does it, so the setting
; can be changed later). Uninstalling has to take that entry away again, together with the
; on/off choice Task Manager keeps beside it.
;
; The app's own updater installs over the old version without uninstalling. The uninstaller
; only runs with /UPDATE when a person reinstalls by hand and chooses to uninstall first.
; Both entries stay then: the app only adds its entry when it is missing, so removing it
; here would switch a start at login the user turned off in Task Manager back on. (The
; installer's own script removes the Run value on a real uninstall too; the second line is
; the one only this hook does.)
!macro NSIS_HOOK_POSTUNINSTALL
  ${If} $UpdateMode <> 1
    DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "Clipframes"
    DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run" "Clipframes"
  ${EndIf}
!macroend
