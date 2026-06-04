use crate::physics::spring_bones::{SpringBoneSystem, SpringCollider, SpringParticle};
use nalgebra::{Vector3, Vector4};
use serde_json::Value;

pub fn generate_uv_sphere(
    radius: f32,
    latitudes: usize,
    longitudes: usize,
) -> (Vec<blue_engine::Vertex>, Vec<u16>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    for i in 0..=latitudes {
        let lat = std::f32::consts::PI * i as f32 / latitudes as f32;
        let sin_lat = lat.sin();
        let cos_lat = lat.cos();

        for j in 0..=longitudes {
            let lon = 2.0 * std::f32::consts::PI * j as f32 / longitudes as f32;
            let sin_lon = lon.sin();
            let cos_lon = lon.cos();

            let x = sin_lat * cos_lon;
            let y = cos_lat;
            let z = sin_lat * sin_lon;

            vertices.push(blue_engine::Vertex {
                position: [x * radius, y * radius, z * radius],
                uv: [j as f32 / longitudes as f32, i as f32 / latitudes as f32],
                normal: [x, y, z],
            });
        }
    }

    for i in 0..latitudes {
        for j in 0..longitudes {
            let first = (i * (longitudes + 1) + j) as u16;
            let second = first + longitudes as u16 + 1;

            indices.push(first);
            indices.push(second);
            indices.push(first + 1);

            indices.push(second);
            indices.push(second + 1);
            indices.push(first + 1);
        }
    }

    (vertices, indices)
}
pub fn parse_vrm0_spring_bones(
    vrm_json: &Value,
    nodes: &[crate::rendering::skinning::Node],
    skins: &[crate::rendering::skinning::Skin],
    skinning_data: &[crate::models::vrm_loader::SkinningData],
    system: &mut SpringBoneSystem,
) {
    let secondary = match vrm_json.get("secondaryAnimation") {
        Some(s) => s,
        None => return,
    };

    // In VRM 0.0, Collidergroups is an array of objects: { node, colliders: [ { offset: {x,y,z}, radius } ] }
    let mut collider_group_indices = Vec::new(); // maps group_idx -> array of collider indices in system

    if let Some(c_groups) = secondary.get("colliderGroups").and_then(|v| v.as_array()) {
        for (group_idx, group) in c_groups.iter().enumerate() {
            let mut group_indices = Vec::new();
            if let Some(node_idx) = group
                .get("node")
                .and_then(|v| v.as_u64())
                .map(|v| v as usize)
            {
                if let Some(colliders) = group.get("colliders").and_then(|v| v.as_array()) {
                    for (col_idx, col) in colliders.iter().enumerate() {
                        let radius =
                            col.get("radius").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                        let offset = col.get("offset").and_then(|v| v.as_object());

                        let offset_vec = if let Some(o) = offset {
                            let x = o.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                            let y = o.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                            let z = o.get("z").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                            Vector3::new(x, y, z)
                        } else {
                            Vector3::zeros()
                        };

                        let idx = system.colliders.len();
                        system.colliders.push(SpringCollider {
                            node_idx,
                            offset: Vector3::new(offset_vec.x, offset_vec.y, offset_vec.z),
                            radius,
                            tail: None, // VRM 0.0 doesn't specify capsule Colliders by Default unless through an Extension, so sticking to Spheres for now
                        });
                        group_indices.push(idx);

                        let node_name = nodes
                            .get(node_idx)
                            .and_then(|n| n.name.clone())
                            .unwrap_or_else(|| format!("Node {}", node_idx));
                        let key = format!("{} (Group {}, Col {})", node_name, group_idx, col_idx);
                        {
                            let mut state = crate::ui::overlay::OVERLAY_STATE.lock().unwrap();
                            state.spring_collider_keys.push(key.clone());
                            state.spring_colliders.insert(
                                key,
                                crate::ui::overlay::SpringColliderConfig {
                                    radius,
                                    offset_x: offset_vec.x,
                                    offset_y: offset_vec.y,
                                    offset_z: offset_vec.z,
                                    initial_radius: radius,
                                    initial_offset_x: offset_vec.x,
                                    initial_offset_y: offset_vec.y,
                                    initial_offset_z: offset_vec.z,
                                },
                            );
                        }

                        let vis_mesh = generate_uv_sphere(radius.max(0.001), 12, 12);
                        system.spring_collider_vis_meshes.insert(idx, vis_mesh);
                    }
                }
            }
            collider_group_indices.push(group_indices);
        }
    }

    if let Some(bone_groups) = secondary.get("boneGroups").and_then(|v| v.as_array()) {
        for group in bone_groups.iter() {
            let stiffness = group
                .get("stiffiness")
                .and_then(|v| v.as_f64())
                .unwrap_or(1.0) as f32;
            let gravity_power = group
                .get("gravityPower")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0) as f32;
            let drag_force = (group
                .get("dragForce")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.4) as f32
                + 0.15)
                .min(0.95);
            let hit_radius = group
                .get("hitRadius")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.02) as f32;

            let gravity_dir = if let Some(g) = group.get("gravityDir").and_then(|v| v.as_object()) {
                let x = g.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                let y = g.get("y").and_then(|v| v.as_f64()).unwrap_or(-1.0) as f32;
                let z = g.get("z").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                Vector3::new(x, y, z)
            } else {
                Vector3::new(0.0, -1.0, 0.0)
            };

            let mut active_colliders = Vec::new();
            if let Some(cg) = group.get("colliderGroups").and_then(|v| v.as_array()) {
                for c in cg {
                    if let Some(c_idx) = c.as_u64().map(|v| v as usize) {
                        if c_idx < collider_group_indices.len() {
                            active_colliders.extend_from_slice(&collider_group_indices[c_idx]);
                        }
                    }
                }
            }

            if let Some(bones) = group.get("bones").and_then(|v| v.as_array()) {
                for bone_val in bones {
                    if let Some(root_node_idx) = bone_val.as_u64().map(|v| v as usize) {
                        let params = SpringBoneRecursiveParams {
                            stiffness,
                            gravity_power,
                            gravity_dir,
                            drag_force,
                            hit_radius,
                            colliders: &active_colliders,
                        };
                        add_spring_bone_recursive(
                            root_node_idx,
                            root_node_idx, // Use root_node_idx as a unique chain ID instead of VRM group_id
                            &params,
                            nodes,
                            skins,
                            skinning_data,
                            system,
                        );
                    }
                }
            }
        }
    }
}

struct SpringBoneRecursiveParams<'a> {
    stiffness: f32,
    gravity_power: f32,
    gravity_dir: Vector3<f32>,
    drag_force: f32,
    hit_radius: f32,
    colliders: &'a [usize],
}

fn add_spring_bone_recursive(
    node_idx: usize,
    group_id: usize,
    params: &SpringBoneRecursiveParams,
    nodes: &[crate::rendering::skinning::Node],
    skins: &[crate::rendering::skinning::Skin],
    skinning_data: &[crate::models::vrm_loader::SkinningData],
    system: &mut SpringBoneSystem,
) {
    let node = &nodes[node_idx];

    let local_tail = if !node.children.is_empty() {
        let child = &nodes[node.children[0]];
        child.local_transform.column(3).xyz()
    } else {
        // Pseudo tail: 1mm in the Direction of the bone's local -Y
        Vector3::new(0.0, -0.001, 0.0) // VRM is +Y up, bones point Down? Or +Y? Wait and See.
    };

    let initial_local_matrix = node.local_transform;
    let initial_local_tail = Vector4::new(local_tail.x, local_tail.y, local_tail.z, 1.0);
    let bone_length = local_tail.norm();

    let current_tail = (node.global_transform * initial_local_tail).xyz();

    let parent_node_idx = nodes
        .iter()
        .position(|n| n.children.contains(&node_idx))
        .unwrap_or(node_idx);

    // Excluding by Default for busts/skirts, but leaving it toggleable
    let default_exclude = if let Some(ref name) = node.name {
        let n = name.to_lowercase();
        n.contains("bust")
            || n.contains("skirt")
            || n.contains("sleeve")
            || n.contains("breast")
            || n.contains("chest")
    } else {
        false
    };

    let node_name = node
        .name
        .clone()
        .unwrap_or_else(|| format!("Node {}", node_idx));

    let (exclude_from_mesh_collision, hull_config) = {
        let mut state = crate::ui::overlay::OVERLAY_STATE.lock().unwrap();
        if !state.hair_hull_toggles.contains_key(&node_name) {
            state
                .hair_hull_toggles
                .insert(node_name.clone(), !default_exclude);
            state
                .hair_hull_configs
                .insert(node_name.clone(), crate::ui::overlay::HullConfig::default());
        }

        let enabled = *state.hair_hull_toggles.get(&node_name).unwrap();
        let config = state.hair_hull_configs.get(&node_name).unwrap().clone();
        (!enabled, config)
    };

    // Using a Capsule for the bone physics Hull instead of Generating a massive blob from all vertices
    if !exclude_from_mesh_collision {
        let mut hull_points = Vec::new();

        let p_start = blue_engine::glam::Vec3::new(0.0, 0.0, 0.0);
        let p_end = blue_engine::glam::Vec3::new(local_tail.x, local_tail.y, local_tail.z);
        let ab = p_end - p_start;
        let ab_len_sq = ab.length_squared();

        let m3 = node.global_transform.fixed_view::<3, 3>(0, 0);
        let global_scale = m3.column(0).norm().max(0.0001);
        let local_hit_radius = params.hit_radius / global_scale;

        for data in skinning_data {
            let skin = &skins[data.skin_idx];
            if let Some(joint_idx) = skin.joints.iter().position(|&j| j == node_idx) {
                let j_idx = joint_idx as u16;
                let inv_bind = skin.inverse_bind_matrices[joint_idx];

                for (i, joints) in data.joints.iter().enumerate() {
                    let mut weight = 0.0;
                    for (j, &joint) in joints.iter().enumerate().take(4) {
                        if joint == j_idx {
                            weight += data.weights[i][j];
                        }
                    }
                    if weight >= 0.49 {
                        let v = data.original_vertices[i].position;
                        let world_v = Vector4::new(v[0], v[1], v[2], 1.0);
                        let local_v = inv_bind * world_v;

                        let mut pt = blue_engine::glam::Vec3::new(local_v.x, local_v.y, local_v.z);
                        pt.x *= hull_config.x_squash;
                        pt.y *= hull_config.y_squash;
                        pt.z *= hull_config.z_squash;

                        // Strict Distance filter to prevent large overlapping blobs
                        let t = if ab_len_sq > 0.000001 {
                            ((pt - p_start).dot(ab) / ab_len_sq).clamp(0.0, 1.0)
                        } else {
                            0.0
                        };
                        let closest = p_start + ab * t;

                        let dir = pt - closest;
                        pt = closest + dir * hull_config.shrink_factor;

                        let dist = pt.distance(closest);

                        // Tossing out Points that project near the bone Root (t < 0.30) to Stop
                        // The hull base From digging into the scalp/head and Pushing the
                        // Hair straight out.
                        // Going with a Slightly expanded radius (local_hit_radius + 0.005 / scale) to Close
                        // Inter-bone gaps, but Bounded to dodge huge blobs on Small hairs.
                        if dist
                            <= (local_hit_radius + 0.005 / global_scale).min(0.02 / global_scale)
                            && t > 0.30
                        {
                            hull_points.push(pt);
                        }
                    }
                }
            }
        }

        // If not enough Points are found from skinning, Generate synthetic points along the bone
        // So it still gets a convex hull That respects the squash/shrink UI settings
        if hull_points.len() < 4 {
            let cap_end = if ab_len_sq > 0.000001 {
                p_end
            } else {
                blue_engine::glam::Vec3::new(0.0, -0.001 / global_scale, 0.0)
            };
            let fallback_r = if ab_len_sq > 0.000001 {
                (ab_len_sq.sqrt() * 0.1).min(0.003 / global_scale)
            } else {
                0.003 / global_scale
            };
            // Going with a tiny radius (max 5mm) to Stop massive blocky Geometry at the ends of Hairs.
            let radius = local_hit_radius.min(0.005 / global_scale).max(fallback_r);
            let dir = if ab_len_sq > 0.000001 {
                ab.normalize()
            } else {
                blue_engine::glam::Vec3::new(0.0, -1.0, 0.0)
            };
            let up = if dir.y.abs() < 0.9 {
                blue_engine::glam::Vec3::Y
            } else {
                blue_engine::glam::Vec3::Z
            };
            let right = dir.cross(up).normalize();
            let forward = right.cross(dir).normalize();

            let mut fallback_pts = vec![p_start - dir * radius, cap_end + dir * radius];

            for i in 0..8 {
                let angle = (i as f32) * std::f32::consts::PI * 2.0 / 8.0;
                let offset = right * angle.cos() * radius + forward * angle.sin() * radius;
                fallback_pts.push(p_start + offset);
                fallback_pts.push(cap_end + offset);
            }

            for mut pt in fallback_pts {
                pt.x *= hull_config.x_squash;
                pt.y *= hull_config.y_squash;
                pt.z *= hull_config.z_squash;

                let t = if ab_len_sq > 0.000001 {
                    ((pt - p_start).dot(ab) / ab_len_sq).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                let closest = p_start + ab * t;
                let d = pt - closest;
                pt = closest + d * hull_config.shrink_factor;

                hull_points.push(pt);
            }
        }

        if hull_points.len() >= 4 {
            // Minimum points for a 3D convex hull
            if let Some(shape) = rapier3d::prelude::SharedShape::convex_hull(&hull_points) {
                println!(
                    "Generated Physics Hull for Node {} with {} points!",
                    node_idx,
                    hull_points.len()
                );

                if let Some(poly) = shape.as_convex_polyhedron() {
                    let mut vis_verts = Vec::new();
                    let mut vis_indices = Vec::new();

                    let (vertices, indices) = poly.to_trimesh();

                    for pt in vertices {
                        vis_verts.push(blue_engine::Vertex {
                            position: [pt.x, pt.y, pt.z],
                            uv: [0.0, 0.0],
                            normal: [0.0, 1.0, 0.0], // Flat shading
                        });
                    }

                    for idx in indices {
                        vis_indices.push(idx[0] as u16);
                        vis_indices.push(idx[1] as u16);
                        vis_indices.push(idx[2] as u16);
                    }

                    system
                        .hull_vis_meshes
                        .insert(node_idx, (vis_verts, vis_indices));
                }

                system.hull_colliders.insert(node_idx, shape);
            } else {
                println!("Convex hull failed for Node {}.", node_idx);
            }
        } else {
            println!(
                "Not enough points ({}) for hull on Node {}.",
                hull_points.len(),
                node_idx
            );
        }
    }

    system.particles.push(SpringParticle {
        node_idx,
        parent_node_idx,
        group_id,
        exclude_from_mesh_collision,
        current_tail,
        prev_tail: current_tail,
        initial_local_matrix,
        initial_local_tail,
        hit_radius: params.hit_radius,
        stiffness: params.stiffness,
        gravity_power: params.gravity_power,
        gravity_dir: params.gravity_dir,
        drag_force: params.drag_force,
        bone_length,
        max_angle: std::f32::consts::PI / 4.0, // Default to 45 degree angular limit
        colliders: params.colliders.to_vec(),
    });

    for &child_idx in &node.children {
        add_spring_bone_recursive(
            child_idx,
            group_id,
            params,
            nodes,
            skins,
            skinning_data,
            system,
        );
    }
}

/// Building convex hull Colliders for the body bones (head, neck, chest, arms, hands, etc.)
/// Storing these in `system.body_hull_colliders` and using them in Pass 1.5 so the spring-bone Hair
/// Clashes with the actual body Mesh geometry instead of just the VRM metadata.
pub fn build_body_hull_colliders(
    nodes: &[crate::rendering::skinning::Node],
    skins: &[crate::rendering::skinning::Skin],
    skinning_data: &[crate::models::vrm_loader::SkinningData],
    system: &mut SpringBoneSystem,
) {
    // Bone names to Generate body Hulls for.
    // These are the Bones the Hair tends to Clip through.
    let target_name_fragments: &[&str] = &[
        "head",
        "neck",
        "chest",
        "spine",
        "shoulder",
        "arm", // catches upperarm, lowerarm, forearm
        "hand",
        "hair",
    ];

    for (node_idx, node) in nodes.iter().enumerate() {
        let name = match &node.name {
            Some(n) => n.as_str(),
            None => continue,
        };
        
        let name_lower = name.to_lowercase();

        if !target_name_fragments
            .iter()
            .any(|&frag| name_lower.contains(frag))
        {
            continue;
        }

        let mut hull_points: Vec<blue_engine::glam::Vec3> = Vec::new();
        let is_head = name_lower.contains("head");

        for data in skinning_data {
            let skin = &skins[data.skin_idx];
            if let Some(joint_idx) = skin.joints.iter().position(|&j| j == node_idx) {
                let j_idx = joint_idx as u16;
                let inv_bind = skin.inverse_bind_matrices[joint_idx];

                for (i, joints) in data.joints.iter().enumerate() {
                    let mut weight = 0.0f32;
                    for (j, &joint) in joints.iter().enumerate().take(4) {
                        if joint == j_idx {
                            weight += data.weights[i][j];
                        }
                    }
                    if weight > 0.55 {
                        let v = data.original_vertices[i].position;
                        let world_v = Vector4::new(v[0], v[1], v[2], 1.0);
                        let local_v = inv_bind * world_v;
                        hull_points.push(blue_engine::glam::Vec3::new(
                            local_v.x, local_v.y, local_v.z,
                        ));
                    }
                }
            }
        }

        // For the head bone there are 425k+ points - uniformly subsample to 2000 so the
        // Convex hull covers the FULL head shape (face + scalp + top) without crashing.
        if is_head && hull_points.len() > 2000 {
            let stride = hull_points.len() / 2000;
            hull_points = hull_points.into_iter().step_by(stride.max(1)).collect();
        }

        if hull_points.len() < 4 {
            println!(
                "Body hull: not enough points ({}) for bone '{}' ({}), skipping.",
                hull_points.len(),
                name,
                node_idx
            );
            continue;
        }

        let mut config = crate::ui::overlay::HullConfig::default();
        {
            let mut state = crate::ui::overlay::OVERLAY_STATE.lock().unwrap();
            if let Some(cfg) = state.hull_configs.get(name) {
                config = cfg.clone();
            } else {
                state.hull_configs.insert(name.to_string(), config.clone());
            }
        }

        // Shrinking hull points Toward their centroid so the collision Surface sits
        // Slightly inside the Mesh, leaving Hair room to Rest on top Without being pushed out.
        let centroid = hull_points
            .iter()
            .fold(blue_engine::glam::Vec3::ZERO, |a, &b| a + b)
            / hull_points.len() as f32;
        for pt in &mut hull_points {
            let mut diff = *pt - centroid;
            diff.x *= config.x_squash;
            diff.y *= config.y_squash;
            diff.z *= config.z_squash;
            *pt = centroid + diff * config.shrink_factor;
        }

        if hull_points.len() < 4 {
            println!(
                "Body hull: not enough points ({}) for bone '{}' ({}), skipping.",
                hull_points.len(),
                name,
                node_idx
            );
            continue;
        }

        // Special case: the head bone influences Enormous numbers of vertices.
        // Sampled up to 2000 points and shrank them — now Build a Normal Convex hull from that.
        match rapier3d::prelude::SharedShape::convex_hull(&hull_points) {
            Some(shape) => {
                println!(
                    "Body hull generated for '{}' ({}) with {} points.",
                    name,
                    node_idx,
                    hull_points.len()
                );

                if let Some(poly) = shape.as_convex_polyhedron() {
                    let (vertices, indices) = poly.to_trimesh();
                    let vis_verts: Vec<blue_engine::Vertex> = vertices
                        .iter()
                        .map(|pt| blue_engine::Vertex {
                            position: [pt.x, pt.y, pt.z],
                            uv: [0.0, 0.0],
                            normal: [0.0, 1.0, 0.0],
                        })
                        .collect();
                    let vis_indices: Vec<u16> = indices
                        .iter()
                        .flat_map(|tri| [tri[0] as u16, tri[1] as u16, tri[2] as u16])
                        .collect();
                    system
                        .body_hull_vis_meshes
                        .push((node_idx, (vis_verts, vis_indices)));
                }

                system.body_hull_colliders.push((node_idx, shape));
            }
            None => {
                println!(
                    "Body hull: convex_hull() failed for bone '{}' ({}).",
                    name, node_idx
                );
            }
        }
    }
    println!(
        "Built {} body hull colliders.",
        system.body_hull_colliders.len()
    );
}
