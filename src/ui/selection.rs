#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum Selection {
    #[default]
    None,
    Participant(usize),
    Event(usize),
}

impl Selection {
    /// Move to the next item in order: Participants -> Events
    /// Does not cycle - stops at the last event
    pub fn next(self, participant_count: usize, event_count: usize) -> Self {
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
                } else {
                    Selection::Participant(idx)
                }
            }
            Selection::Event(idx) => {
                if idx + 1 < event_count {
                    Selection::Event(idx + 1)
                } else {
                    Selection::Event(idx)
                }
            }
        }
    }

    /// Move to the previous item in order: Events -> Participants
    /// Does not cycle - stops at the first participant
    pub fn prev(self, participant_count: usize) -> Self {
        match self {
            Selection::None => Selection::None,
            Selection::Event(idx) => {
                if idx > 0 {
                    Selection::Event(idx - 1)
                } else if participant_count > 0 {
                    Selection::Participant(participant_count - 1)
                } else {
                    Selection::Event(0)
                }
            }
            Selection::Participant(idx) => {
                if idx > 0 {
                    Selection::Participant(idx - 1)
                } else {
                    Selection::Participant(0)
                }
            }
        }
    }
}
