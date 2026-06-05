use std::sync::mpsc::{self, Receiver, Sender};

#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    Hotkey(u32),
    WebSocketMessage(String),
    // Sticking Future events here
}

pub struct EventBus {
    sender: Sender<Event>,
    receiver: Receiver<Event>,
}

impl EventBus {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel();
        Self { sender, receiver }
    }

    pub fn get_sender(&self) -> Sender<Event> {
        self.sender.clone()
    }

    pub fn poll(&self) -> Option<Event> {
        match self.receiver.try_recv() {
            Ok(event) => Some(event),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => {
                eprintln!("EventBus: all senders disconnected!");
                None
            }
        }
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_bus() {
        let bus = EventBus::new();
        let sender = bus.get_sender();

        sender.send(Event::Hotkey(1)).unwrap();
        sender.send(Event::Hotkey(2)).unwrap();

        assert_eq!(bus.poll(), Some(Event::Hotkey(1)));
        assert_eq!(bus.poll(), Some(Event::Hotkey(2)));
        assert_eq!(bus.poll(), None);
    }
}
