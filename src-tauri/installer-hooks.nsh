; NSIS hooks for the Mushroom installer.
;
; The only one we use is the uninstall message. R6.6 says the uninstaller must
; not remove the user's notes *and must say so* — the second half matters as
; much as the first. Somebody uninstalling an app they have kept a year of
; notes in deserves to be told, at the moment they are worrying about it, that
; the notes are still there and where to find them. Saying nothing is how you
; get someone reinstalling just to check.

!macro NSIS_HOOK_PREUNINSTALL
  ; Before anything is removed, while backing out is still free.
  MessageBox MB_OKCANCEL|MB_ICONINFORMATION \
    "Uninstalling Mushroom removes the program and its shortcuts.$\r$\n$\r$\n\
Your notes are NOT removed. They stay as Markdown files in:$\r$\n\
    $PROFILE\Mushroom\notes$\r$\n\
    (or wherever you moved your notes folder)$\r$\n$\r$\n\
Your settings and search index stay in:$\r$\n\
    $APPDATA\Mushroom$\r$\n$\r$\n\
Delete those folders yourself if you want them gone." \
    IDOK continue_uninstall
    Abort
  continue_uninstall:
!macroend
