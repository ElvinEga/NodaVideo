//! Command history and unified undo/redo stack.

use nodavideo_timeline::Sequence;
use uuid::Uuid;

use crate::command::EditorCommand;
use crate::error::CommandError;

/// An executed command entry retained in the history stack.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryEntry {
    /// Unique entry identifier.
    pub id: Uuid,
    /// User-facing description of the action.
    pub description: String,
    /// Forward command executed.
    pub command: EditorCommand,
    /// Exact inverse command to undo the action.
    pub inverse: EditorCommand,
}

/// Unified undo/redo stack managing deterministic sequence state mutations.
#[derive(Debug, Clone)]
pub struct CommandHistory {
    /// Stack of executed actions eligible for undo.
    undo_stack: Vec<HistoryEntry>,
    /// Stack of undone actions eligible for redo.
    redo_stack: Vec<HistoryEntry>,
    /// Maximum number of undo steps retained.
    max_entries: usize,
}

impl Default for CommandHistory {
    fn default() -> Self {
        Self::new(100)
    }
}

impl CommandHistory {
    /// Create a new history manager with a maximum capacity.
    pub fn new(max_entries: usize) -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_entries: max_entries.max(1),
        }
    }

    /// Whether an action is available to undo.
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Whether an action is available to redo.
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Description of the next action to be undone.
    pub fn undo_description(&self) -> Option<&str> {
        self.undo_stack.last().map(|e| e.description.as_str())
    }

    /// Description of the next action to be redone.
    pub fn redo_description(&self) -> Option<&str> {
        self.redo_stack.last().map(|e| e.description.as_str())
    }

    /// Number of items on the undo stack.
    pub fn undo_count(&self) -> usize {
        self.undo_stack.len()
    }

    /// Number of items on the redo stack.
    pub fn redo_count(&self) -> usize {
        self.redo_stack.len()
    }

    /// Execute a command against the sequence, recording it in history if successful.
    /// If execution fails, state is guaranteed unchanged and history is not modified.
    pub fn execute(
        &mut self,
        command: EditorCommand,
        sequence: &mut Sequence,
    ) -> Result<(), CommandError> {
        // Apply forward command
        command.apply(sequence)?;

        let description = command.description();
        let inverse = command.inverse();

        self.undo_stack.push(HistoryEntry {
            id: Uuid::new_v4(),
            description,
            command,
            inverse,
        });

        // Any new action clears the redo stack
        self.redo_stack.clear();

        // Enforce maximum capacity
        if self.undo_stack.len() > self.max_entries {
            self.undo_stack.remove(0);
        }

        Ok(())
    }

    /// Undo the most recent action on the undo stack.
    pub fn undo(&mut self, sequence: &mut Sequence) -> Result<Option<String>, CommandError> {
        let entry = match self.undo_stack.pop() {
            Some(e) => e,
            None => return Err(CommandError::NoCommandToUndo),
        };

        // Apply inverse command
        if let Err(err) = entry.inverse.apply(sequence) {
            // Restore entry on error to maintain stack integrity
            self.undo_stack.push(entry);
            return Err(err);
        }

        let desc = entry.description.clone();
        self.redo_stack.push(entry);
        Ok(Some(desc))
    }

    /// Redo the most recently undone action on the redo stack.
    pub fn redo(&mut self, sequence: &mut Sequence) -> Result<Option<String>, CommandError> {
        let entry = match self.redo_stack.pop() {
            Some(e) => e,
            None => return Err(CommandError::NoCommandToRedo),
        };

        // Apply forward command
        if let Err(err) = entry.command.apply(sequence) {
            // Restore entry on error to maintain stack integrity
            self.redo_stack.push(entry);
            return Err(err);
        }

        let desc = entry.description.clone();
        self.undo_stack.push(entry);
        Ok(Some(desc))
    }

    /// Clear all undo and redo history.
    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }

    /// Commit an entire staged changeset as a single transactional command.
    pub fn commit_staged(
        &mut self,
        mut changeset: crate::staged::StagedChangeset,
        sequence: &mut Sequence,
    ) -> Result<crate::staged::StagedChangeset, CommandError> {
        if changeset.commands.is_empty() {
            changeset.status = crate::staged::ChangesetStatus::Committed;
            return Ok(changeset);
        }

        let batch = changeset.clone().into_batch_command();
        self.execute(batch, sequence)?;
        changeset.status = crate::staged::ChangesetStatus::Committed;
        Ok(changeset)
    }
}
