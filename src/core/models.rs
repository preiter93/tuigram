#[derive(Clone)]
pub enum Event {
    Message {
        from: usize,
        to: usize,
        text: String,
    },
}
