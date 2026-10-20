use crate::core::SequenceDiagram;

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Selection {
    #[default]
    None,
    Participant(usize),
    Event(usize),
    Fragment(usize),
    /// Frame index and divider index within the frame.
    Section(usize, usize),
    Box(usize),
}

impl Selection {
    /// A box is a stop just before its first participant.
    pub fn left(self, diagram: &SequenceDiagram) -> Self {
        match self {
            Selection::None if diagram.participant_count() > 0 => Selection::Participant(0),
            Selection::Participant(idx) => match Self::box_starting_at(diagram, idx) {
                Some(box_idx) => Selection::Box(box_idx),
                None if idx > 0 => Selection::Participant(idx - 1),
                None => self,
            },
            Selection::Box(idx) => match diagram.boxes.get(idx) {
                Some(b) if b.start > 0 => Selection::Participant(b.start - 1),
                _ => self,
            },
            _ => self,
        }
    }

    pub fn right(self, diagram: &SequenceDiagram) -> Self {
        let participant_count = diagram.participant_count();

        match self {
            Selection::None if participant_count > 0 => Self::box_or_participant(diagram, 0),
            Selection::Participant(idx) if idx + 1 < participant_count => {
                Self::box_or_participant(diagram, idx + 1)
            }
            Selection::Box(idx) => diagram
                .boxes
                .get(idx)
                .map_or(self, |b| Selection::Participant(b.start)),
            _ => self,
        }
    }

    /// A frame header or divider is a stop just before the event below it.
    pub fn down(self, diagram: &SequenceDiagram) -> Self {
        let participant_count = diagram.participant_count();
        let event_count = diagram.event_count();

        match self {
            Selection::None if participant_count > 0 => Selection::Participant(0),
            Selection::None | Selection::Participant(_) | Selection::Box(_) if event_count > 0 => {
                Self::stop_at(diagram, 0)
            }
            Selection::Event(idx) if idx + 1 < event_count => Self::stop_at(diagram, idx + 1),
            Selection::Fragment(idx) => diagram
                .fragments
                .get(idx)
                .map_or(self, |f| Selection::Event(f.start)),
            Selection::Section(frame_idx, k) => {
                Self::section_start(diagram, frame_idx, k).map_or(self, Selection::Event)
            }
            _ => self,
        }
    }

    pub fn up(self, diagram: &SequenceDiagram) -> Self {
        let participant_count = diagram.participant_count();
        let first_row = if participant_count > 0 {
            Selection::Participant(0)
        } else {
            self
        };

        match self {
            Selection::None => first_row,
            Selection::Event(idx) => {
                if let Some(frame_idx) = Self::frame_starting_at(diagram, idx) {
                    Selection::Fragment(frame_idx)
                } else if let Some((frame_idx, k)) = diagram.section_at(idx) {
                    Selection::Section(frame_idx, k)
                } else if idx > 0 {
                    Selection::Event(idx - 1)
                } else {
                    first_row
                }
            }
            Selection::Fragment(idx) => match diagram.fragments.get(idx) {
                Some(f) if f.start > 0 => Selection::Event(f.start - 1),
                Some(_) => first_row,
                None => self,
            },
            Selection::Section(frame_idx, k) => Self::section_start(diagram, frame_idx, k)
                .map_or(self, |at| Selection::Event(at - 1)),
            Selection::Participant(_) | Selection::Box(_) => self,
        }
    }

    pub fn first_event(self, diagram: &SequenceDiagram) -> Self {
        if diagram.event_count() == 0 {
            return self;
        }
        Self::stop_at(diagram, 0)
    }

    pub fn last_event(self, diagram: &SequenceDiagram) -> Self {
        match diagram.event_count() {
            0 => self,
            count => Selection::Event(count - 1),
        }
    }

    fn box_or_participant(diagram: &SequenceDiagram, participant_idx: usize) -> Self {
        match Self::box_starting_at(diagram, participant_idx) {
            Some(box_idx) => Selection::Box(box_idx),
            None => Selection::Participant(participant_idx),
        }
    }

    fn box_starting_at(diagram: &SequenceDiagram, participant_idx: usize) -> Option<usize> {
        diagram
            .boxes
            .iter()
            .position(|b| b.start == participant_idx)
    }

    fn stop_at(diagram: &SequenceDiagram, event_idx: usize) -> Self {
        if let Some(frame_idx) = Self::frame_starting_at(diagram, event_idx) {
            return Selection::Fragment(frame_idx);
        }
        match diagram.section_at(event_idx) {
            Some((frame_idx, k)) => Selection::Section(frame_idx, k),
            None => Selection::Event(event_idx),
        }
    }

    fn section_start(diagram: &SequenceDiagram, frame_idx: usize, k: usize) -> Option<usize> {
        let (at, _) = diagram.fragments.get(frame_idx)?.sections.get(k)?;
        Some(*at)
    }

    fn frame_starting_at(diagram: &SequenceDiagram, event_idx: usize) -> Option<usize> {
        diagram.fragments.iter().position(|f| f.start == event_idx)
    }

    pub fn clamp(
        self,
        participant_count: usize,
        event_count: usize,
        fragment_count: usize,
        box_count: usize,
    ) -> Self {
        match self {
            Selection::Participant(idx) if participant_count > 0 => {
                Selection::Participant(idx.min(participant_count - 1))
            }
            Selection::Event(idx) if event_count > 0 => Selection::Event(idx.min(event_count - 1)),
            Selection::Fragment(idx) | Selection::Section(idx, _) if idx < fragment_count => self,
            Selection::Box(idx) if idx < box_count => self,
            Selection::Participant(_)
            | Selection::Event(_)
            | Selection::Fragment(_)
            | Selection::Section(..)
            | Selection::Box(_) => {
                if participant_count > 0 {
                    Selection::Participant(0)
                } else {
                    Selection::None
                }
            }
            Selection::None => Selection::None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    const BOX_AB_C: &str = "sequenceDiagram\n    box A\n        participant A\n        participant B\n    end\n    participant C\n";
    const X_BOX_A_BOX_B: &str = "sequenceDiagram\n    participant X\n    box\n        participant A\n    end\n    box\n        participant B\n    end\n";
    const BOX_A_THEN_EVENT: &str = "sequenceDiagram\n    box\n        participant A\n    end\n    participant B\n    A->>B: x\n";

    fn events_diagram(body: &str) -> SequenceDiagram {
        let source = format!("sequenceDiagram\n    participant A\n    participant B\n{body}");
        SequenceDiagram::from_mermaid(&source).unwrap()
    }

    fn walk(start: Selection, step: impl Fn(Selection) -> Selection) -> Vec<Selection> {
        let mut path = vec![start];
        loop {
            let last = *path.last().unwrap();
            let next = step(last);
            if next == last {
                return path;
            }
            path.push(next);
        }
    }

    #[rstest]
    #[case::success_valid_participant_stays(
        Selection::Participant(1),
        2,
        0,
        0,
        0,
        Selection::Participant(1)
    )]
    #[case::success_valid_section_stays(
        Selection::Section(1, 3),
        2,
        3,
        2,
        0,
        Selection::Section(1, 3)
    )]
    #[case::success_participant_clamps_to_last(
        Selection::Participant(5),
        2,
        0,
        0,
        0,
        Selection::Participant(1)
    )]
    #[case::success_event_clamps_to_last(Selection::Event(5), 2, 3, 0, 0, Selection::Event(2))]
    #[case::success_event_without_events_falls_back(
        Selection::Event(0),
        2,
        0,
        0,
        0,
        Selection::Participant(0)
    )]
    #[case::success_missing_frame_falls_back(
        Selection::Fragment(1),
        2,
        3,
        1,
        0,
        Selection::Participant(0)
    )]
    #[case::success_missing_section_frame_falls_back(
        Selection::Section(1, 0),
        2,
        3,
        1,
        0,
        Selection::Participant(0)
    )]
    #[case::success_missing_box_falls_back(
        Selection::Box(1),
        3,
        0,
        0,
        1,
        Selection::Participant(0)
    )]
    #[case::success_empty_diagram_gives_none(
        Selection::Participant(0),
        0,
        0,
        0,
        0,
        Selection::None
    )]
    fn clamp_keeps_valid_and_falls_back(
        #[case] given_selection: Selection,
        #[case] given_participants: usize,
        #[case] given_events: usize,
        #[case] given_frames: usize,
        #[case] given_boxes: usize,
        #[case] expected_selection: Selection,
    ) {
        assert_eq!(
            given_selection.clamp(given_participants, given_events, given_frames, given_boxes),
            expected_selection
        );
    }

    #[rstest]
    #[case::success_event_then_frame_then_event(
        "    A->>B: 0\n    opt\n        A->>B: 1\n    end\n    A->>B: 2\n",
        &[
            Selection::Participant(0),
            Selection::Event(0),
            Selection::Fragment(0),
            Selection::Event(1),
            Selection::Event(2),
        ]
    )]
    #[case::success_adjacent_frames(
        "    opt\n        A->>B: 0\n    end\n    loop\n        A->>B: 1\n    end\n",
        &[
            Selection::Participant(0),
            Selection::Fragment(0),
            Selection::Event(0),
            Selection::Fragment(1),
            Selection::Event(1),
        ]
    )]
    #[case::success_frame_with_divider(
        "    alt a\n        A->>B: 0\n    else b\n        A->>B: 1\n    end\n    A->>B: 2\n",
        &[
            Selection::Participant(0),
            Selection::Fragment(0),
            Selection::Event(0),
            Selection::Section(0, 0),
            Selection::Event(1),
            Selection::Event(2),
        ]
    )]
    fn vertical_walk_visits_headers_and_dividers_and_reverses(
        #[case] given_body: &str,
        #[case] expected_walk: &[Selection],
    ) {
        let d = events_diagram(given_body);

        assert_eq!(
            walk(Selection::Participant(0), |s| s.down(&d)),
            expected_walk
        );

        let last = *expected_walk.last().unwrap();
        let up: Vec<_> = expected_walk.iter().rev().copied().collect();
        assert_eq!(walk(last, |s| s.up(&d)), up);
    }

    #[test]
    fn horizontal_walk_goes_through_box_and_back() {
        let d = SequenceDiagram::from_mermaid(BOX_AB_C).unwrap();

        assert_eq!(
            walk(Selection::None, |s| s.right(&d)),
            [
                Selection::None,
                Selection::Box(0),
                Selection::Participant(0),
                Selection::Participant(1),
                Selection::Participant(2),
            ]
        );
        assert_eq!(
            walk(Selection::Participant(2), |s| s.left(&d)),
            [
                Selection::Participant(2),
                Selection::Participant(1),
                Selection::Participant(0),
                Selection::Box(0),
            ]
        );
    }

    #[rstest]
    #[case::success_right_from_free_participant_to_box(
        Selection::Participant(0),
        true,
        Selection::Box(0)
    )]
    #[case::success_right_from_box_to_its_member(
        Selection::Box(0),
        true,
        Selection::Participant(1)
    )]
    #[case::success_right_from_member_to_next_box(
        Selection::Participant(1),
        true,
        Selection::Box(1)
    )]
    #[case::success_left_from_second_box_to_previous_participant(
        Selection::Box(1),
        false,
        Selection::Participant(1)
    )]
    #[case::success_left_from_first_box_to_free_participant(
        Selection::Box(0),
        false,
        Selection::Participant(0)
    )]
    #[case::success_left_from_member_to_its_box(
        Selection::Participant(2),
        false,
        Selection::Box(1)
    )]
    fn box_neighbours_without_leading_box(
        #[case] given_from: Selection,
        #[case] given_right: bool,
        #[case] expected_selection: Selection,
    ) {
        let d = SequenceDiagram::from_mermaid(X_BOX_A_BOX_B).unwrap();

        let next = if given_right {
            given_from.right(&d)
        } else {
            given_from.left(&d)
        };

        assert_eq!(next, expected_selection);
    }

    #[test]
    fn box_goes_down_to_events_and_up_stays() {
        let d = SequenceDiagram::from_mermaid(BOX_A_THEN_EVENT).unwrap();

        assert_eq!(Selection::Box(0).down(&d), Selection::Event(0));
        assert_eq!(Selection::Box(0).up(&d), Selection::Box(0));
        assert_eq!(Selection::Box(0).left(&d), Selection::Box(0));
    }
}
