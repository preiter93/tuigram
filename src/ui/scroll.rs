use ratatui::widgets::ScrollbarState;
use std::ops::Range;

use crate::core::{Event, SequenceDiagram};
use crate::ui::HEADER_HEIGHT;

pub struct ScrollState {
    pub offset: usize,
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

    pub fn ensure_visible(&mut self, index: usize, diagram: &SequenceDiagram) {
        let visible = self.visible_range(diagram);

        if index < visible.start {
            self.offset = index;
        } else if index >= visible.end {
            self.offset = self.find_offset_for_index(index, diagram);
        }
    }

    pub fn visible_range(&self, diagram: &SequenceDiagram) -> Range<usize> {
        let mut height_used: u16 = 0;
        let mut end = self.offset;

        for event in diagram.events.iter().skip(self.offset) {
            if height_used + event.height() > self.viewport_height {
                break;
            }
            height_used += event.height();
            end += 1;
        }

        self.offset..end
    }

    pub fn needs_scroll(&self, diagram: &SequenceDiagram) -> bool {
        Self::total_height(diagram) > self.viewport_height
    }

    pub fn scrollbar_state(&self, diagram: &SequenceDiagram) -> ScrollbarState {
        let visible_count = self.visible_range(diagram).len();
        let max_offset = diagram.event_count().saturating_sub(visible_count);
        ScrollbarState::new(max_offset + 1).position(self.offset)
    }

    fn find_offset_for_index(&self, index: usize, diagram: &SequenceDiagram) -> usize {
        let mut height: u16 = 0;
        let mut offset = index;

        for i in (0..=index).rev() {
            let event_height = diagram.events.get(i).map_or(3, Event::height);
            if height + event_height > self.viewport_height {
                break;
            }
            height += event_height;
            offset = i;
        }

        offset
    }

    fn total_height(diagram: &SequenceDiagram) -> u16 {
        diagram.events.iter().map(Event::height).sum()
    }
}
