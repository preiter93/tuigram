mod editor;
pub mod help;
pub mod input;
pub mod scroll;
mod selection;
pub mod status_bar;

pub use editor::{EditorMode, EditorState};
pub use selection::Selection;

/// Height of the participant header area
pub const HEADER_HEIGHT: u16 = 3;
/// Vertical spacing between messages
pub const MESSAGE_SPACING: u16 = 3;
/// Offset from lifeline start to first message
pub const FIRST_MESSAGE_OFFSET: u16 = 2;
