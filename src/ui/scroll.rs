use ratatui::widgets::ScrollbarState;
use std::ops::Range;

use crate::core::SequenceDiagram;
use crate::ui::HEADER_HEIGHT;

pub struct ScrollState {
    pub offset: usize,
    pub x_offset: u16,
    viewport_height: u16,
}

impl ScrollState {
    pub fn new() -> Self {
        Self {
            offset: 0,
            x_offset: 0,
            viewport_height: 0,
        }
    }

    pub fn set_viewport(&mut self, height: u16) {
        // One more row than the header: events start one row below the lifeline start.
        self.viewport_height = height.saturating_sub(HEADER_HEIGHT + 1);
    }

    pub fn ensure_visible(&mut self, index: usize, diagram: &SequenceDiagram) {
        let visible = self.visible_range(diagram);

        if index < visible.start {
            self.offset = index;
        } else if index >= visible.end {
            self.offset = self.find_offset_for_index(index, diagram);
        }
    }

    /// Scrolls sideways so the canvas columns `left..=right` are inside the border.
    /// A span wider than the view keeps its left edge visible.
    pub fn follow_x(&mut self, left: u16, right: u16, viewport: u16, max_offset: u16) {
        let last_visible = self.x_offset + viewport.saturating_sub(2);

        if right > last_visible {
            self.x_offset = (right + 2).saturating_sub(viewport);
        }
        if left < self.x_offset + 1 {
            self.x_offset = left.saturating_sub(1);
        }

        self.x_offset = self.x_offset.min(max_offset);
    }

    pub fn visible_range(&self, diagram: &SequenceDiagram) -> Range<usize> {
        let mut height_used: u16 = 0;
        let mut end = self.offset;

        for i in self.offset..diagram.event_count() {
            let height = diagram.event_height(i);
            if height_used.saturating_add(height) > self.viewport_height {
                break;
            }
            height_used += height;
            end += 1;
        }

        // An event taller than the viewport is still drawn, clipped.
        if end == self.offset && self.offset < diagram.event_count() {
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
            let event_height = diagram.event_height(i);
            if height + event_height > self.viewport_height {
                break;
            }
            height += event_height;
            offset = i;
        }

        offset
    }

    fn total_height(diagram: &SequenceDiagram) -> u16 {
        (0..diagram.event_count())
            .map(|i| diagram.event_height(i))
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn diagram(with_frame: bool) -> SequenceDiagram {
        let frame = if with_frame { "loop\n" } else { "" };
        let end = if with_frame { "end\n" } else { "" };
        let source = format!("sequenceDiagram\n{frame}A->>B: 1\n{end}A->>B: 2\nA->>B: 3\n");
        SequenceDiagram::from_mermaid(&source).unwrap()
    }

    #[test]
    fn viewport_counts_frame_header_and_footer_rows() {
        let mut scroll = ScrollState::new();
        scroll.set_viewport(HEADER_HEIGHT + 1 + 9);

        assert_eq!(scroll.visible_range(&diagram(false)), 0..3);
        assert_eq!(scroll.visible_range(&diagram(true)), 0..2);
    }

    #[test]
    fn viewport_counts_divider_rows() {
        let source = |keyword: &str, divider: &str| {
            format!("sequenceDiagram\n{keyword}\nA->>B: 1\n{divider}A->>B: 2\nend\nA->>B: 3\n")
        };
        let plain = SequenceDiagram::from_mermaid(&source("loop", "")).unwrap();
        let divided = SequenceDiagram::from_mermaid(&source("alt", "else\n")).unwrap();
        let mut scroll = ScrollState::new();
        scroll.set_viewport(HEADER_HEIGHT + 1 + 8);

        assert_eq!(scroll.visible_range(&plain), 0..2);
        assert_eq!(scroll.visible_range(&divided), 0..1);
    }

    #[test]
    fn tall_events_reduce_visible_range() {
        let source = "sequenceDiagram\nA->>B: a<br>b<br>c\nA->>B: 2\nA->>B: 3\n";
        let d = SequenceDiagram::from_mermaid(source).unwrap();
        let mut scroll = ScrollState::new();
        scroll.set_viewport(HEADER_HEIGHT + 1 + 9);

        assert_eq!(scroll.visible_range(&d), 0..2);

        scroll.ensure_visible(2, &d);
        assert_eq!(scroll.visible_range(&d).start, 1);
    }

    #[test]
    fn event_taller_than_viewport_is_still_visible() {
        let d =
            SequenceDiagram::from_mermaid("sequenceDiagram\nA->>B: a<br>b<br>c<br>d\n").unwrap();
        let mut scroll = ScrollState::new();
        scroll.set_viewport(3);

        assert_eq!(scroll.visible_range(&d), 0..1);
    }

    #[test]
    fn follow_x_keeps_visible_span_in_place() {
        let mut scroll = ScrollState::new();
        scroll.follow_x(10, 20, 40, 100);

        assert_eq!(scroll.x_offset, 0);
    }

    #[test]
    fn follow_x_scrolls_right_until_span_ends_inside_border() {
        let mut scroll = ScrollState::new();
        scroll.follow_x(50, 60, 40, 100);

        // visible columns are 23..=60
        assert_eq!(scroll.x_offset, 22);
    }

    #[test]
    fn follow_x_scrolls_left_until_span_starts_inside_border() {
        let mut scroll = ScrollState::new();
        scroll.x_offset = 30;
        scroll.follow_x(10, 15, 40, 100);

        assert_eq!(scroll.x_offset, 9);
    }

    #[test]
    fn follow_x_prefers_left_edge_of_wide_span() {
        let mut scroll = ScrollState::new();
        scroll.x_offset = 5;
        scroll.follow_x(20, 90, 40, 100);

        assert_eq!(scroll.x_offset, 19);
    }

    #[test]
    fn follow_x_clamps_to_max_offset() {
        let mut scroll = ScrollState::new();
        scroll.x_offset = 50;
        scroll.follow_x(60, 65, 40, 10);

        assert_eq!(scroll.x_offset, 10);
    }
}
