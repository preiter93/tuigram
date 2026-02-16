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

    pub fn to_mermaid(&self) -> String {
        let mut lines = vec!["sequenceDiagram".to_string()];

        for name in &self.participants {
            lines.push(format!("    participant {name}"));
        }

        for event in &self.events {
            let Event::Message { from, to, text } = event;
            if let (Some(from_name), Some(to_name)) =
                (self.participants.get(*from), self.participants.get(*to))
            {
                lines.push(format!("    {from_name}->>{to_name}:{text}"));
            }
        }

        lines.join("\n") + "\n"
    }
}
