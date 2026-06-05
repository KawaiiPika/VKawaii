use crate::scripting::node_graph::NodeGraph;
use blue_engine_utilities::egui_plugin::egui;
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug)]
pub struct HullConfig {
    pub x_squash: f32,
    pub y_squash: f32,
    pub z_squash: f32,
    pub shrink_factor: f32,
}

impl Default for HullConfig {
    fn default() -> Self {
        Self {
            x_squash: 1.0,
            y_squash: 1.0,
            z_squash: 1.0,
            shrink_factor: 0.88,
        }
    }
}

#[derive(Clone, Debug)]
pub struct SpringColliderConfig {
    pub radius: f32,
    pub offset_x: f32,
    pub offset_y: f32,
    pub offset_z: f32,
    pub initial_radius: f32,
    pub initial_offset_x: f32,
    pub initial_offset_y: f32,
    pub initial_offset_z: f32,
}

pub struct OverlayState {
    pub show_spring_bone_editor: bool,
    pub show_material_editor: bool,
    pub show_node_editor: bool,

    // Visibility toggles
    pub show_body_hulls: bool,
    pub show_spring_bone_hulls: bool,
    pub show_spring_colliders: bool,

    pub global_gravity: f32,
    pub global_stiffness: f32,
    pub hull_configs: HashMap<String, HullConfig>,
    pub hair_hull_configs: HashMap<String, HullConfig>,
    pub hair_hull_toggles: HashMap<String, bool>,
    pub spring_colliders: HashMap<String, SpringColliderConfig>,
    pub spring_collider_keys: Vec<String>,
    pub dynamic_rebuild: bool,
    pub trigger_rebuild: bool,
    pub selected_hull_key: Option<String>,
    pub scroll_requested: bool,
    pub mouse_pos: (f32, f32),
    pub ui_wants_pointer: bool,

    // Node Editor Dragging State
    pub drag_start_pin: Option<uuid::Uuid>,
    pub pin_positions: std::collections::HashMap<uuid::Uuid, egui::Rect>,
}

impl Default for OverlayState {
    fn default() -> Self {
        Self {
            show_spring_bone_editor: true,
            show_material_editor: false,
            show_node_editor: false,
            show_body_hulls: true,
            show_spring_bone_hulls: true,
            show_spring_colliders: true,
            global_gravity: -9.81,
            global_stiffness: 1.0,
            hull_configs: HashMap::new(),
            hair_hull_configs: HashMap::new(),
            hair_hull_toggles: HashMap::new(),
            spring_colliders: HashMap::new(),
            spring_collider_keys: Vec::new(),
            dynamic_rebuild: false,
            trigger_rebuild: false,
            selected_hull_key: None,
            scroll_requested: false,
            mouse_pos: (0.0, 0.0),
            ui_wants_pointer: false,
            drag_start_pin: None,
            pin_positions: std::collections::HashMap::new(),
        }
    }
}

lazy_static! {
    pub static ref OVERLAY_STATE: Arc<Mutex<OverlayState>> =
        Arc::new(Mutex::new(OverlayState::default()));
}

pub fn draw_ui(ctx: &egui::Context, graph: &mut NodeGraph) {
    let mut state = OVERLAY_STATE.lock().unwrap();

    if let Some(pos) = ctx.input(|i| i.pointer.latest_pos()) {
        state.mouse_pos = (pos.x, pos.y);
    }

    state.ui_wants_pointer = ctx.wants_pointer_input() || ctx.is_pointer_over_area();

    egui::SidePanel::right("controls_panel").show(ctx, |ui| {
        ui.heading("Vtubing Engine");
        ui.separator();

        ui.checkbox(&mut state.show_spring_bone_editor, "Spring Bone Editor");
        ui.checkbox(&mut state.show_node_editor, "Node Editor");
    });

    if state.show_spring_bone_editor {
        egui::Window::new("Spring Bone / Hull Editor").show(ctx, |ui| {
            ui.heading("Body Collision Hulls");
            ui.label("Adjust the size and shape of collision hulls.");

            ui.checkbox(&mut state.dynamic_rebuild, "Dynamic Rebuild (may drop FPS)");
            if ui.button("Rebuild Hulls Now").clicked() {
                state.trigger_rebuild = true;
            }

            ui.separator();
            ui.heading("Visibility Toggles");
            ui.checkbox(
                &mut state.show_body_hulls,
                "Show Body Hulls (Blue Capsules)",
            );
            ui.checkbox(
                &mut state.show_spring_bone_hulls,
                "Show Spring Bone Hulls (Yellow Dots)",
            );
            ui.checkbox(
                &mut state.show_spring_colliders,
                "Show Spring Colliders (Orange Spheres)",
            );

            ui.separator();

            let mut any_changed = false;

            egui::ScrollArea::vertical().show(ui, |ui| {
                let mut keys: Vec<String> = state.hull_configs.keys().cloned().collect();
                keys.sort();
                for key in keys {
                    ui.push_id(&key, |ui| {
                        let is_selected = state.selected_hull_key.as_ref() == Some(&key);
                        let header_text = if is_selected {
                            egui::RichText::new(&key).color(egui::Color32::YELLOW)
                        } else {
                            egui::RichText::new(&key)
                        };

                        let response = ui.collapsing(header_text, |ui| {
                            let config = state.hull_configs.get_mut(&key).unwrap();

                            ui.horizontal(|ui| {
                                if ui.button("⟲").clicked() {
                                    config.x_squash = 1.0;
                                    any_changed = true;
                                }
                                if ui
                                    .add(
                                        egui::Slider::new(&mut config.x_squash, 0.5..=1.5)
                                            .text("X Squash"),
                                    )
                                    .changed()
                                {
                                    any_changed = true;
                                }
                            });
                            ui.horizontal(|ui| {
                                if ui.button("⟲").clicked() {
                                    config.y_squash = 1.0;
                                    any_changed = true;
                                }
                                if ui
                                    .add(
                                        egui::Slider::new(&mut config.y_squash, 0.5..=1.5)
                                            .text("Y Squash"),
                                    )
                                    .changed()
                                {
                                    any_changed = true;
                                }
                            });
                            ui.horizontal(|ui| {
                                if ui.button("⟲").clicked() {
                                    config.z_squash = 1.0;
                                    any_changed = true;
                                }
                                if ui
                                    .add(
                                        egui::Slider::new(&mut config.z_squash, 0.5..=1.5)
                                            .text("Z Squash"),
                                    )
                                    .changed()
                                {
                                    any_changed = true;
                                }
                            });
                            ui.horizontal(|ui| {
                                if ui.button("⟲").clicked() {
                                    config.shrink_factor = 0.88;
                                    any_changed = true;
                                }
                                if ui
                                    .add(
                                        egui::Slider::new(&mut config.shrink_factor, 0.5..=1.0)
                                            .text("Overall Shrink"),
                                    )
                                    .changed()
                                {
                                    any_changed = true;
                                }
                            });
                        });

                        if response.header_response.clicked() {
                            state.selected_hull_key = Some(key.clone());
                        }

                        if is_selected && state.scroll_requested {
                            response
                                .header_response
                                .scroll_to_me(Some(egui::Align::Center));
                        }

                        ui.separator();
                    });
                }

                ui.heading("Spring bone hulls");
                ui.label("Enable/disable self-collision and shape spring bone hulls.");
                ui.separator();

                let mut hair_keys: Vec<String> = state.hair_hull_configs.keys().cloned().collect();
                hair_keys.sort();
                for key in hair_keys {
                    ui.push_id(format!("hair_{}", key), |ui| {
                        let is_selected = state.selected_hull_key.as_ref() == Some(&key);
                        let mut enabled = *state.hair_hull_toggles.get(&key).unwrap_or(&false);
                        ui.horizontal(|ui| {
                            if ui.checkbox(&mut enabled, "").changed() {
                                state.hair_hull_toggles.insert(key.clone(), enabled);
                                any_changed = true;
                            }

                            let header_text = if is_selected {
                                egui::RichText::new(&key).color(egui::Color32::YELLOW)
                            } else {
                                egui::RichText::new(&key)
                            };

                            let response = ui.collapsing(header_text, |ui| {
                                let config = state.hair_hull_configs.get_mut(&key).unwrap();

                                ui.horizontal(|ui| {
                                    if ui.button("⟲").clicked() {
                                        config.x_squash = 1.0;
                                        any_changed = true;
                                    }
                                    if ui
                                        .add(
                                            egui::Slider::new(&mut config.x_squash, 0.5..=1.5)
                                                .text("X Squash"),
                                        )
                                        .changed()
                                    {
                                        any_changed = true;
                                    }
                                });
                                ui.horizontal(|ui| {
                                    if ui.button("⟲").clicked() {
                                        config.y_squash = 1.0;
                                        any_changed = true;
                                    }
                                    if ui
                                        .add(
                                            egui::Slider::new(&mut config.y_squash, 0.5..=1.5)
                                                .text("Y Squash"),
                                        )
                                        .changed()
                                    {
                                        any_changed = true;
                                    }
                                });
                                ui.horizontal(|ui| {
                                    if ui.button("⟲").clicked() {
                                        config.z_squash = 1.0;
                                        any_changed = true;
                                    }
                                    if ui
                                        .add(
                                            egui::Slider::new(&mut config.z_squash, 0.5..=1.5)
                                                .text("Z Squash"),
                                        )
                                        .changed()
                                    {
                                        any_changed = true;
                                    }
                                });
                                ui.horizontal(|ui| {
                                    if ui.button("⟲").clicked() {
                                        config.shrink_factor = 0.88;
                                        any_changed = true;
                                    }
                                    if ui
                                        .add(
                                            egui::Slider::new(&mut config.shrink_factor, 0.5..=1.0)
                                                .text("Overall Shrink"),
                                        )
                                        .changed()
                                    {
                                        any_changed = true;
                                    }
                                });
                            });

                            if response.header_response.clicked() {
                                state.selected_hull_key = Some(key.clone());
                            }

                            if is_selected && state.scroll_requested {
                                response
                                    .header_response
                                    .scroll_to_me(Some(egui::Align::Center));
                            }
                        });
                        ui.separator();
                    });
                }

                ui.heading("VRM Spring Colliders");
                ui.label("Adjust the radius and offset of VRM-defined colliders.");
                ui.separator();

                for (idx, key) in state.spring_collider_keys.clone().iter().enumerate() {
                    ui.push_id(format!("vrm_col_{}", idx), |ui| {
                        let is_selected = state.selected_hull_key.as_ref() == Some(key);
                        let header_text = if is_selected {
                            egui::RichText::new(key).color(egui::Color32::YELLOW)
                        } else {
                            egui::RichText::new(key)
                        };

                        let response = ui.collapsing(header_text, |ui| {
                            let config = state.spring_colliders.get_mut(key).unwrap();
                            ui.horizontal(|ui| {
                                if ui.button("⟲").clicked() {
                                    config.radius = config.initial_radius;
                                }
                                ui.add(
                                    egui::Slider::new(&mut config.radius, 0.0..=0.5).text("Radius"),
                                );
                            });
                            ui.horizontal(|ui| {
                                if ui.button("⟲").clicked() {
                                    config.offset_x = config.initial_offset_x;
                                }
                                ui.add(
                                    egui::Slider::new(&mut config.offset_x, -1.0..=1.0)
                                        .text("Offset X"),
                                );
                            });
                            ui.horizontal(|ui| {
                                if ui.button("⟲").clicked() {
                                    config.offset_y = config.initial_offset_y;
                                }
                                ui.add(
                                    egui::Slider::new(&mut config.offset_y, -1.0..=1.0)
                                        .text("Offset Y"),
                                );
                            });
                            ui.horizontal(|ui| {
                                if ui.button("⟲").clicked() {
                                    config.offset_z = config.initial_offset_z;
                                }
                                ui.add(
                                    egui::Slider::new(&mut config.offset_z, -1.0..=1.0)
                                        .text("Offset Z"),
                                );
                            });
                        });

                        if response.header_response.clicked() {
                            state.selected_hull_key = Some(key.clone());
                        }

                        if is_selected && state.scroll_requested {
                            response
                                .header_response
                                .scroll_to_me(Some(egui::Align::Center));
                        }

                        ui.separator();
                    });
                }

                state.scroll_requested = false;
            });

            if any_changed && state.dynamic_rebuild {
                state.trigger_rebuild = true;
            }
        });
    }

    if state.show_node_editor {
        crate::ui::node_editor::draw_node_editor(ctx, graph);
    }
}
