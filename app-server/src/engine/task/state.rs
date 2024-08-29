//! Task input and output
//!
//! [`Output`] and [`Input`] represent the output and input of the task respectively.
//!
//! # [`Output`]
//!
//! Users should consider the output results of the task when defining the specific
//! behavior of the task. The input results may be: normal output, no output, or task
//! execution error message.
//! It should be noted that the content stored in [`Output`] must implement the [`Clone`] trait.
//!
//! # Example
//! In general, a task may produce output or no output:
//! ```rust
//! use crate::engine::Output;
//! let out=Output::new(10);
//! let non_out=Output::empty();
//! ```
//! In some special cases, when a predictable error occurs in the execution of a task's
//! specific behavior, the user can choose to return the error message as the output of
//! the task. Of course, this will cause subsequent tasks to abandon execution.
//!
//! ```rust
//! use crate::engine::Output;
//! use crate::engine::task::Content;
//! let err_out = Output::Err("some error messages!".to_string());
//! ```
//!
//! # [`Input`]
//!
//! [`Input`] represents the input required by the task. The input comes from the output
//! generated by multiple predecessor tasks of the task. If a predecessor task does not produce
//! output, the output will not be stored in [`Input`].
//! [`Input`] will be used directly by the user without user construction. [`Input`] is actually
//! constructed by cloning multiple [`Output`]. Users can obtain the content stored in [`Input`]
//! to implement the logic of the program.

use crate::pipeline::nodes::Message;
use core::panic;
use std::{
    fmt::Debug,
    sync::{Arc, Mutex},
};
use tokio::sync::Semaphore;

/// [`ExeState`] internally stores [`Output`], which represents whether the execution of
/// the task is successful, and its internal semaphore is used to synchronously obtain
/// the output of the predecessor task as the input of this task.
#[derive(Debug)]
pub(crate) struct ExecState {
    /// Output produced by a task.
    output: Arc<Mutex<State>>,
    /// The semaphore is used to control the synchronous blocking of subsequent tasks to obtain the
    /// execution results of this task.
    /// When a task is successfully executed, the permits inside the semaphore will be increased to
    /// n (n represents the number of successor tasks of this task or can also be called the output
    /// of the node), which means that the output of the task is available, and then each successor
    /// The task will obtain a permits synchronously (the permit will not be returned), which means
    /// that the subsequent task has obtained the execution result of this task.
    semaphore: Semaphore,
    /// Exec state output is resettable if the corresponding input handle is cyclic. This is used to
    /// make sure the node in a cyclic flow does not take the input from the previous iteration.
    resettable: bool,
}

/// Output produced by a task.
#[derive(Debug, Clone)]
pub enum State {
    Success(Arc<Message>),
    Empty(Arc<Message>),
    Termination,
}

impl ExecState {
    /// Construct a new [`ExeState`].
    pub fn new_with_resettable(resettable: bool) -> Self {
        Self {
            output: Arc::new(Mutex::new(State::empty())),
            semaphore: Semaphore::new(0),
            resettable,
        }
    }

    /// After the task is successfully executed, set the execution result.
    pub fn set_state(&self, output: State) {
        *self.output.lock().unwrap() = output;
    }

    /// [`Output`] for fetching internal storage.
    /// This function is generally not called directly, but first uses the semaphore for synchronization control.
    pub fn get_state(&self) -> State {
        self.output.lock().unwrap().to_owned()
    }

    /// A utility function to set the successful output and propagate down the line
    /// by adding permits.
    pub fn set_state_and_permits(&self, state: State, n_permits_to_add: usize) {
        self.set_state(state);
        self.semaphore().add_permits(n_permits_to_add);
    }

    /// The semaphore is used to control the synchronous acquisition of task output results.
    /// Under normal circumstances, first use the semaphore to obtain a permit, and then call
    /// the `get_output` function to obtain the output. If the current task is not completed
    /// (no output is generated), the subsequent task will be blocked until the current task
    /// is completed and output is generated.
    pub fn semaphore(&self) -> &Semaphore {
        &self.semaphore
    }

    pub fn is_resettable(&self) -> bool {
        self.resettable
    }
}

impl State {
    pub fn new(val: Message) -> Self {
        Self::Success(Arc::new(val))
    }

    pub fn empty() -> Self {
        Self::Empty(Arc::new(Message::empty()))
    }

    pub fn termination() -> Self {
        Self::Termination
    }

    /// Determine whether [`Output`] stores success information.
    pub fn is_success(&self) -> bool {
        match self {
            Self::Success(_) => true,
            Self::Termination | Self::Empty(_) => false,
        }
    }

    pub fn is_termination(&self) -> bool {
        matches!(self, Self::Termination)
    }

    /// Get the contents of [`Output`].
    pub fn get_out(&self) -> Arc<Message> {
        match self {
            Self::Success(ref out) => out.clone(),
            Self::Empty(ref out) => out.clone(),
            Self::Termination => panic!("Task is terminated!"),
        }
    }
}
