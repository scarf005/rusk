pub enum Message {
    Move {
        x: i32,
        y: i32,
    },
    Write(String),
    Quit,
}

pub fn score(message: Message) -> i32 {
    match message {
        Message::Move { x, y } => x + y,
        Message::Write(text) => text.len() as i32,
        Message::Quit => 0,
    }
}
