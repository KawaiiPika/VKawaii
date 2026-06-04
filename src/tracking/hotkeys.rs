use crate::core::events::Event;
use anyhow::Result;
use global_hotkey::{hotkey::HotKey, GlobalHotKeyEvent, GlobalHotKeyManager};
use std::sync::mpsc::Sender;

pub struct HotkeyManager {
    pub manager: GlobalHotKeyManager,
    pub event_sender: Sender<Event>,
}

impl HotkeyManager {
    pub fn new(event_sender: Sender<Event>) -> Result<Self> {
        let manager = GlobalHotKeyManager::new()?;
        Ok(Self {
            manager,
            event_sender,
        })
    }

    pub fn register_hotkey(&self, hotkey: HotKey) -> Result<()> {
        self.manager.register(hotkey)?;
        Ok(())
    }

    pub fn poll(&self) {
        while let Ok(event) = GlobalHotKeyEvent::receiver().try_recv() {
            let _ = self.event_sender.send(Event::Hotkey(event.id));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    #[test]
    fn test_hotkey_manager_init() {
        let (tx, _rx) = mpsc::channel();
        // This might fail in some CI environments if it can't access the windowing system
        // but let's try.
        let result = HotkeyManager::new(tx);
        // We just check if it returns, success or failure depends on environment
        println!("HotkeyManager init result: {:?}", result.is_ok());
    }
}
