use core::{error, fmt};

/// Represents one thing that will be applied to an object `For`, to reach a desired state.
///
/// While the name `Operation` usually implies a single type of operation, you'll most likely want
/// to implement this on an enum of operations to apply over `For`.
pub trait Operation<For> {
	fn apply(&self, item: &mut For);
}

/// An undo-redo history implemented as a list of [`Action`]s.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct UndoRedo<Op> {
	actions: Vec<Action<Op>>,
	/// Where we are in `self.actions`, as an index that points to the "beginning" of an action's
	/// slot - before the list of undo & redo operations.
	///
	/// To help the explanation make more sense, here's what this position refers to during
	/// different operations:
	///
	/// * When creating an action, this index is overwritten, and all indices after it are cleared.
	/// * When redoing/applying an action, this index points to the action whose operations will be
	///   applied.
	/// * When undoing/reverting an action, this index points to the action *after* the one whose
	///   operations will be reverted.
	tapehead: usize,
}

impl<Op> UndoRedo<Op> {
	/// Resets the undo-redo history to its default state.
	pub fn clear_history(&mut self) {
		self.actions.clear();
		self.tapehead = 0;
	}

	/// Creates a new action at the current point in history, returning it so it can be filled with
	/// undo/redo operations.
	///
	/// If any unapplied actions exist, they are erased from the actions list.
	///
	/// # Panics
	/// Panics if the capacity of the list of actions exceeds `isize::MAX` bytes.
	pub fn create_action(&mut self) -> &mut Action<Op> {
		// If there is an action at (or past) the tapehead, delete everything past the tapehead.
		if self.actions.len() > self.tapehead {
			self.actions.truncate(self.tapehead);
		}

		// TODO: Switch to `Vec::push_mut` when it becomes stable
		self.actions.push(Action::default());
		self.actions
			.last_mut()
			.expect("action should have been pushed")
	}

	/// Applies the first unapplied action.
	///
	/// If no action exists to be applied, nothing happens.
	///
	/// # Errors
	/// Returns `UndoRedoError::NothingToDo` if there is nothing to apply (usually because you're
	/// at the end of undo-redo history.)
	///
	/// # Panics
	/// Panics if the current action index is at `usize::MAX` before this is called.
	pub fn redo<For>(&mut self, apply_to: &mut For) -> Result<(), UndoRedoError>
	where
		Op: Operation<For>,
	{
		match self.actions.get(self.tapehead) {
			Some(action) => {
				self.tapehead = self
					.tapehead
					.checked_add(1)
					.expect("tapehead should not be at usize::MAX");

				action.apply(apply_to);
				Ok(())
			}
			None => Err(UndoRedoError::NothingToDo),
		}
	}

	/// Reverts the last applied action.
	///
	/// # Errors
	/// Returns `UndoRedoError::NothingToDo` if there is nothing to revert (usually because you're
	/// at the beginning of undo-redo history.)
	pub fn undo<For>(&mut self, apply_to: &mut For) -> Result<(), UndoRedoError>
	where
		Op: Operation<For>,
	{
		match self.tapehead.checked_sub(1) {
			Some(new_index) => self.tapehead = new_index,
			None => return Err(UndoRedoError::NothingToDo),
		}

		if let Some(action) = self.actions.get(self.tapehead) {
			action.revert(apply_to);
			return Ok(());
		}

		Err(UndoRedoError::NothingToDo)
	}
}

// `Op` is only used inside of `Vec`s, so a "default" state would not generate any `Op`. As the
// `Default` derive macro assumes that we want a trait bound on `Op` no matter what, we have to
// manually implement `Default`.
impl<Op> Default for UndoRedo<Op> {
	fn default() -> Self {
		Self {
			actions: Default::default(),
			tapehead: Default::default(),
		}
	}
}

/// An error indicating an issue with performing an undo or redo.
#[derive(Debug)]
pub enum UndoRedoError {
	NothingToDo,
}

impl fmt::Display for UndoRedoError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::NothingToDo => write!(f, "nothing to perform"),
		}
	}
}

impl error::Error for UndoRedoError {}

/// Represents a series of [`Operation`]-implementing `Op`s that will be performed, to reach the
/// next or previous state.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Action<Op> {
	name: Option<String>,
	apply_ops: Vec<Op>,
	revert_ops: Vec<Op>,
}

impl<Op> Action<Op> {
	pub fn get_name(&self) -> Option<&str> {
		self.name.as_deref()
	}

	pub fn set_name(&mut self, new_name: impl ToString) -> &mut Self {
		self.name = Some(new_name.to_string());
		self
	}

	/// Adds an operation to perform when redoing/applying this action.
	///
	/// Operations are performed in the order they're added.
	pub fn add_redo_operation(&mut self, operation: Op) -> &mut Self {
		self.apply_ops.push(operation);
		self
	}

	/// Adds an operation to perform when undoing/reverting this action.
	///
	/// Operations are performed in the order they're added.
	pub fn add_undo_operation(&mut self, operation: Op) -> &mut Self {
		self.revert_ops.push(operation);
		self
	}

	pub fn apply<For>(&self, apply_to: &mut For)
	where
		Op: Operation<For>,
	{
		self.apply_ops.iter().for_each(|o| o.apply(apply_to));
	}

	pub fn revert<For>(&self, apply_to: &mut For)
	where
		Op: Operation<For>,
	{
		self.revert_ops.iter().for_each(|o| o.apply(apply_to));
	}
}

// `Op` is only used inside of `Vec`s, so a "default" state would not generate any `Op`. As the
// `Default` derive macro assumes that we want a trait bound on `Op` no matter what, we have to
// manually implement `Default`.
impl<Op> Default for Action<Op> {
	fn default() -> Self {
		Self {
			name: Default::default(),
			apply_ops: Default::default(),
			revert_ops: Default::default(),
		}
	}
}
