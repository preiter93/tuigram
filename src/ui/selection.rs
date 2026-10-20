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

    #[test]
    fn clamp_keeps_valid_selection() {
        assert_eq!(
            Selection::Participant(1).clamp(2, 0, 0, 0),
            Selection::Participant(1)
        );
        assert_eq!(Selection::Event(2).clamp(2, 3, 0, 0), Selection::Event(2));
    }

    #[test]
    fn clamp_limits_index_to_last() {
        assert_eq!(
            Selection::Participant(5).clamp(2, 0, 0, 0),
            Selection::Participant(1)
        );
        assert_eq!(Selection::Event(5).clamp(2, 3, 0, 0), Selection::Event(2));
    }

    #[test]
    fn clamp_event_without_events_falls_back_to_participant() {
        assert_eq!(
            Selection::Event(0).clamp(2, 0, 0, 0),
            Selection::Participant(0)
        );
    }

    #[test]
    fn clamp_empty_diagram_clears_selection() {
        assert_eq!(Selection::Participant(0).clamp(0, 0, 0, 0), Selection::None);
        assert_eq!(Selection::Event(0).clamp(0, 0, 0, 0), Selection::None);
        assert_eq!(Selection::None.clamp(2, 2, 0, 0), Selection::None);
    }

    fn diagram(body: &str) -> SequenceDiagram {
        let source = format!("sequenceDiagram\n    participant A\n    participant B\n{body}");
        SequenceDiagram::from_mermaid(&source).unwrap()
    }

    fn walk_down(d: &SequenceDiagram) -> Vec<Selection> {
        let mut path = vec![Selection::Participant(0)];
        loop {
            let next = path.last().unwrap().down(d);
            if next == *path.last().unwrap() {
                return path;
            }
            path.push(next);
        }
    }

    #[test]
    fn walk_visits_frame_header_before_first_event() {
        let d = diagram("    A->>B: 0\n    opt\n        A->>B: 1\n    end\n    A->>B: 2\n");
        let expected = [
            Selection::Participant(0),
            Selection::Event(0),
            Selection::Fragment(0),
            Selection::Event(1),
            Selection::Event(2),
        ];
        assert_eq!(walk_down(&d), expected);

        let mut path = vec![Selection::Event(2)];
        loop {
            let last = *path.last().unwrap();
            let next = last.up(&d);
            if next == last {
                break;
            }
            path.push(next);
        }
        assert_eq!(
            path,
            [
                Selection::Event(2),
                Selection::Event(1),
                Selection::Fragment(0),
                Selection::Event(0),
                Selection::Participant(0),
            ]
        );
    }

    #[test]
    fn walk_handles_frame_at_start_and_adjacent_frames() {
        let d =
            diagram("    opt\n        A->>B: 0\n    end\n    loop\n        A->>B: 1\n    end\n");
        let expected = [
            Selection::Participant(0),
            Selection::Fragment(0),
            Selection::Event(0),
            Selection::Fragment(1),
            Selection::Event(1),
        ];
        assert_eq!(walk_down(&d), expected);

        assert_eq!(Selection::Fragment(0).up(&d), Selection::Participant(0));
        assert_eq!(Selection::Fragment(1).up(&d), Selection::Event(0));
    }

    #[test]
    fn clamp_fragment() {
        assert_eq!(
            Selection::Fragment(1).clamp(2, 3, 2, 0),
            Selection::Fragment(1)
        );
        assert_eq!(
            Selection::Fragment(1).clamp(2, 3, 1, 0),
            Selection::Participant(0)
        );
        assert_eq!(Selection::Fragment(0).clamp(0, 0, 0, 0), Selection::None);
    }

    #[test]
    fn walk_visits_divider_before_first_event_of_its_section() {
        let d = diagram(
            "    alt a\n        A->>B: 0\n    else b\n        A->>B: 1\n    end\n    A->>B: 2\n",
        );
        let expected = [
            Selection::Participant(0),
            Selection::Fragment(0),
            Selection::Event(0),
            Selection::Section(0, 0),
            Selection::Event(1),
            Selection::Event(2),
        ];
        assert_eq!(walk_down(&d), expected);

        let mut path = vec![Selection::Event(2)];
        loop {
            let last = *path.last().unwrap();
            let next = last.up(&d);
            if next == last {
                break;
            }
            path.push(next);
        }
        let expected_up: Vec<Selection> = expected.iter().rev().copied().collect();
        assert_eq!(path, expected_up);
    }

    #[test]
    fn clamp_section() {
        assert_eq!(
            Selection::Section(1, 3).clamp(2, 3, 2, 0),
            Selection::Section(1, 3)
        );
        assert_eq!(
            Selection::Section(1, 0).clamp(2, 3, 1, 0),
            Selection::Participant(0)
        );
    }

    #[test]
    fn clamp_box() {
        assert_eq!(Selection::Box(1).clamp(3, 0, 0, 2), Selection::Box(1));
        assert_eq!(
            Selection::Box(1).clamp(3, 0, 0, 1),
            Selection::Participant(0)
        );
        assert_eq!(Selection::Box(0).clamp(0, 0, 0, 0), Selection::None);
    }

    fn boxed(source: &str) -> SequenceDiagram {
        SequenceDiagram::from_mermaid(source).unwrap()
    }

    #[test]
    fn walk_right_and_left_through_box() {
        let d = boxed(
            "sequenceDiagram\n    box A\n        participant A\n        participant B\n    end\n    participant C\n",
        );
        let mut path = vec![Selection::None];
        loop {
            let last = *path.last().unwrap();
            let next = last.right(&d);
            if next == last {
                break;
            }
            path.push(next);
        }
        let expected = [
            Selection::None,
            Selection::Box(0),
            Selection::Participant(0),
            Selection::Participant(1),
            Selection::Participant(2),
        ];
        assert_eq!(path, expected);

        let mut back = vec![Selection::Participant(2)];
        loop {
            let last = *back.last().unwrap();
            let next = last.left(&d);
            if next == last {
                break;
            }
            back.push(next);
        }
        let expected_back = [
            Selection::Participant(2),
            Selection::Participant(1),
            Selection::Participant(0),
            Selection::Box(0),
        ];
        assert_eq!(back, expected_back);
    }

    #[test]
    fn walk_handles_adjacent_boxes_and_box_not_at_start() {
        let d = boxed(
            "sequenceDiagram\n    participant X\n    box\n        participant A\n    end\n    box\n        participant B\n    end\n",
        );
        assert_eq!(Selection::Participant(0).right(&d), Selection::Box(0));
        assert_eq!(Selection::Box(0).right(&d), Selection::Participant(1));
        assert_eq!(Selection::Participant(1).right(&d), Selection::Box(1));
        assert_eq!(Selection::Box(1).left(&d), Selection::Participant(1));
        assert_eq!(Selection::Box(0).left(&d), Selection::Participant(0));
        assert_eq!(Selection::Participant(2).left(&d), Selection::Box(1));
    }

    #[test]
    fn box_goes_down_to_events_and_up_stays() {
        let d = boxed(
            "sequenceDiagram\n    box\n        participant A\n    end\n    participant B\n    A->>B: x\n",
        );
        assert_eq!(Selection::Box(0).down(&d), Selection::Event(0));
        assert_eq!(Selection::Box(0).up(&d), Selection::Box(0));
        assert_eq!(Selection::Box(0).left(&d), Selection::Box(0));
    }
}
