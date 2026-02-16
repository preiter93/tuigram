pub struct Participant(pub String);

#[derive(Clone)]
pub enum Event {
    Message {
        from: usize,
        to: usize,
        text: String,
    },
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MessageKind {
    Sync,
    Async,
}
