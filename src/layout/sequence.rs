use crate::{
    core::{models::Event, sequence::SequenceDiagram},
    layout::models::{MessageLayout, ParticipantLayout},
};

pub struct SequenceLayout {
    pub width: u16,
    pub height: u16,
    pub participants: Vec<ParticipantLayout>,
    pub messages: Vec<MessageLayout>,
}

impl SequenceLayout {
    pub fn compute(diagram: &SequenceDiagram, term_width: u16) -> Self {
        let header_height = 3;
        let mut current_y = header_height;

        let participants = {
            let mut participants = Vec::new();
            let count = diagram.participants.len().max(1) as u16;

            let spacing = term_width / (count + 1);

            for (i, name) in diagram.participants.iter().enumerate() {
                let x = spacing * (i as u16 + 1);

                participants.push(ParticipantLayout {
                    index: i,
                    name: name.clone(),
                    x,
                });
            }

            participants
        };

        let messages = {
            let mut messages = Vec::new();

            let message_spacing = 4;

            for event in &diagram.events {
                match event {
                    Event::Message { from, to, text } => {
                        let from_x = participants[*from].x;
                        let to_x = participants[*to].x;

                        messages.push(MessageLayout {
                            from_x,
                            to_x,
                            y: current_y,
                            text: text.clone(),
                        });

                        current_y += message_spacing;
                    }
                }
            }

            messages
        };

        Self {
            width: term_width,
            height: current_y + 2,
            participants,
            messages,
        }
    }
}
