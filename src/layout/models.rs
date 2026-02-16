pub struct ParticipantLayout {
    pub index: usize,
    pub name: String,
    pub x: u16,
}

pub struct MessageLayout {
    pub from_x: u16,
    pub to_x: u16,
    pub y: u16,
    pub text: String,
}
