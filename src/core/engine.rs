use crate::core::events::{Event, EventBus};
use crate::scripting::action_executor::ActionExecutor;
use crate::scripting::node_graph::NodeGraph;
use crate::tracking::hotkeys::HotkeyManager;
use blue_engine::Engine;
use blue_engine_utilities::egui_plugin;

pub struct VKawaiiEngine {
    pub engine: Engine,
    pub event_bus: EventBus,
    pub hotkey_manager: HotkeyManager,
    pub action_executor: ActionExecutor,
}

impl VKawaiiEngine {
    pub fn new() -> anyhow::Result<Self> {
        let settings = blue_engine::EngineSettings {
            alpha_mode: blue_engine::wgpu::CompositeAlphaMode::PreMultiplied,
            ..Default::default()
        };

        let mut engine = Engine::new_config(settings)?;
        engine.window.set_transparent(true);
        engine.renderer.set_clear_color(0.0, 0.0, 0.0, 0.0);

        let gui_context = egui_plugin::EGUIPlugin::new();
        engine.signals.add_signal("egui", Box::new(gui_context));

        let event_bus = EventBus::new();
        let hotkey_manager = HotkeyManager::new(event_bus.get_sender())?;

        let action_executor = ActionExecutor::new(NodeGraph::default());

        #[cfg(windows)]
        if let Some(spout_sender) = crate::streaming::spout_sender::SpoutSender::new("VKawaii") {
            engine.signals.add_signal(
                "spout",
                Box::new(crate::streaming::spout_sender::SpoutSignal {
                    sender: spout_sender,
                }),
            );
        }

        crate::streaming::websocket_server::start_websocket_server(event_bus.get_sender());

        // Registering a Sample hotkey (like F1)
        use global_hotkey::hotkey::{Code, HotKey, Modifiers};
        let hotkey = HotKey::new(Some(Modifiers::CONTROL), Code::KeyA);
        let _ = hotkey_manager.register_hotkey(hotkey);

        Ok(Self {
            engine,
            event_bus,
            hotkey_manager,
            action_executor,
        })
    }

    pub fn run(mut self) -> anyhow::Result<()> {
        self.engine.update_loop(move |_engine| {
            // Checking the Hotkeys
            self.hotkey_manager.poll();

            // Dealing with Events coming from the Eventbus
            while let Some(event) = self.event_bus.poll() {
                println!("Received event: {:?}", event);
                match event {
                    Event::Hotkey(_id) => {
                        self.action_executor.trigger("HotkeyTrigger");
                    }
                    Event::WebSocketMessage(msg) => {
                        println!("WebSocket Message Received: {}", msg);
                    }
                }
            }

            // Running the Scripting Engine updates
            self.action_executor.update();

            // Doing the main Render Loop
            let egui_plugin = _engine
                .signals
                .get_signal::<egui_plugin::EGUIPlugin>("egui")
                .expect("Plugin not found")
                .expect("Plugin type mismatch");

            egui_plugin.ui(
                |ctx| {
                    crate::ui::overlay::draw_ui(ctx, &mut self.action_executor.graph);
                },
                &_engine.window,
            );
        })?;
        Ok(())
    }
}
