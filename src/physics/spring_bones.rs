use nalgebra::{Matrix4, UnitQuaternion, Vector3, Vector4};
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct SpringBoneColliderShapeSphere {
    pub offset: [f32; 3],
    pub radius: f32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SpringBoneColliderShapeCapsule {
    pub offset: [f32; 3],
    pub radius: f32,
    pub tail: [f32; 3],
}

#[derive(Debug, Deserialize, Clone)]
pub struct SpringBoneColliderShape {
    pub sphere: Option<SpringBoneColliderShapeSphere>,
    pub capsule: Option<SpringBoneColliderShapeCapsule>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SpringBoneCollider {
    pub node: Option<usize>,
    pub shape: SpringBoneColliderShape,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SpringBoneColliderGroup {
    pub name: Option<String>,
    pub colliders: Vec<SpringBoneCollider>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SpringBoneJoint {
    pub node: usize,
    #[serde(rename = "hitRadius")]
    pub hit_radius: Option<f32>,
    pub stiffness: Option<f32>,
    pub gravity_power: Option<f32>,
    pub gravity_dir: Option<[f32; 3]>,
    pub drag_force: Option<f32>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SpringBoneSpring {
    pub name: Option<String>,
    pub joints: Vec<SpringBoneJoint>,
    #[serde(rename = "colliderGroups")]
    pub collider_groups: Option<Vec<usize>>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct VrmcSpringBone {
    #[serde(rename = "colliderGroups")]
    pub collider_groups: Option<Vec<SpringBoneColliderGroup>>,
    pub springs: Option<Vec<SpringBoneSpring>>,
}

#[derive(Debug, Clone)]
pub struct SpringCollider {
    pub node_idx: usize,
    pub offset: Vector3<f32>,
    pub radius: f32,
    pub tail: Option<Vector3<f32>>,
}

pub struct SpringParticle {
    pub node_idx: usize,
    pub parent_node_idx: usize,
    pub group_id: usize,
    pub exclude_from_mesh_collision: bool,

    pub current_tail: Vector3<f32>,
    pub prev_tail: Vector3<f32>,

    pub initial_local_matrix: Matrix4<f32>,
    pub initial_local_tail: Vector4<f32>, // Local position of the tail (w=1)

    pub hit_radius: f32,
    pub stiffness: f32,
    pub gravity_power: f32,
    pub gravity_dir: Vector3<f32>,
    pub drag_force: f32,
    pub bone_length: f32,
    pub max_angle: f32,

    pub colliders: Vec<usize>,
}

pub struct SpringBoneSystem {
    pub particles: Vec<SpringParticle>,
    pub colliders: Vec<SpringCollider>,

    pub enable_self_collision: bool,
    pub self_collision_iterations: usize,

    // We will use Rapier purely for NarrowPhase Collision Shapes!
    /// Hair bone local-space convex hulls (keyed by node_idx)
    pub hull_colliders: std::collections::HashMap<usize, rapier3d::prelude::SharedShape>,
    pub hull_vis_meshes: std::collections::HashMap<usize, (Vec<blue_engine::Vertex>, Vec<u16>)>,

    /// Body-mesh bone convex hulls (head, arms, hands…) used as collision surfaces for spring hair.
    /// Each entry is (node_idx, local-space SharedShape).
    pub body_hull_colliders: Vec<(usize, rapier3d::prelude::SharedShape)>,
    pub body_hull_vis_meshes: Vec<(usize, (Vec<blue_engine::Vertex>, Vec<u16>))>,

    /// Visual meshes for VRM spring colliders (spheres/capsules)
    pub spring_collider_vis_meshes:
        std::collections::HashMap<usize, (Vec<blue_engine::Vertex>, Vec<u16>)>,
}

impl Default for SpringBoneSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl SpringBoneSystem {
    pub fn new() -> Self {
        Self {
            particles: Vec::new(),
            colliders: Vec::new(),
            enable_self_collision: false,
            self_collision_iterations: 4,
            hull_colliders: std::collections::HashMap::new(),
            hull_vis_meshes: std::collections::HashMap::new(),
            body_hull_colliders: Vec::new(),
            body_hull_vis_meshes: Vec::new(),
            spring_collider_vis_meshes: std::collections::HashMap::new(),
        }
    }

    pub fn step(&mut self, dt: f32, nodes: &mut [crate::rendering::skinning::Node]) {
        for particle in &mut self.particles {
            let parent_world = nodes[particle.parent_node_idx].global_transform;

            let bone_world_origin_matrix = parent_world * particle.initial_local_matrix;
            let bone_origin = bone_world_origin_matrix.column(3).xyz();
            let stiffness_target = (bone_world_origin_matrix * particle.initial_local_tail).xyz();

            let inertia =
                (particle.current_tail - particle.prev_tail) * (1.0 - particle.drag_force);
            let stiffness_force =
                (stiffness_target - particle.current_tail) * particle.stiffness * dt;
            let gravity = particle.gravity_dir * particle.gravity_power * dt * dt;

            let mut next_tail = particle.current_tail + inertia + stiffness_force + gravity;

            for col_idx in &particle.colliders {
                let col = &self.colliders[*col_idx];
                let col_world = nodes[col.node_idx].global_transform;
                let col_pos =
                    (col_world * Vector4::new(col.offset.x, col.offset.y, col.offset.z, 1.0)).xyz();

                if let Some(tail) = col.tail {
                    let col_tail = (col_world * Vector4::new(tail.x, tail.y, tail.z, 1.0)).xyz();

                    let ab = col_tail - col_pos;
                    let t = ((next_tail - col_pos).dot(&ab) / ab.norm_squared()).clamp(0.0, 1.0);
                    let closest_pt = col_pos + ab * t;

                    let dist = next_tail.metric_distance(&closest_pt);
                    if dist < col.radius + particle.hit_radius {
                        let mut dir = next_tail - closest_pt;
                        if dir.norm_squared() < 0.000001 {
                            dir = Vector3::new(0.0, 1.0, 0.0);
                        } else {
                            dir = dir.normalize();
                        }
                        next_tail = closest_pt + dir * (col.radius + particle.hit_radius);
                    }
                } else {
                    let dist = next_tail.metric_distance(&col_pos);
                    if dist < col.radius + particle.hit_radius {
                        let mut dir = next_tail - col_pos;
                        if dir.norm_squared() < 0.000001 {
                            dir = Vector3::new(0.0, 1.0, 0.0);
                        } else {
                            dir = dir.normalize();
                        }
                        next_tail = col_pos + dir * (col.radius + particle.hit_radius);
                    }
                }
            }

            let tail_dir = (next_tail - bone_origin).normalize();
            next_tail = bone_origin + tail_dir * particle.bone_length;

            // Update particle state (temporarily store next_tail in current_tail for self-collision pass)
            particle.prev_tail = particle.current_tail;
            particle.current_tail = next_tail;
        }

        for i in 0..self.particles.len() {
            if self.particles[i].exclude_from_mesh_collision {
                continue;
            }
            if let Some(shape1) = self
                .hull_colliders
                .get(&self.particles[i].node_idx)
                .cloned()
            {
                let p = &mut self.particles[i];
                let parent_world = nodes[p.parent_node_idx].global_transform;
                let bone_world_origin_matrix = parent_world * p.initial_local_matrix;
                let bone_origin = bone_world_origin_matrix.column(3).xyz();
                let stiffness_target = (bone_world_origin_matrix * p.initial_local_tail).xyz();

                let tail_dir = (p.current_tail - bone_origin).normalize();
                let initial_tail_dir = (stiffness_target - bone_origin).normalize();
                let rot = UnitQuaternion::rotation_between(&initial_tail_dir, &tail_dir)
                    .unwrap_or_else(UnitQuaternion::identity);

                let mut rot_only = bone_world_origin_matrix;
                rot_only.set_column(3, &Vector4::new(0.0, 0.0, 0.0, 1.0));
                let final_rot_mat = rot.to_homogeneous() * rot_only;

                let mut m3 = final_rot_mat.fixed_view::<3, 3>(0, 0).into_owned();
                m3.set_column(0, &m3.column(0).normalize());
                m3.set_column(1, &m3.column(1).normalize());
                m3.set_column(2, &m3.column(2).normalize());
                let rotation3 = nalgebra::Rotation3::from_matrix_unchecked(m3);
                let abs_rot = UnitQuaternion::from_rotation_matrix(&rotation3);

                let axisangle = abs_rot.scaled_axis();
                let glam_axisangle =
                    blue_engine::glam::Vec3::new(axisangle.x, axisangle.y, axisangle.z);
                let glam_pos =
                    blue_engine::glam::Vec3::new(bone_origin.x, bone_origin.y, bone_origin.z);
                let pose1 = rapier3d::math::Pose::new(glam_pos, glam_axisangle);

                let mut pushed_tail = p.current_tail;

                for col_idx in &p.colliders {
                    let col = &self.colliders[*col_idx];
                    let col_world = nodes[col.node_idx].global_transform;
                    let col_pos = (col_world
                        * Vector4::new(col.offset.x, col.offset.y, col.offset.z, 1.0))
                    .xyz();

                    let (shape2, pose2) = if let Some(tail) = col.tail {
                        let col_tail =
                            (col_world * Vector4::new(tail.x, tail.y, tail.z, 1.0)).xyz();

                        let shape = rapier3d::prelude::SharedShape::capsule(
                            blue_engine::glam::Vec3::new(0.0, 0.0, 0.0),
                            blue_engine::glam::Vec3::new(
                                col_tail.x - col_pos.x,
                                col_tail.y - col_pos.y,
                                col_tail.z - col_pos.z,
                            ),
                            col.radius,
                        );
                        let pose = rapier3d::math::Pose::new(
                            blue_engine::glam::Vec3::new(col_pos.x, col_pos.y, col_pos.z),
                            blue_engine::glam::Vec3::ZERO,
                        );
                        (shape, pose)
                    } else {
                        let shape = rapier3d::prelude::SharedShape::ball(col.radius);
                        let pose = rapier3d::math::Pose::new(
                            blue_engine::glam::Vec3::new(col_pos.x, col_pos.y, col_pos.z),
                            blue_engine::glam::Vec3::ZERO,
                        );
                        (shape, pose)
                    };

                    if let Ok(Some(contact)) =
                        rapier3d::parry::query::contact(&pose1, &*shape1, &pose2, &*shape2, 0.001)
                    {
                        if contact.dist < 0.0 {
                            // normal1 points outwards from shape1. We move shape1 away from shape2 by moving in the -normal1 direction
                            let push = contact.normal1 * -contact.dist;

                            let pt1 =
                                Vector3::new(contact.point1.x, contact.point1.y, contact.point1.z);
                            let r1 = (pt1 - bone_origin).norm();
                            let mut lever = 1.0;
                            if r1 > 0.001 {
                                lever = (p.bone_length / r1).clamp(1.0, 5.0);
                            }

                            pushed_tail -= Vector3::new(push.x, push.y, push.z) * lever;
                        }
                    }
                }

                // Also check against body mesh bone hulls (head, hands, arms...)
                for (body_node_idx, body_shape) in &self.body_hull_colliders {
                    let body_world = nodes[*body_node_idx].global_transform;
                    let body_pos = body_world.column(3).xyz();

                    let mut m3 = body_world.fixed_view::<3, 3>(0, 0).into_owned();
                    m3.set_column(0, &m3.column(0).normalize());
                    m3.set_column(1, &m3.column(1).normalize());
                    m3.set_column(2, &m3.column(2).normalize());
                    let body_rot = nalgebra::Rotation3::from_matrix_unchecked(m3);
                    let body_abs_rot = UnitQuaternion::from_rotation_matrix(&body_rot);
                    let body_axisangle = body_abs_rot.scaled_axis();

                    let pose_body = rapier3d::math::Pose::new(
                        blue_engine::glam::Vec3::new(body_pos.x, body_pos.y, body_pos.z),
                        blue_engine::glam::Vec3::new(
                            body_axisangle.x,
                            body_axisangle.y,
                            body_axisangle.z,
                        ),
                    );

                    if let Ok(Some(contact)) = rapier3d::parry::query::contact(
                        &pose1,
                        &*shape1,
                        &pose_body,
                        &**body_shape,
                        0.001,
                    ) {
                        if contact.dist < 0.0 {
                            let push = contact.normal1 * -contact.dist;
                            let pt1 =
                                Vector3::new(contact.point1.x, contact.point1.y, contact.point1.z);
                            let r1 = (pt1 - bone_origin).norm();
                            let mut lever = 1.0;
                            if r1 > 0.001 {
                                lever = (p.bone_length / r1).clamp(1.0, 5.0);
                            }
                            pushed_tail -= Vector3::new(push.x, push.y, push.z) * lever;
                        }
                    }
                }

                p.current_tail = pushed_tail;
            }
        }
        // Pass 2: Self-Collision (PBD)
        // Self-collision is enabled globally, but we skip intra-chain collisions.
        if true {
            let soft_factor = 0.5f32; // We can use a stronger soft factor because this method is perfectly stable

            for _ in 0..self.self_collision_iterations {
                for i in 0..self.particles.len() {
                    let group_id1 = self.particles[i].group_id;
                    if self.particles[i].exclude_from_mesh_collision {
                        continue;
                    }
                    let t1 = self.particles[i].current_tail;
                    let r1 = self.particles[i].hit_radius.max(0.02);

                    for j in (i + 1)..self.particles.len() {
                        let group_id2 = self.particles[j].group_id;
                        if self.particles[j].exclude_from_mesh_collision {
                            continue;
                        }

                        // ONLY collide with other chains. Do NOT collide with our own chain.
                        if group_id1 == group_id2 {
                            continue;
                        }

                        let t2 = self.particles[j].current_tail;
                        let r2 = self.particles[j].hit_radius.max(0.02);

                        let diff = t1 - t2;
                        let dist_sq = diff.norm_squared();
                        let min_dist = (r1 + r2) * 1.5; // Multiply by 1.5 for a slightly thicker virtual self-collision radius

                        if dist_sq < min_dist * min_dist && dist_sq > 0.000001 {
                            let dist = dist_sq.sqrt();
                            let push = (min_dist - dist) * 0.5 * soft_factor;
                            let normal = diff / dist;

                            self.particles[i].current_tail += normal * push;
                            self.particles[j].current_tail -= normal * push;
                        }
                    }
                }
            }
        }

        for particle in &mut self.particles {
            let parent_world = nodes[particle.parent_node_idx].global_transform;
            let bone_world_origin_matrix = parent_world * particle.initial_local_matrix;
            let bone_origin = bone_world_origin_matrix.column(3).xyz();
            let stiffness_target = (bone_world_origin_matrix * particle.initial_local_tail).xyz();

            let initial_tail_dir = (stiffness_target - bone_origin).normalize();
            let mut tail_dir = (particle.current_tail - bone_origin).normalize();
            let dot = tail_dir.dot(&initial_tail_dir).clamp(-1.0, 1.0);
            let angle = dot.acos();

            if angle > particle.max_angle {
                if let Some(rot_axis) = initial_tail_dir.cross(&tail_dir).try_normalize(0.000001) {
                    // Create a quaternion that rotates exactly `max_angle` from `initial_tail_dir`
                    let clamped_rot = UnitQuaternion::from_axis_angle(
                        &nalgebra::Unit::new_unchecked(rot_axis),
                        particle.max_angle,
                    );

                    let target_clamped_dir = clamped_rot * initial_tail_dir;

                    // Smoothly interpolate towards the boundary to act as a soft constraint
                    let blend_factor = 0.15; // 15% pull-back to boundary per frame (gentle soft constraint)
                    tail_dir = (tail_dir * (1.0 - blend_factor)
                        + target_clamped_dir * blend_factor)
                        .normalize();
                } else {
                    // Fallback if exactly 180 degrees reversed
                    tail_dir = initial_tail_dir;
                }
            }

            particle.current_tail = bone_origin + tail_dir * particle.bone_length;

            if let Some(rotation) = UnitQuaternion::rotation_between(&initial_tail_dir, &tail_dir) {
                let rot_mat = rotation.to_homogeneous();

                let mut bone_rot_only = bone_world_origin_matrix;
                bone_rot_only.set_column(3, &Vector4::new(0.0, 0.0, 0.0, 1.0));

                let mut new_world_matrix = rot_mat * bone_rot_only;
                new_world_matrix.set_column(3, &bone_world_origin_matrix.column(3));

                if let Some(parent_inv) = parent_world.try_inverse() {
                    let new_local_matrix = parent_inv * new_world_matrix;

                    nodes[particle.node_idx].local_transform = new_local_matrix;
                    nodes[particle.node_idx].global_transform = new_world_matrix;
                }
            }
        }
    }
}
