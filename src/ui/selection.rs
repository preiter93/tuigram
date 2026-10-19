#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Selection {
    #[default]
    None,
    Participant(usize),
    Event(usize),
}

impl Selection {
    pub fn left(self, participant_count: usize) -> Self {
        match self {
            Selection::None if participant_count > 0 => Selection::Participant(0),
            Selection::Participant(idx) if idx > 0 => Selection::Participant(idx - 1),
            _ => self,
        }
    }

    pub fn right(self, participant_count: usize) -> Self {
        match self {
            Selection::None if participant_count > 0 => Selection::Participant(0),
            Selection::Participant(idx) if idx + 1 < participant_count => {
                Selection::Participant(idx + 1)
            }
            _ => self,
        }
    }

    pub fn down(self, participant_count: usize, event_count: usize) -> Self {
        match self {
            Selection::None if participant_count > 0 => Selection::Participant(0),
            Selection::None | Selection::Participant(_) if event_count > 0 => Selection::Event(0),
            Selection::Event(idx) if idx + 1 < event_count => Selection::Event(idx + 1),
            _ => self,
        }
    }

    pub fn clamp(self, participant_count: usize, event_count: usize) -> Self {
        match self {
            Selection::Participant(idx) if participant_count > 0 => {
                Selection::Participant(idx.min(participant_count - 1))
            }
            Selection::Event(idx) if event_count > 0 => Selection::Event(idx.min(event_count - 1)),
            Selection::Participant(_) | Selection::Event(_) => {
                if participant_count > 0 {
                    Selection::Participant(0)
                } else {
                    Selection::None
                }
            }
            Selection::None => Selection::None,
        }
    }

    pub fn up(self, participant_count: usize) -> Self {
        match self {
            Selection::None | Selection::Event(0) if participant_count > 0 => {
                Selection::Participant(0)
            }
            Selection::Event(idx) if idx > 0 => Selection::Event(idx - 1),
            _ => self,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamp_keeps_valid_selection() {
        assert_eq!(
            Selection::Participant(1).clamp(2, 0),
            Selection::Participant(1)
        );
        assert_eq!(Selection::Event(2).clamp(2, 3), Selection::Event(2));
    }

    #[test]
    fn clamp_limits_index_to_last() {
        assert_eq!(
            Selection::Participant(5).clamp(2, 0),
            Selection::Participant(1)
        );
        assert_eq!(Selection::Event(5).clamp(2, 3), Selection::Event(2));
    }

    #[test]
    fn clamp_event_without_events_falls_back_to_participant() {
        assert_eq!(Selection::Event(0).clamp(2, 0), Selection::Participant(0));
    }

    #[test]
    fn clamp_empty_diagram_clears_selection() {
        assert_eq!(Selection::Participant(0).clamp(0, 0), Selection::None);
        assert_eq!(Selection::Event(0).clamp(0, 0), Selection::None);
        assert_eq!(Selection::None.clamp(2, 2), Selection::None);
    }
}
