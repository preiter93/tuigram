pub mod confirm;
mod editor;
pub mod help;
pub mod input;
pub mod scroll;
mod selection;
pub mod status_bar;

pub use editor::{EditorMode, EditorState};
pub use selection::Selection;

pub const HEADER_HEIGHT: u16 = 4;
pub const FIRST_MESSAGE_OFFSET: u16 = 2;
