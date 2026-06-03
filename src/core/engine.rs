use blue_engine::Engine;
use blue_engine_utilities::egui_plugin;

pub struct VKawaiiEngine {
    pub engine: Engine,
}

impl VKawaiiEngine {
    pub fn new() -> anyhow::Result<Self> {
        let mut engine = Engine::new()?;

        let gui_context = egui_plugin::EGUIPlugin::new();
        engine.signals.add_signal("egui", Box::new(gui_context));

        Ok(Self { engine })
    }

    pub fn run(mut self) -> anyhow::Result<()> {
        self.engine.update_loop(move |_engine| {
            // Main Render Loop

            let egui_plugin = _engine
                .signals
                .get_signal::<egui_plugin::EGUIPlugin>("egui")
                .expect("Plugin not found")
                .expect("Plugin type mismatch");

            egui_plugin.ui(
                |ctx| {
                    crate::ui::overlay::draw_ui(ctx);
                },
                &_engine.window,
            );
        })?;
        Ok(())
    }
}
