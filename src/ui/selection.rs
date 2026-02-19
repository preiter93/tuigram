#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum Selection {
    #[default]
    None,
    Participant(usize),
    Event(usize),
}

impl Selection {
    /// Move left within participants only
    pub fn left(self, participant_count: usize) -> Self {
        match self {
            Selection::None if participant_count > 0 => Selection::Participant(0),
            Selection::Participant(idx) if idx > 0 => Selection::Participant(idx - 1),
            _ => self,
        }
    }

    /// Move right within participants only
    pub fn right(self, participant_count: usize) -> Self {
        match self {
            Selection::None if participant_count > 0 => Selection::Participant(0),
            Selection::Participant(idx) if idx + 1 < participant_count => {
                Selection::Participant(idx + 1)
            }
            _ => self,
        }
    }

    /// Move down: from participant to first event, or to next event
    pub fn down(self, participant_count: usize, event_count: usize) -> Self {
        match self {
            Selection::None if participant_count > 0 => Selection::Participant(0),
            Selection::None if event_count > 0 => Selection::Event(0),
            Selection::Participant(_) if event_count > 0 => Selection::Event(0),
            Selection::Event(idx) if idx + 1 < event_count => Selection::Event(idx + 1),
            _ => self,
        }
    }

    /// Move up: from event to previous event, or from first event to first participant
    pub fn up(self, participant_count: usize, event_count: usize) -> Self {
        match self {
            Selection::None if event_count > 0 => Selection::Event(event_count - 1),
            Selection::None if participant_count > 0 => Selection::Participant(0),
            Selection::Event(0) if participant_count > 0 => Selection::Participant(0),
            Selection::Event(idx) if idx > 0 => Selection::Event(idx - 1),
            _ => self,
        }
    }
}
