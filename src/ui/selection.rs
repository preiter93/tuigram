#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum Selection {
    #[default]
    None,
    Participant(usize),
    Event(usize),
}

impl Selection {
    pub fn cycle_back(self, participant_count: usize, event_count: usize) -> Self {
        match self {
            Selection::None => {
                if event_count > 0 {
                    Selection::Event(event_count - 1)
                } else if participant_count > 0 {
                    Selection::Participant(participant_count - 1)
                } else {
                    Selection::None
                }
            }
            Selection::Participant(idx) => {
                if idx > 0 {
                    Selection::Participant(idx - 1)
                } else if event_count > 0 {
                    Selection::Event(event_count - 1)
                } else if participant_count > 0 {
                    Selection::Participant(participant_count - 1)
                } else {
                    Selection::None
                }
            }
            Selection::Event(idx) => {
                if idx > 0 {
                    Selection::Event(idx - 1)
                } else if participant_count > 0 {
                    Selection::Participant(participant_count - 1)
                } else if event_count > 0 {
                    Selection::Event(event_count - 1)
                } else {
                    Selection::None
                }
            }
        }
    }

    pub fn next_event(self, event_count: usize, participant_count: usize) -> Self {
        match self {
            Selection::Event(idx) if idx + 1 < event_count => Selection::Event(idx + 1),
            Selection::Event(_) if participant_count > 0 => Selection::Participant(0),
            _ if event_count > 0 => Selection::Event(0),
            _ if participant_count > 0 => Selection::Participant(0),
            _ => Selection::None,
        }
    }

    pub fn prev_event(self, event_count: usize, participant_count: usize) -> Self {
        match self {
            Selection::Event(idx) if idx > 0 => Selection::Event(idx - 1),
            Selection::Event(_) if participant_count > 0 => {
                Selection::Participant(participant_count - 1)
            }
            _ if event_count > 0 => Selection::Event(event_count - 1),
            _ if participant_count > 0 => Selection::Participant(participant_count - 1),
            _ => Selection::None,
        }
    }

    pub fn next_participant(self, participant_count: usize, event_count: usize) -> Self {
        match self {
            Selection::Participant(idx) if idx + 1 < participant_count => {
                Selection::Participant(idx + 1)
            }
            Selection::Participant(_) if event_count > 0 => Selection::Event(0),
            _ if participant_count > 0 => Selection::Participant(0),
            _ if event_count > 0 => Selection::Event(0),
            _ => Selection::None,
        }
    }

    pub fn prev_participant(self, participant_count: usize, event_count: usize) -> Self {
        match self {
            Selection::Participant(idx) if idx > 0 => Selection::Participant(idx - 1),
            Selection::Participant(_) if event_count > 0 => Selection::Event(event_count - 1),
            Selection::Participant(_) if participant_count > 0 => {
                Selection::Participant(participant_count - 1)
            }
            _ if participant_count > 0 => Selection::Participant(participant_count - 1),
            _ if event_count > 0 => Selection::Event(event_count - 1),
            _ => Selection::None,
        }
    }

    pub fn cycle(self, participant_count: usize, event_count: usize) -> Self {
        match self {
            Selection::None => {
                if participant_count > 0 {
                    Selection::Participant(0)
                } else if event_count > 0 {
                    Selection::Event(0)
                } else {
                    Selection::None
                }
            }
            Selection::Participant(idx) => {
                if idx + 1 < participant_count {
                    Selection::Participant(idx + 1)
                } else if event_count > 0 {
                    Selection::Event(0)
                } else if participant_count > 0 {
                    Selection::Participant(0)
                } else {
                    Selection::None
                }
            }
            Selection::Event(idx) => {
                if idx + 1 < event_count {
                    Selection::Event(idx + 1)
                } else if participant_count > 0 {
                    Selection::Participant(0)
                } else if event_count > 0 {
                    Selection::Event(0)
                } else {
                    Selection::None
                }
            }
        }
    }
}
