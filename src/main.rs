use iced::widget::{button, column, text Column};

struct Counter {
    value : i32,
}

#[derive(Debug, Clone, Copy)]
pub enum Message {
    IncrementPressed,
    
}

fn main() {
    println!("Hello, world!");
}
