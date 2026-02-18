use ratatui::widgets::ScrollbarState;

/// Height of the participant header area
pub const HEADER_HEIGHT: u16 = 3;
/// Vertical spacing between messages
pub const MESSAGE_SPACING: u16 = 3;
/// Offset from lifeline start to first message
pub const FIRST_MESSAGE_OFFSET: u16 = 2;

pub struct ScrollState {
    /// Index of the first visible event
    pub offset: usize,
    /// Height available for rendering events
    viewport_height: u16,
}

impl ScrollState {
    pub fn new() -> Self {
        Self {
            offset: 0,
            viewport_height: 0,
        }
    }

    /// Update the viewport height (call this on resize/render)
    pub fn set_viewport(&mut self, height: u16) {
        self.viewport_height = height.saturating_sub(HEADER_HEIGHT + FIRST_MESSAGE_OFFSET);
    }

    /// Ensure the given event index is visible, adjusting offset if needed
    pub fn ensure_visible(&mut self, index: usize) {
        let capacity = self.capacity();
        if capacity == 0 {
            return;
        }

        if index < self.offset {
            // Scrolling up
            self.offset = index;
        } else if index >= self.offset + capacity {
            // Scrolling down
            self.offset = index.saturating_sub(capacity - 1);
        }
    }

    /// Number of events that fit in the viewport
    fn capacity(&self) -> usize {
        (self.viewport_height / MESSAGE_SPACING) as usize
    }

    /// Returns the range of visible event indices
    pub fn visible_range(&self, total_events: usize) -> std::ops::Range<usize> {
        let end = (self.offset + self.capacity()).min(total_events);
        self.offset..end
    }

    /// Returns true if scrolling is needed (more events than viewport can show)
    pub fn needs_scroll(&self, total_events: usize) -> bool {
        total_events > self.capacity()
    }

    /// Returns a `ScrollbarState` for rendering the scrollbar
    pub fn scrollbar_state(&self, total_events: usize) -> ScrollbarState {
        // content_length is the max scroll position + 1 (the scrollable range)
        let max_offset = total_events.saturating_sub(self.capacity());
        ScrollbarState::new(max_offset + 1).position(self.offset)
    }
}

impl Default for ScrollState {
    fn default() -> Self {
        Self::new()
    }
}
