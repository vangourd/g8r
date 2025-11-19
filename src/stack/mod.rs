pub mod source;
pub mod git;
pub mod manager;

pub use source::StackSource;
pub use git::GitSource;
pub use manager::StackManager;
pub use crate::db::{Stack, NewStack};
