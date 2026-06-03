use blue_engine::{Engine, Vertex};
use nalgebra::{Matrix4, Vector3, Vector4};

pub struct Node {
    pub local_transform: Matrix4<f32>,
    pub global_transform: Matrix4<f32>,
    pub children: Vec<usize>,
    pub name: Option<String>,
}

pub struct Skin {
    pub joints: Vec<usize>,
    pub inverse_bind_matrices: Vec<Matrix4<f32>>,
}

pub struct SkinningSystem {
    pub nodes: Vec<Node>,
    pub skins: Vec<Skin>,
}

impl Default for SkinningSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl SkinningSystem {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            skins: Vec::new(),
        }
    }

    /// Updates global transforms by walking the node hierarchy
    pub fn update_global_transforms(&mut self, root_nodes: &[usize]) {
        for &root in root_nodes {
            self.update_node_global(root, Matrix4::identity());
        }
    }

    fn update_node_global(&mut self, node_idx: usize, parent_global: Matrix4<f32>) {
        if node_idx >= self.nodes.len() {
            return;
        }

        let global = parent_global * self.nodes[node_idx].local_transform;
        self.nodes[node_idx].global_transform = global;

        // Clone children to avoid borrowing self while mutating
        let children = self.nodes[node_idx].children.clone();
        for child in children {
            self.update_node_global(child, global);
        }
    }

    /// Compute joint matrices for a specific skin
    pub fn compute_joint_matrices(&self, skin_idx: usize) -> Vec<Matrix4<f32>> {
        let skin = &self.skins[skin_idx];
        let mut joint_matrices = Vec::with_capacity(skin.joints.len());

        for (i, &joint_node_idx) in skin.joints.iter().enumerate() {
            let global_transform = self.nodes[joint_node_idx].global_transform;
            let inverse_bind = skin.inverse_bind_matrices[i];
            joint_matrices.push(global_transform * inverse_bind);
        }

        joint_matrices
    }

    /// Update vertices using CPU skinning
    pub fn skin_vertices(
        &self,
        skin_idx: usize,
        original_vertices: &[Vertex],
        joints: &[[u16; 4]],
        weights: &[[f32; 4]],
    ) -> Vec<Vertex> {
        if skin_idx >= self.skins.len() {
            return original_vertices.to_vec();
        }

        let joint_matrices = self.compute_joint_matrices(skin_idx);
        let mut skinned_vertices = original_vertices.to_vec();

        for i in 0..original_vertices.len() {
            let v = &original_vertices[i];
            let j = joints[i];
            let w = weights[i];

            let mut skinned_pos = Vector3::zeros();
            let mut skinned_norm = Vector3::zeros();

            let pos = Vector4::new(v.position[0], v.position[1], v.position[2], 1.0);
            let norm = Vector4::new(v.normal[0], v.normal[1], v.normal[2], 0.0);

            for k in 0..4 {
                let weight = w[k];
                if weight > 0.0 {
                    let joint_idx = j[k] as usize;
                    if joint_idx < joint_matrices.len() {
                        let joint_mat = joint_matrices[joint_idx];

                        let p = joint_mat * pos;
                        skinned_pos += Vector3::new(p.x, p.y, p.z) * weight;

                        let n = joint_mat * norm;
                        skinned_norm += Vector3::new(n.x, n.y, n.z) * weight;
                    }
                }
            }

            let norm_mag = (skinned_norm.x * skinned_norm.x
                + skinned_norm.y * skinned_norm.y
                + skinned_norm.z * skinned_norm.z)
                .sqrt();
            if norm_mag > 0.0001 {
                skinned_norm /= norm_mag;
            }

            skinned_vertices[i].position = [skinned_pos.x, skinned_pos.y, skinned_pos.z];
            skinned_vertices[i].normal = [skinned_norm.x, skinned_norm.y, skinned_norm.z];
        }

        skinned_vertices
    }

    /// Upload skinned vertices back to blue_engine Object buffer
    pub fn upload_to_gpu(engine: &mut Engine, mesh_name: &str, skinned_vertices: Vec<Vertex>) {
        if let Some(object) = engine.objects.get_mut(mesh_name) {
            object.vertices = skinned_vertices;
            // Easiest way in 0.10.0: just rebuild the vertex buffer on the GPU
            let new_vb = engine
                .renderer
                .build_vertex_buffer(&object.vertices, &object.indices);
            object.pipeline.vertex_buffer = blue_engine::PipelineData::Data(new_vb);
        }
    }
}
