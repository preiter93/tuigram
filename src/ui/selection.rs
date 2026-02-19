#[derive(Default, Clone, Copy, PartialEq, Eq)]
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
