; NSIS hooks for the Mushroom installer.
;
; The only one we use is the uninstall message. R6.6 says the uninstaller must
; not remove the user's notes *and must say so* — the second half matters as
; much as the first. Somebody uninstalling an app they have kept a year of
; notes in deserves to be told, at the moment they are worrying about it, that
; the notes are still there and where to find them. Saying nothing is how you
; get someone reinstalling just to check.

!macro NSIS_HOOK_PREUNINSTALL
  ; Only ever for an uninstall a person started and is watching.
  ;
  ; v1.0.0 showed this unconditionally, which was two bugs in one. `/S` is a
  ; silent uninstall, run by deployment tooling and by scripts, and a modal
  ; dialog in it does not annoy anyone — it hangs forever, invisibly, because
  ; nobody is there to click OK. And installing a newer version runs the old
  ; uninstaller with `/UPDATE`, so upgrading would have announced that your
  ; notes were safe from a removal that was not happening.
  ;
  ; $UpdateMode and $PassiveMode are Tauri's, set from the command line in
  ; un.onInit, which has already run by the time this macro is inserted.
  ; StrCmp rather than LogicLib so this does not depend on include order.
  IfSilent mushroom_skip_notice
  StrCmp $UpdateMode 1 mushroom_skip_notice
  StrCmp $PassiveMode 1 mushroom_skip_notice

  MessageBox MB_OKCANCEL|MB_ICONINFORMATION \
    "Uninstalling Mushroom removes the program and its shortcuts.$\r$\n$\r$\n\
Your notes are NOT removed. They stay as Markdown files in:$\r$\n\
    $PROFILE\Mushroom\notes$\r$\n\
    (or wherever you moved your notes folder)$\r$\n$\r$\n\
Your settings and search index stay in:$\r$\n\
    $APPDATA\Mushroom$\r$\n$\r$\n\
Delete those folders yourself if you want them gone." \
    IDOK mushroom_skip_notice
    Abort

  mushroom_skip_notice:
!macroend
