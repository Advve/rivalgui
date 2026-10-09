; RivalGUI installer hooks (bundle.windows.nsis.installerHooks)

!macro NSIS_HOOK_POSTINSTALL
  ; Builds before 1.0.0 final had no publisher set, so Tauri used "github"
  ; (from the identifier) and stored the install location under that key.
  ; The publisher is now "Adve": drop the stale key, and its parent only if empty.
  DeleteRegKey HKCU "Software\github\${PRODUCTNAME}"
  DeleteRegKey /ifempty HKCU "Software\github"
!macroend
