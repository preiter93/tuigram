use crate::ui::HEADER_HEIGHT;
use crate::ui::MESSAGE_SPACING;
use ratatui::widgets::ScrollbarState;

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

    pub fn set_viewport(&mut self, height: u16) {
        self.viewport_height = height.saturating_sub(HEADER_HEIGHT);
    }

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

    pub fn visible_range(&self, total_events: usize) -> std::ops::Range<usize> {
        let end = (self.offset + self.capacity()).min(total_events);
        self.offset..end
    }

    pub fn needs_scroll(&self, total_events: usize) -> bool {
        total_events > self.capacity()
    }

    pub fn scrollbar_state(&self, total_events: usize) -> ScrollbarState {
        let max_offset = total_events.saturating_sub(self.capacity());
        ScrollbarState::new(max_offset + 1).position(self.offset)
    }
}
