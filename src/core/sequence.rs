use super::models::Event;
use anyhow::{Result, bail};

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

    pub fn remove_event(&mut self, idx: usize) {
        if idx < self.events.len() {
            self.events.remove(idx);
        }
    }

    pub fn swap_participants(&mut self, a: usize, b: usize) {
        if a >= self.participants.len() || b >= self.participants.len() {
            return;
        }
        self.participants.swap(a, b);
        for e in &mut self.events {
            let Event::Message { from, to, .. } = e;
            if *from == a {
                *from = b;
            } else if *from == b {
                *from = a;
            }
            if *to == a {
                *to = b;
            } else if *to == b {
                *to = a;
            }
        }
    }

    pub fn point_event_left(&mut self, idx: usize) {
        if let Some(Event::Message { from, to, .. }) = self.events.get_mut(idx)
            && *from < *to
        {
            std::mem::swap(from, to);
        }
    }

    pub fn point_event_right(&mut self, idx: usize) {
        if let Some(Event::Message { from, to, .. }) = self.events.get_mut(idx)
            && *from > *to
        {
            std::mem::swap(from, to);
        }
    }

    pub fn remove_participant(&mut self, idx: usize) {
        if idx >= self.participants.len() {
            return;
        }
        self.participants.remove(idx);
        self.events.retain(|e| {
            let Event::Message { from, to, .. } = e;
            *from != idx && *to != idx
        });
        for e in &mut self.events {
            let Event::Message { from, to, .. } = e;
            if *from > idx {
                *from -= 1;
            }
            if *to > idx {
                *to -= 1;
            }
        }
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

    pub fn from_mermaid(input: &str) -> Result<Self> {
        let mut diagram = SequenceDiagram::new();
        let mut lines = input.lines();

        let first_line = lines.next().map_or("", str::trim);
        if first_line != "sequenceDiagram" {
            bail!("First line must be 'sequenceDiagram'");
        }

        for line in lines {
            let trimmed = line.trim();

            if trimmed.is_empty() {
                bail!("Empty lines are not supported");
            }
            if trimmed.starts_with("%%") {
                bail!("Comments are not supported");
            }

            // Parse participant
            if let Some(rest) = trimmed.strip_prefix("participant ") {
                let name = rest.trim().to_string();
                if name.is_empty() {
                    bail!("Invalid participant declaration: {line}");
                }
                if !diagram.participants.contains(&name) {
                    diagram.participants.push(name);
                }
                continue;
            }

            // Parse message
            if let Some(arrow_pos) = trimmed.find("->>") {
                let from_name = trimmed[..arrow_pos].trim();
                let rest = &trimmed[arrow_pos + 3..];

                let (to_name, message) = if let Some(colon_pos) = rest.find(':') {
                    let to = rest[..colon_pos].trim();
                    let msg = rest[colon_pos + 1..].trim();
                    (to, msg.to_string())
                } else {
                    bail!("Invalid message syntax (missing ':'): {line}");
                };

                if from_name.is_empty() || to_name.is_empty() {
                    bail!("Invalid message syntax: {line}");
                }

                if !diagram.participants.contains(&from_name.to_string()) {
                    diagram.participants.push(from_name.to_string());
                }
                if !diagram.participants.contains(&to_name.to_string()) {
                    diagram.participants.push(to_name.to_string());
                }

                let from_idx = diagram
                    .participants
                    .iter()
                    .position(|p| p == from_name)
                    .unwrap();

                let to_idx = diagram
                    .participants
                    .iter()
                    .position(|p| p == to_name)
                    .unwrap();

                diagram.events.push(Event::Message {
                    from: from_idx,
                    to: to_idx,
                    text: message,
                });
                continue;
            }

            bail!("Unsupported mermaid feature: {trimmed}");
        }

        Ok(diagram)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip() {
        let mut diagram = SequenceDiagram::new();
        diagram.add_participant("Alice".to_string());
        diagram.add_participant("Bob".to_string());
        diagram.add_message(0, 1, "Hello".to_string());
        diagram.add_message(1, 0, "Hi there".to_string());

        let mermaid = diagram.to_mermaid();
        let parsed = SequenceDiagram::from_mermaid(&mermaid).unwrap();

        assert_eq!(parsed.participants, diagram.participants);
        assert_eq!(parsed.events.len(), diagram.events.len());
    }

    #[test]
    fn test_from_mermaid_basic() {
        let input = "sequenceDiagram
    participant Alice
    participant Bob
    Alice->>Bob: Hello
    Bob->>Alice: Hi";
        let diagram = SequenceDiagram::from_mermaid(input).unwrap();
        assert_eq!(diagram.participants, vec!["Alice", "Bob"]);
        assert_eq!(diagram.events.len(), 2);
    }

    #[test]
    fn test_from_mermaid_auto_participants() {
        let input = "sequenceDiagram
    Alice->>Bob: Hello";
        let diagram = SequenceDiagram::from_mermaid(input).unwrap();
        assert_eq!(diagram.participants, vec!["Alice", "Bob"]);
    }

    #[test]
    fn test_invalid_header() {
        let input = "Alice->>Bob: Hello";
        let result = SequenceDiagram::from_mermaid(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_unsupported_feature() {
        let input = "sequenceDiagram
    Note right of Alice: This is a note";
        let result = SequenceDiagram::from_mermaid(input);
        assert!(result.is_err());
    }
}
