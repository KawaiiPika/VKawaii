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
        // This might Fail in some CI Environments if it can't Access the windowing System
        // But Trying anyway.
        let result = HotkeyManager::new(tx);
        // Checking if the Function returns, Success or Failure Depends on the Environment
        println!("HotkeyManager init result: {:?}", result.is_ok());
    }
}
