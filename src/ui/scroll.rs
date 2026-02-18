pub struct ScrollState {
    pub offset: u16,
    pub height: u16,
}

impl ScrollState {
    pub fn new() -> Self {
        ScrollState {
            offset: 0,
            height: 0,
        }
    }
}
