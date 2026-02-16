pub struct Participant(String);

pub enum Event {
    Message {
        from: usize,
        to: usize,
        text: String,
        // kind: MessageKind,
    },
}

pub enum MessageKind {
    Sync,
    Async,
}
