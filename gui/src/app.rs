// todo(gui): Define App (shared state: settings, lists, warnings, current error), the screen enum, the top-level Message, update, and view
//  - The selected list lives in App, not in the List screen variant, so it survives navigation.
//  - Deleting the selected list clears the selection and the list screen shows an empty state.
//  - Deletion asks for confirmation first; the pending action is stored in App.

// todo(gui): Mark App dirty on data change; save on Ctrl+S, 5 seconds after the last change, and on window close
//  - Saving on Ctrl+S, 5 seconds after the last change, and on window close keeps
//    writes and presave backups low while limiting data loss on a crash.