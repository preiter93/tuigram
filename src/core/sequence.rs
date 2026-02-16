use crate::core::models::Event;

pub struct SequenceDiagram {
    pub participants: Vec<String>,
    pub events: Vec<Event>,
}
