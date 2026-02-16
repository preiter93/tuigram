use super::models::Event;

#[derive(Default, Clone)]
pub struct SequenceDiagram {
    pub participants: Vec<String>,
    pub events: Vec<Event>,
}

impl SequenceDiagram {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_participant(&mut self, name: String) {
        self.participants.push(name);
    }

    pub fn add_message(&mut self, from: usize, to: usize, text: String) {
        if from < self.participants.len() && to < self.participants.len() {
            self.events.push(Event::Message { from, to, text });
        }
    }

    pub fn participant_count(&self) -> usize {
        self.participants.len()
    }

    pub fn event_count(&self) -> usize {
        self.events.len()
    }
}
