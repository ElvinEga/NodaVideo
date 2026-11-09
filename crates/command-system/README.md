# `nodavideo-command-system`

Canonical command system, transactional mutation execution, and unified undo/redo history for **NodaVideo**.

## Architectural Contract

- **One Source of Truth:** All mutations to project and timeline state — whether initiated by human user interaction or autonomous AI agents — must go through `EditorCommand`.
- **Deterministic & Reversible:** Every forward command calculates an exact inverse command.
- **Transactional Safety:** Commands validate prerequisites before mutation. If an operation fails, the sequence state is left completely unmodified, and command history is preserved.
- **Unified Undo/Redo:** Provides a unified `CommandHistory` stack supporting undo, redo, capacity trimming, and human-readable descriptions for UI menus.

## Key Types

- [`EditorCommand`](src/command.rs): Canonical command enumeration:
  - `AddClip`: Places a new clip on a track.
  - `MoveClip`: Repositions a clip across frames or compatible tracks.
  - `TrimClip`: Adjusts in-point or out-point boundaries.
  - `SplitClip`: Splits a clip at a timeline frame.
  - `UnsplitClip`: Re-merges two previously split clips during undo.
  - `DeleteClip`: Removes a clip from a track.
  - `AddTrack` / `DeleteTrack`: Modifies sequence track collections.
  - `ToggleMute` / `ToggleSolo` / `ToggleLock`: Modifies track state.
- [`CommandHistory`](src/history.rs): Stack holding undo entries, redo entries, and state dispatchers.
- [`CommandError`](src/error.rs): Structured error representations for command execution and stack bounds.
