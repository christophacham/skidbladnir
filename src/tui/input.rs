/// Input mode for the TUI
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InputMode {
    /// Normal mode - navigating the board
    #[default]
    Normal,
    /// Entering task title
    InputTitle,
    /// Entering task description/prompt
    InputDescription,
}
