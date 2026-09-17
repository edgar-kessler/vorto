; Hooks for Tauri's NSIS installer (bundle.windows.nsis.installerHooks).

; Windows won't replace a running executable. The hooks run before the installer's own check
; for a running vorto.exe, so they close the app first: otherwise it would restart a voice
; engine that was just ended.
!macro NSIS_HOOK_PREINSTALL
  nsExec::Exec '"$SYSDIR\taskkill.exe" /F /IM vorto.exe'
  Pop $0
  nsExec::Exec '"$SYSDIR\taskkill.exe" /F /IM vorto-engine.exe /IM vorto-engine-gpu.exe'
  Pop $0
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  nsExec::Exec '"$SYSDIR\taskkill.exe" /F /IM vorto.exe'
  Pop $0
  nsExec::Exec '"$SYSDIR\taskkill.exe" /F /IM vorto-engine.exe /IM vorto-engine-gpu.exe'
  Pop $0
!macroend
