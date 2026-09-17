# Keyboard shortcuts

Every shortcut Mushroom has. Anything listed here works unless it says
otherwise; anything that works and is missing from this page is a bug.

This page and the `F1` dialog are both generated from one list in
`src/services/shortcuts.ts`, and a test fails if they disagree.

| Shortcut | Action |
|---|---|
| `Ctrl+N` | New note |
| `Ctrl+S` | Save |
| `Ctrl+P` | Quick Open |
| `Ctrl+F` | Search notes |
| `Ctrl+Shift+F` | Toggle the AI panel |
| `Ctrl+Shift+P` | Toggle preview |
| `F1` | Keyboard shortcuts |
| `F5` | Refresh notes |
| `Escape` | Close menu or dialog |
| `Alt` | Focus the menu bar |
| `Ctrl+O` | Open a file — *not available yet* |

## Menus

`Alt` focuses the menu bar. `Alt` plus the underlined letter opens that menu
directly — `Alt+F` for File, `Alt+T` for Tools. Inside an open menu, the arrow
keys move, `Enter` chooses, and `Escape` closes.

## Quick Open

`Ctrl+P` opens a note by name from anywhere, whichever folder is selected.
Type part of the name; the letters do not have to be adjacent, so `gpuinf`
finds **GPU Infrastructure**. Arrow keys move, `Enter` opens, `Escape` cancels.

With an empty box it lists the notes you opened most recently, so `Ctrl+P`
then `Enter` returns to the previous note.

## Dialogs

Every dialog takes `Enter` to accept and `Escape` to cancel, and returns focus
to whatever had it before the dialog opened. `Tab` moves in the order things
appear on screen.
