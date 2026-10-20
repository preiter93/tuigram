use ratatui::{layout::Rect, widgets::ScrollbarState};
use std::ops::Range;

use crate::core::SequenceDiagram;
use crate::ui::{HEADER_HEIGHT, Selection};

pub struct ScrollState {
    pub offset: usize,
    pub x_offset: u16,
    /// Whether drawing scrolls to keep the selection visible. Mouse wheel scrolling turns it
    /// off until the next key press.
    pub follow: bool,
    /// Clickable areas of the last drawn frame in canvas columns and screen rows, in draw order.
    pub hit_areas: Vec<(Rect, Selection)>,
    /// Inner diagram area of the last drawn frame.
    pub diagram_area: Rect,
    viewport_height: u16,
}

impl ScrollState {
    pub fn new() -> Self {
        Self {
            offset: 0,
            x_offset: 0,
            follow: true,
            hit_areas: Vec::new(),
            diagram_area: Rect::ZERO,
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

    pub fn scroll_by(&mut self, delta: isize, diagram: &SequenceDiagram) {
        self.follow = false;

        let Some(last) = diagram.event_count().checked_sub(1) else {
            self.offset = 0;
            return;
        };
        let max_offset = self.find_offset_for_index(last, diagram);
        self.offset = self.offset.saturating_add_signed(delta).min(max_offset);
    }

    /// The upper bound is applied when drawing.
    pub fn scroll_x_by(&mut self, delta: i16) {
        self.follow = false;
        self.x_offset = self.x_offset.saturating_add_signed(delta);
    }

    /// Applies the sideways scroll.
    pub fn selection_at(&self, x: u16, y: u16) -> Selection {
        let x = x.saturating_add(self.x_offset);

        self.hit_areas
            .iter()
            .rev()
            .find(|(area, _)| area.contains((x, y).into()))
            .map_or(Selection::None, |(_, selection)| *selection)
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
    use rstest::rstest;

    fn parse(body: &str) -> SequenceDiagram {
        SequenceDiagram::from_mermaid(&format!("sequenceDiagram\n{body}\n")).unwrap()
    }

    #[rstest]
    #[case::success_plain_events_all_fit("A->>B: 1\nA->>B: 2\nA->>B: 3", HEADER_HEIGHT + 1 + 9, 0..3)]
    #[case::success_frame_rows_reduce_range("loop\nA->>B: 1\nend\nA->>B: 2\nA->>B: 3", HEADER_HEIGHT + 1 + 9, 0..2)]
    #[case::success_frame_footer_rows_reduce_range("loop\nA->>B: 1\nA->>B: 2\nend\nA->>B: 3", HEADER_HEIGHT + 1 + 8, 0..2)]
    #[case::success_divider_row_reduces_range("alt\nA->>B: 1\nelse\nA->>B: 2\nend\nA->>B: 3", HEADER_HEIGHT + 1 + 8, 0..1)]
    #[case::success_tall_event_reduces_range("A->>B: a<br>b<br>c\nA->>B: 2\nA->>B: 3", HEADER_HEIGHT + 1 + 9, 0..2)]
    // an event taller than the viewport is still drawn, clipped
    #[case::success_event_taller_than_viewport_still_drawn("A->>B: a<br>b<br>c<br>d", 3, 0..1)]
    fn visible_range_counts_event_frame_and_divider_rows(
        #[case] given_body: &str,
        #[case] given_viewport: u16,
        #[case] expected_range: std::ops::Range<usize>,
    ) {
        let mut scroll = ScrollState::new();
        scroll.set_viewport(given_viewport);

        assert_eq!(scroll.visible_range(&parse(given_body)), expected_range);
    }

    #[test]
    fn ensure_visible_accounts_for_tall_events() {
        let d = parse("A->>B: a<br>b<br>c\nA->>B: 2\nA->>B: 3");
        let mut scroll = ScrollState::new();
        scroll.set_viewport(HEADER_HEIGHT + 1 + 9);

        scroll.ensure_visible(2, &d);

        assert_eq!(scroll.visible_range(&d).start, 1);
    }

    // visible columns are `x_offset + 1 ..= x_offset + width - 2`
    #[rstest]
    #[case::success_keeps_span(0, 10, 20, 100, 0)]
    #[case::success_scrolls_right(0, 50, 60, 100, 22)]
    #[case::success_scrolls_left(30, 10, 15, 100, 9)]
    #[case::success_prefers_left_edge(5, 20, 90, 100, 19)]
    #[case::success_clamps_to_max(50, 60, 65, 10, 10)]
    fn follow_x_keeps_span_inside_border(
        #[case] given_x_offset: u16,
        #[case] given_start: u16,
        #[case] given_end: u16,
        #[case] given_max_offset: u16,
        #[case] expected_x_offset: u16,
    ) {
        let mut scroll = ScrollState::new();
        scroll.x_offset = given_x_offset;

        scroll.follow_x(given_start, given_end, 40, given_max_offset);

        assert_eq!(scroll.x_offset, expected_x_offset);
    }

    fn tall_diagram() -> SequenceDiagram {
        let messages = (0..20)
            .map(|i| format!("A->>B: {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let source = format!("sequenceDiagram\n{messages}\n");
        SequenceDiagram::from_mermaid(&source).unwrap()
    }

    #[test]
    fn scroll_by_clamps_and_stops_following() {
        let d = tall_diagram();
        let mut scroll = ScrollState::new();
        scroll.set_viewport(HEADER_HEIGHT + 1 + 9);

        scroll.scroll_by(-1, &d);
        assert_eq!(scroll.offset, 0);
        assert!(!scroll.follow);

        scroll.scroll_by(2, &d);
        assert_eq!(scroll.offset, 2);

        scroll.scroll_by(100, &d);
        assert_eq!(scroll.offset, 17);
    }

    #[test]
    fn selection_at_applies_x_offset_and_prefers_last_pushed() {
        let mut scroll = ScrollState::new();
        scroll
            .hit_areas
            .push((Rect::new(0, 0, 20, 3), Selection::Participant(0)));
        scroll
            .hit_areas
            .push((Rect::new(5, 1, 10, 1), Selection::Event(2)));

        assert_eq!(scroll.selection_at(6, 1), Selection::Event(2));
        assert_eq!(scroll.selection_at(6, 0), Selection::Participant(0));
        assert_eq!(scroll.selection_at(30, 0), Selection::None);

        scroll.x_offset = 10;
        assert_eq!(scroll.selection_at(0, 1), Selection::Event(2));
        assert_eq!(scroll.selection_at(6, 1), Selection::Participant(0));
        assert_eq!(scroll.selection_at(15, 0), Selection::None);
    }
}
