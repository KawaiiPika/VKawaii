use crate::models::vrm_loader::SkinningData;
use blue_engine::{Engine, KeyCode, Signal};
use nalgebra::{UnitQuaternion, Vector4};

pub struct ArmatureTest {
    pub skinning_system: crate::rendering::skinning::SkinningSystem,
    pub skinning_data: Vec<SkinningData>,
    pub spring_bone_system: crate::physics::spring_bones::SpringBoneSystem,
    pub root_nodes: Vec<usize>,
    pub time: f32,

    // Cached bone indices
    pub head_idx: Option<usize>,
    pub chest_idx: Option<usize>,
    pub left_hand_idx: Option<usize>,
    pub right_hand_idx: Option<usize>,

    // Toggles
    pub show_hulls: bool,
    pub show_meshes: bool,
    pub p_pressed: bool,
    pub m_pressed: bool,
    pub mouse_pos: (f32, f32),
}

impl ArmatureTest {
    pub fn new(vrm: &mut crate::models::vrm_loader::VrmModel) -> Self {
        let head_idx = vrm.skinning_system.nodes.iter().position(|n| {
            let name = n.name.as_deref().unwrap_or("").to_lowercase();
            name.contains("head")
        });

        let chest_idx = vrm.skinning_system.nodes.iter().position(|n| {
            let name = n.name.as_deref().unwrap_or("").to_lowercase();
            name.contains("chest") || name.contains("spine") || name.contains("torso")
        });

        let left_hand_idx = vrm.skinning_system.nodes.iter().position(|n| {
            let name = n.name.as_deref().unwrap_or("").to_lowercase();
            name.contains("hand") && name.contains("l") && !name.contains("r")
        });

        let right_hand_idx = vrm.skinning_system.nodes.iter().position(|n| {
            let name = n.name.as_deref().unwrap_or("").to_lowercase();
            name.contains("hand") && name.contains("r") && !name.contains("l")
        });

        println!("--- ARMATURE TEST BONE SEARCH ---");
        for (i, node) in vrm.skinning_system.nodes.iter().enumerate() {
            if let Some(name) = &node.name {
                println!("Bone {}: {}", i, name);
            }
        }
        println!("Head Bone Idx: {:?}", head_idx);
        println!("Chest Bone Idx: {:?}", chest_idx);
        println!("Left Hand Bone Idx: {:?}", left_hand_idx);
        println!("Right Hand Bone Idx: {:?}", right_hand_idx);
        println!("---------------------------------");

        // We steal the skinning data from the VrmModel so we can own it
        let mut skinning_data = Vec::new();
        std::mem::swap(&mut skinning_data, &mut vrm.skinning_data);

        let mut skinning_system = crate::rendering::skinning::SkinningSystem::new();
        std::mem::swap(&mut skinning_system, &mut vrm.skinning_system);

        let mut spring_bone_system = crate::physics::spring_bones::SpringBoneSystem::new();
        std::mem::swap(&mut spring_bone_system, &mut vrm.spring_bone_system);

        let root_nodes = vrm
            .gltf_document
            .scenes()
            .flat_map(|s| s.nodes().map(|n| n.index()))
            .collect();

        Self {
            skinning_system,
            skinning_data,
            spring_bone_system,
            root_nodes,
            time: 0.0,
            head_idx,
            chest_idx,
            left_hand_idx,
            right_hand_idx,
            show_hulls: false,
            show_meshes: true,
            p_pressed: false,
            m_pressed: false,
            mouse_pos: (0.0, 0.0),
        }
    }
}

impl Signal for ArmatureTest {
    fn frame(
        &mut self,
        engine: &mut Engine,
        _encoder: &mut blue_engine::CommandEncoder,
        _view: &blue_engine::TextureView,
    ) {
        let _test_device = &engine.renderer.device;
        let _test_queue = &engine.renderer.queue;
        let mut do_rebuild = false;
        {
            let mut state = crate::ui::overlay::OVERLAY_STATE.lock().unwrap();

            for (idx, col) in self.spring_bone_system.colliders.iter_mut().enumerate() {
                if let Some(key) = state.spring_collider_keys.get(idx) {
                    if let Some(config) = state.spring_colliders.get(key) {
                        let radius_changed = (col.radius - config.radius).abs() > 0.0001;
                        col.radius = config.radius;
                        col.offset.x = config.offset_x;
                        col.offset.y = config.offset_y;
                        col.offset.z = config.offset_z;

                        if radius_changed {
                            let vis_mesh = crate::models::spring_bone_parser::generate_uv_sphere(
                                col.radius.max(0.001),
                                12,
                                12,
                            );
                            self.spring_bone_system
                                .spring_collider_vis_meshes
                                .insert(idx, vis_mesh);
                        }
                    }
                }
            }

            if state.trigger_rebuild {
                state.trigger_rebuild = false;
                do_rebuild = true;
            }
        }

        if do_rebuild {
            for (node_idx, _) in &self.spring_bone_system.body_hull_vis_meshes {
                let name = format!("BodyHullVis_{}", node_idx);
                engine.objects.remove(name.as_str());
            }

            self.spring_bone_system.body_hull_colliders.clear();
            self.spring_bone_system.body_hull_vis_meshes.clear();

            crate::models::spring_bone_parser::build_body_hull_colliders(
                &self.skinning_system.nodes,
                &self.skinning_system.skins,
                &self.skinning_data,
                &mut self.spring_bone_system,
            );

            for (node_idx, (vertices, indices)) in &self.spring_bone_system.body_hull_vis_meshes {
                let object_name = format!("BodyHullVis_{}", node_idx);
                if let Ok(mut object) = blue_engine::Object::new(
                    &object_name,
                    vertices.clone(),
                    indices.clone(),
                    blue_engine::ObjectSettings::default(),
                    &mut engine.renderer,
                ) {
                    let _ = object.set_color(0.0, 0.8, 1.0, 0.7); // Cyan
                    object.is_visible = self.show_hulls;
                    engine.objects.insert(object_name.into(), object);
                }
            }
        }

        self.time += 0.016; // Approx 60fps delta

        // --- 1. Animate Bones ---

        if let Some(idx) = self.head_idx {
            let rot_x = (self.time * 2.0).sin() * 0.2;
            let rot_y = (self.time * 1.5).cos() * 0.3;
            let rotation = UnitQuaternion::from_euler_angles(rot_x, rot_y, 0.0);

            let original = self.skinning_system.nodes[idx].local_transform;
            let translation = original.column(3).into_owned();

            self.skinning_system.nodes[idx].local_transform = rotation.to_homogeneous();
            self.skinning_system.nodes[idx]
                .local_transform
                .set_column(3, &translation);
        }

        if let Some(idx) = self.chest_idx {
            let rot_z = (self.time).sin() * 0.1;
            let rotation = UnitQuaternion::from_euler_angles(0.0, 0.0, rot_z);

            let original = self.skinning_system.nodes[idx].local_transform;
            let translation = original.column(3).into_owned();

            self.skinning_system.nodes[idx].local_transform = rotation.to_homogeneous();
            self.skinning_system.nodes[idx]
                .local_transform
                .set_column(3, &translation);
        }

        if let Some(idx) = self.left_hand_idx {
            let rot_z = (self.time * 3.0).sin() * 0.5;
            let rotation = UnitQuaternion::from_euler_angles(0.0, 0.0, rot_z);

            let original = self.skinning_system.nodes[idx].local_transform;
            let translation = original.column(3).into_owned();

            self.skinning_system.nodes[idx].local_transform = rotation.to_homogeneous();
            self.skinning_system.nodes[idx]
                .local_transform
                .set_column(3, &translation);
        }

        // --- 2. Compute Global Transforms ---
        // Clone root nodes to avoid borrow checker issues
        let roots = self.root_nodes.clone();
        self.skinning_system.update_global_transforms(&roots);

        // --- 3. Physics / Spring Bones ---
        // The spring bones require parent global matrices, and they will update
        // their own local and global matrices based on physics.
        self.spring_bone_system
            .step(0.016, &mut self.skinning_system.nodes);

        // Update global transforms AGAIN because physics modified locals
        self.skinning_system.update_global_transforms(&roots);

        // --- 4. CPU Skinning & GPU Upload ---
        for data in &self.skinning_data {
            let skinned_vertices = self.skinning_system.skin_vertices(
                data.skin_idx,
                &data.original_vertices,
                &data.joints,
                &data.weights,
            );

            // CPU skinning is still required to animate Face/Body, but we no longer
            // feed it into physics mesh_colliders since that was removed.

            crate::rendering::skinning::SkinningSystem::upload_to_gpu(
                engine,
                &data.mesh_name,
                skinned_vertices,
            );
        }

        // --- 5. Toggles & Hull Vis Skinning ---
        let p_down = engine.simple_input.key_held(KeyCode::KeyP);
        if p_down && !self.p_pressed {
            self.show_hulls = !self.show_hulls;
        }
        self.p_pressed = p_down;

        let m_down = engine.simple_input.key_held(KeyCode::KeyM);
        if m_down && !self.m_pressed {
            self.show_meshes = !self.show_meshes;
        }
        self.m_pressed = m_down;

        let mut selected_key = None;

        // Handle 3D Mouse Picking
        if engine
            .simple_input
            .mouse_pressed(blue_engine::MouseButton::Left)
        {
            let mouse_pos = crate::ui::overlay::OVERLAY_STATE.lock().unwrap().mouse_pos;
            let (logical_mouse_x, logical_mouse_y) = mouse_pos;
            if let Some(main_cam) = engine.camera.get("main") {
                let w = main_cam.resolution.x;
                let h = main_cam.resolution.y;

                let scale_factor = engine
                    .window
                    .as_ref()
                    .map(|w| w.scale_factor())
                    .unwrap_or(1.0) as f32;
                let phys_mouse_x = logical_mouse_x * scale_factor;
                let phys_mouse_y = logical_mouse_y * scale_factor;

                if phys_mouse_x < w * 0.75 {
                    // Ignore if clicking on UI area
                    let ndc_x = (2.0 * phys_mouse_x) / w - 1.0;
                    let ndc_y = 1.0 - (2.0 * phys_mouse_y) / h;

                    println!("RAYCAST DEBUG: log_mouse=({}, {}), phys_mouse=({}, {}), w={}, h={}, ndc=({}, {})",
                             logical_mouse_x, logical_mouse_y, phys_mouse_x, phys_mouse_y, w, h, ndc_x, ndc_y);

                    let view_data = main_cam.view_data;
                    let slice: &[f32; 16] = view_data.as_ref();
                    let view_proj_mat = nalgebra::Matrix4::from_column_slice(slice);

                    let mut closest_depth = std::f32::MAX;
                    let mut closest_k = None;

                    let state = crate::ui::overlay::OVERLAY_STATE.lock().unwrap();
                    let show_body_hulls = state.show_body_hulls;
                    let show_spring_bone_hulls = state.show_spring_bone_hulls;
                    let show_spring_colliders = state.show_spring_colliders;
                    drop(state);

                    if show_spring_colliders {
                        for (idx, _) in self.spring_bone_system.spring_collider_vis_meshes.iter() {
                            let col = &self.spring_bone_system.colliders[*idx];
                            let global = self.skinning_system.nodes[col.node_idx].global_transform;
                            let offset_matrix = nalgebra::Matrix4::new_translation(&col.offset);
                            let final_mat = global * offset_matrix;

                            let center = final_mat.column(3); // Vector4
                            let mut ndc_pt = view_proj_mat * center;
                            ndc_pt /= ndc_pt.w;

                            if ndc_pt.z > 0.0 && ndc_pt.z < 1.0 {
                                let screen_x = (ndc_pt.x + 1.0) / 2.0 * w;
                                let screen_y = (1.0 - ndc_pt.y) / 2.0 * h;
                                let dist = ((screen_x - phys_mouse_x).powi(2)
                                    + (screen_y - phys_mouse_y).powi(2))
                                .sqrt();

                                // Since spheres project to circles, we use a larger 2D threshold (40px)
                                if dist < 40.0 && ndc_pt.z < closest_depth {
                                    closest_depth = ndc_pt.z;
                                    let state = crate::ui::overlay::OVERLAY_STATE.lock().unwrap();
                                    if let Some(key) = state.spring_collider_keys.get(*idx) {
                                        closest_k = Some(key.clone());
                                    }
                                }
                            }
                        }
                    }

                    if show_spring_bone_hulls {
                        for (node_idx, (orig_verts, _)) in
                            self.spring_bone_system.hull_vis_meshes.iter()
                        {
                            let global = self.skinning_system.nodes[*node_idx].global_transform;

                            let mut min_dist = std::f32::MAX;
                            let mut min_depth = std::f32::MAX;
                            for v in orig_verts {
                                let local_pos =
                                    Vector4::new(v.position[0], v.position[1], v.position[2], 1.0);
                                let world_pos = global * local_pos;
                                let mut ndc_pt = view_proj_mat * world_pos;
                                ndc_pt /= ndc_pt.w;

                                if ndc_pt.z > 0.0 && ndc_pt.z < 1.0 {
                                    let screen_x = (ndc_pt.x + 1.0) / 2.0 * w;
                                    let screen_y = (1.0 - ndc_pt.y) / 2.0 * h;
                                    let dist = ((screen_x - phys_mouse_x).powi(2)
                                        + (screen_y - phys_mouse_y).powi(2))
                                    .sqrt();
                                    if dist < min_dist {
                                        min_dist = dist;
                                        min_depth = ndc_pt.z;
                                    }
                                }
                            }

                            // Hitbox radius: 30 pixels!
                            if min_dist < 30.0 && min_depth < closest_depth {
                                closest_depth = min_depth;
                                let node_name = self.skinning_system.nodes[*node_idx]
                                    .name
                                    .clone()
                                    .unwrap_or_else(|| format!("Node {}", node_idx));
                                closest_k = Some(node_name);
                            }
                        }
                    }

                    if show_body_hulls {
                        for (node_idx, (orig_verts, _)) in
                            self.spring_bone_system.body_hull_vis_meshes.iter()
                        {
                            let global = self.skinning_system.nodes[*node_idx].global_transform;

                            let mut min_dist = std::f32::MAX;
                            let mut min_depth = std::f32::MAX;
                            for v in orig_verts {
                                let local_pos =
                                    Vector4::new(v.position[0], v.position[1], v.position[2], 1.0);
                                let world_pos = global * local_pos;
                                let mut ndc_pt = view_proj_mat * world_pos;
                                ndc_pt /= ndc_pt.w;

                                if ndc_pt.z > 0.0 && ndc_pt.z < 1.0 {
                                    let screen_x = (ndc_pt.x + 1.0) / 2.0 * w;
                                    let screen_y = (1.0 - ndc_pt.y) / 2.0 * h;
                                    let dist = ((screen_x - phys_mouse_x).powi(2)
                                        + (screen_y - phys_mouse_y).powi(2))
                                    .sqrt();
                                    if dist < min_dist {
                                        min_dist = dist;
                                        min_depth = ndc_pt.z;
                                    }
                                }
                            }

                            // Hitbox radius: 30 pixels!
                            if min_dist < 30.0 && min_depth < closest_depth {
                                closest_depth = min_depth;
                                let node_name = self.skinning_system.nodes[*node_idx]
                                    .name
                                    .clone()
                                    .unwrap_or_else(|| format!("Node {}", node_idx));
                                closest_k = Some(node_name);
                            }
                        }
                    }

                    if let Some(k) = closest_k {
                        let mut state = crate::ui::overlay::OVERLAY_STATE.lock().unwrap();
                        state.selected_hull_key = Some(k);
                        state.scroll_requested = true;
                    }
                }
            }
        }

        {
            let state = crate::ui::overlay::OVERLAY_STATE.lock().unwrap();
            selected_key = state.selected_hull_key.clone();
        }

        let state = crate::ui::overlay::OVERLAY_STATE.lock().unwrap();
        let show_body_hulls = state.show_body_hulls;
        let show_spring_bone_hulls = state.show_spring_bone_hulls;
        let show_spring_colliders = state.show_spring_colliders;
        drop(state);

        for (name, object) in engine.objects.iter_mut() {
            if name.starts_with("HullVis_") {
                object.is_visible = self.show_hulls && show_spring_bone_hulls;
            } else if name.starts_with("BodyHullVis_") {
                object.is_visible = self.show_hulls && show_body_hulls;
            } else if name.starts_with("SpringColVis_") {
                object.is_visible = self.show_hulls && show_spring_colliders;
            } else if !name.contains("Camera") && !name.contains("Light") {
                object.is_visible = self.show_meshes;
            }
        }

        if self.show_hulls {
            for node_idx in self.spring_bone_system.hull_vis_meshes.keys() {
                let name = format!("HullVis_{}", node_idx);
                let node_name = self.skinning_system.nodes[*node_idx]
                    .name
                    .clone()
                    .unwrap_or_else(|| format!("Node {}", node_idx));
                let is_selected = selected_key.as_ref() == Some(&node_name);

                if let Some(object) = engine.objects.get_mut(name.as_str()) {
                    if is_selected {
                        let _ = object.set_color(1.0, 1.0, 0.0, 1.0); // Bright Yellow
                    } else {
                        let _ = object.set_color(0.0, 1.0, 0.0, 0.8); // Default Green
                    }

                    let global = self.skinning_system.nodes[*node_idx].global_transform;
                    let slice: &[f32; 16] = global.as_slice().try_into().unwrap();
                    let glam_global = blue_engine::Matrix4::from_cols_array(slice);

                    object.translation_matrix = glam_global;
                    object.rotation_quaternion = blue_engine::Quaternion::IDENTITY;
                    object.scale_matrix = blue_engine::Matrix4::IDENTITY;

                    // PERFORMANCE FIX: blue_engine recreates the entire vertex buffer if changed == true!
                    // We bypass this by manually updating ONLY the uniform buffer and unflagging it.
                    object.flag_as_changed(false);
                    object.update_uniform_buffer(&mut engine.renderer);
                }
            }

            for (node_idx, _) in &self.spring_bone_system.body_hull_vis_meshes {
                let name = format!("BodyHullVis_{}", node_idx);
                let node_name = self.skinning_system.nodes[*node_idx]
                    .name
                    .clone()
                    .unwrap_or_else(|| format!("Node {}", node_idx));
                let is_selected = selected_key.as_ref() == Some(&node_name);

                if let Some(object) = engine.objects.get_mut(name.as_str()) {
                    if is_selected {
                        let _ = object.set_color(1.0, 1.0, 0.0, 1.0); // Bright Yellow
                    } else {
                        let _ = object.set_color(0.0, 0.8, 1.0, 0.7); // Default Cyan
                    }

                    let global = self.skinning_system.nodes[*node_idx].global_transform;
                    let slice: &[f32; 16] = global.as_slice().try_into().unwrap();
                    let glam_global = blue_engine::Matrix4::from_cols_array(slice);

                    object.translation_matrix = glam_global;
                    object.rotation_quaternion = blue_engine::Quaternion::IDENTITY;
                    object.scale_matrix = blue_engine::Matrix4::IDENTITY;

                    object.flag_as_changed(false);
                    object.update_uniform_buffer(&mut engine.renderer);
                }
            }

            for idx in self.spring_bone_system.spring_collider_vis_meshes.keys() {
                let name = format!("SpringColVis_{}", idx);
                let mut is_selected = false;
                {
                    let state = crate::ui::overlay::OVERLAY_STATE.lock().unwrap();
                    if let Some(key) = state.spring_collider_keys.get(*idx) {
                        is_selected = selected_key.as_ref() == Some(key);
                    }
                }

                if let Some(object) = engine.objects.get_mut(name.as_str()) {
                    if is_selected {
                        let _ = object.set_color(1.0, 1.0, 0.0, 1.0); // Bright Yellow
                    } else {
                        let _ = object.set_color(1.0, 0.5, 0.0, 0.7); // Default Orange
                    }

                    let col = &self.spring_bone_system.colliders[*idx];
                    let global = self.skinning_system.nodes[col.node_idx].global_transform;

                    let offset_matrix = nalgebra::Matrix4::new_translation(&col.offset);
                    let final_mat = global * offset_matrix;
                    let slice: &[f32; 16] = final_mat.as_slice().try_into().unwrap();
                    let glam_global = blue_engine::Matrix4::from_cols_array(slice);

                    object.translation_matrix = glam_global;
                    object.rotation_quaternion = blue_engine::Quaternion::IDENTITY;
                    object.scale_matrix = blue_engine::Matrix4::IDENTITY;

                    object.flag_as_changed(false);
                    object.update_uniform_buffer(&mut engine.renderer);
                }
            }
        }
    }
}
