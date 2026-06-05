use crate::scripting::node_graph::{Connection, NodeGraph};
use blue_engine_utilities::egui_plugin::egui;
use uuid::Uuid;

pub fn draw_node_editor(ctx: &egui::Context, graph: &mut NodeGraph) {
    let mut state = crate::ui::overlay::OVERLAY_STATE.lock().unwrap();
    state.pin_positions.clear();

    for node in &mut graph.nodes {
        let node_position = egui::pos2(node.position[0], node.position[1]);

        let response = egui::Window::new(&node.type_name)
            .id(egui::Id::new(node.id))
            .default_pos(node_position)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    // Left column: Inputs
                    ui.vertical(|ui| {
                        for input in &node.inputs {
                            ui.horizontal(|ui| {
                                let (rect, pin_res) = ui.allocate_at_least(egui::vec2(10.0, 10.0), egui::Sense::click_and_drag());
                                ui.painter().circle_filled(rect.center(), 5.0, egui::Color32::LIGHT_GRAY);
                                state.pin_positions.insert(input.id, rect);

                                if pin_res.drag_started() {
                                    let mut found_conn_idx = None;
                                    for (idx, conn) in graph.connections.iter().enumerate() {
                                        if conn.to_pin == input.id {
                                            found_conn_idx = Some(idx);
                                            break;
                                        }
                                    }
                                    if let Some(idx) = found_conn_idx {
                                        let conn = graph.connections.remove(idx);
                                        state.drag_start_pin = Some(conn.from_pin);
                                    }
                                }

                                ui.label(&input.name);
                            });
                        }
                    });

                    ui.add_space(20.0);

                    // Right column: Outputs
                    ui.vertical(|ui| {
                        for output in &node.outputs {
                            ui.horizontal(|ui| {
                                ui.label(&output.name);
                                let (rect, pin_res) = ui.allocate_at_least(egui::vec2(10.0, 10.0), egui::Sense::click_and_drag());
                                ui.painter().circle_filled(rect.center(), 5.0, egui::Color32::LIGHT_GRAY);
                                state.pin_positions.insert(output.id, rect);

                                if pin_res.drag_started() {
                                    state.drag_start_pin = Some(output.id);
                                }
                            });
                        }
                    });
                });
            });

        if let Some(inner_response) = response {
            let current_pos = inner_response.response.rect.min;
            node.position = [current_pos.x, current_pos.y];
        }
    }

    let painter = ctx.layer_painter(egui::LayerId::background());

    // Drawing the Active Drag
    if let Some(start_pin_id) = state.drag_start_pin {
        let mouse_pos = ctx.input(|i| i.pointer.latest_pos());
        if let Some(start_rect) = state.pin_positions.get(&start_pin_id) {
            let p1 = start_rect.center();
            let p4 = mouse_pos.unwrap_or(p1);

            let dx = (p4.x - p1.x).abs() / 2.0;
            let p2 = p1 + egui::vec2(dx, 0.0);
            let p3 = p4 - egui::vec2(dx, 0.0);

            let bezier = egui::epaint::CubicBezierShape::from_points_stroke(
                [p1, p2, p3, p4],
                false,
                egui::Color32::TRANSPARENT,
                egui::Stroke::new(2.0, egui::Color32::YELLOW),
            );
            painter.add(bezier);
        }

        if ctx.input(|i| i.pointer.any_released()) {
            if let Some(pos) = mouse_pos {
                // Checking if dropped on an input Pin
                let mut target_pin = None;
                for node in &graph.nodes {
                    for input in &node.inputs {
                        if let Some(rect) = state.pin_positions.get(&input.id) {
                            if rect.contains(pos) {
                                target_pin = Some(input.id);
                                break;
                            }
                        }
                    }
                    if target_pin.is_some() {
                        break;
                    }
                }

                if let Some(to_pin) = target_pin {
                    // Checking if Already connected, Removing old if so
                    graph.connections.retain(|c| c.to_pin != to_pin);

                    graph.connections.push(Connection {
                        id: Uuid::new_v4(),
                        from_pin: start_pin_id,
                        to_pin,
                    });
                }
            }
            state.drag_start_pin = None;
        }
    }

    // Drawing Connections
    for conn in &graph.connections {
        if let (Some(from_rect), Some(to_rect)) = (state.pin_positions.get(&conn.from_pin), state.pin_positions.get(&conn.to_pin)) {
            let p1 = from_rect.center();
            let p4 = to_rect.center();

            let dx = (p4.x - p1.x).abs() / 2.0;
            let p2 = p1 + egui::vec2(dx, 0.0);
            let p3 = p4 - egui::vec2(dx, 0.0);

            let bezier = egui::epaint::CubicBezierShape::from_points_stroke(
                [p1, p2, p3, p4],
                false,
                egui::Color32::TRANSPARENT,
                egui::Stroke::new(2.0, egui::Color32::LIGHT_BLUE),
            );
            painter.add(bezier);
        }
    }
}
