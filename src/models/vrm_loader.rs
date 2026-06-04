use anyhow::Result;
use blue_engine::{Engine, Object, ObjectSettings, Vertex};
use gltf::Gltf;

pub struct VrmModel {
    pub gltf_document: gltf::Document,
    pub buffers: Vec<gltf::buffer::Data>,
    pub images: Vec<gltf::image::Data>,
    pub skinning_data: Vec<SkinningData>,
    pub skinning_system: crate::rendering::skinning::SkinningSystem,
    pub spring_bone_system: crate::physics::spring_bones::SpringBoneSystem,
    pub mtoon_materials: std::collections::HashMap<String, serde_json::Value>,
    pub vrm0_data: Option<serde_json::Value>,
}

pub struct SkinningData {
    pub mesh_name: String,
    pub original_vertices: Vec<Vertex>,
    pub indices: Vec<u16>,
    pub joints: Vec<[u16; 4]>,
    pub weights: Vec<[f32; 4]>,
    pub skin_idx: usize,
}

impl VrmModel {
    /// Loading a VRM (or GLB) file and parsing the base Gltf data
    pub fn load(path: &str) -> Result<Self> {
        println!("Loading VRM/{}...", path);
        let (document, buffers, images) = gltf::import(path)?;

        let gltf_raw = Gltf::open(path)?;
        let raw_json = gltf_raw.document.into_json();

        let spring_bone_system = crate::physics::spring_bones::SpringBoneSystem::new();
        let mut mtoon_materials = std::collections::HashMap::new();
        if let Some(extensions) = &raw_json.extensions {
            if let Some(vrm) = extensions.others.get("VRM") {
                if let Some(materials) = vrm.get("materialProperties") {
                    if let Some(materials_array) = materials.as_array() {
                        for mat in materials_array.iter() {
                            if let Some(name) = mat.get("name").and_then(|n| n.as_str()) {
                                mtoon_materials.insert(name.to_string(), mat.clone());
                            }
                        }
                    }
                }
            }
        }

        let mut skinning_system = crate::rendering::skinning::SkinningSystem::new();

        let _skinning_data = Vec::<SkinningData>::new();
        for node in document.nodes() {
            if let Some(mesh) = node.mesh() {
                let _transform = node.transform().matrix();

                if let Some(_skin) = node.skin() {
                    println!(
                        "Mesh {} has skinning data!",
                        mesh.name().unwrap_or("Unnamed")
                    );
                }
            }

            let (t, r, s) = node.transform().decomposed();
            let translation = nalgebra::Translation3::new(t[0], t[1], t[2]);
            let rotation = nalgebra::UnitQuaternion::from_quaternion(nalgebra::Quaternion::new(
                r[3], r[0], r[1], r[2],
            ));
            let scale = nalgebra::Matrix4::new_nonuniform_scaling(&nalgebra::Vector3::new(
                s[0], s[1], s[2],
            ));
            let local_transform = translation.to_homogeneous() * rotation.to_homogeneous() * scale;

            skinning_system
                .nodes
                .push(crate::rendering::skinning::Node {
                    local_transform,
                    global_transform: nalgebra::Matrix4::identity(),
                    children: node.children().map(|c| c.index()).collect(),
                    name: node.name().map(|s| s.to_string()),
                });
        }

        for skin in document.skins() {
            let joints = skin.joints().map(|j| j.index()).collect();
            let reader = skin.reader(|buffer| Some(&buffers[buffer.index()]));
            let inverse_bind_matrices = reader
                .read_inverse_bind_matrices()
                .map(|iter| {
                    iter.map(|m| {
                        nalgebra::Matrix4::new(
                            m[0][0], m[0][1], m[0][2], m[0][3], m[1][0], m[1][1], m[1][2], m[1][3],
                            m[2][0], m[2][1], m[2][2], m[2][3], m[3][0], m[3][1], m[3][2], m[3][3],
                        )
                        .transpose()
                    }) // gltf matrices are column major, nalgebra expects row major arguments, so transpose
                    .collect()
                })
                .unwrap_or_default();

            skinning_system
                .skins
                .push(crate::rendering::skinning::Skin {
                    joints,
                    inverse_bind_matrices,
                });
        }

        let root_nodes: Vec<usize> = document
            .scenes()
            .flat_map(|s| s.nodes().map(|n| n.index()))
            .collect();
        skinning_system.update_global_transforms(&root_nodes);

        let mut vrm0_data = None;
        let raw_json_val = serde_json::to_value(&raw_json).unwrap_or(serde_json::Value::Null);
        if let Some(extensions) = raw_json_val.get("extensions") {
            if let Some(others) = extensions.as_object() {
                if let Some(vrm) = others.get("VRM") {
                    vrm0_data = Some(vrm.clone());
                }
            }
        }

        Ok(Self {
            gltf_document: document,
            buffers,
            images,
            skinning_data: Vec::new(),
            skinning_system,
            spring_bone_system,
            mtoon_materials,
            vrm0_data,
        })
    }

    /// Spawning the Loaded meshes into the Blue Engine and returning SkinningData if any
    pub fn spawn_into_engine(&mut self, engine: &mut Engine) -> Result<()> {
        // Building a dummy Uniform to grab the correct layout (Transform + Color) for Group 2
        let dummy_uniform = engine.renderer.build_uniform_buffer(&[
            engine
                .renderer
                .build_uniform_buffer_part("Transformation Matrix", blue_engine::Matrix4::IDENTITY),
            engine.renderer.build_uniform_buffer_part(
                "Color",
                blue_engine::utils::default_resources::DEFAULT_COLOR,
            ),
        ]);

        for mesh in self.gltf_document.meshes() {
            let mesh_name = mesh.name().unwrap_or("UnnamedMesh").to_string();

            for (primitive_index, primitive) in mesh.primitives().enumerate() {
                let reader = primitive.reader(|buffer| Some(&self.buffers[buffer.index()]));

                let positions = reader
                    .read_positions()
                    .map(|iter| iter.collect::<Vec<[f32; 3]>>())
                    .unwrap_or_default();

                let normals = reader
                    .read_normals()
                    .map(|iter| iter.collect::<Vec<[f32; 3]>>())
                    .unwrap_or_else(|| vec![[0.0, 1.0, 0.0]; positions.len()]);

                let uvs = reader
                    .read_tex_coords(0)
                    .map(|read_tex_coords| read_tex_coords.into_f32().collect::<Vec<[f32; 2]>>())
                    .unwrap_or_else(|| vec![[0.0, 0.0]; positions.len()]);

                let joints = reader
                    .read_joints(0)
                    .map(|iter| iter.into_u16().collect::<Vec<[u16; 4]>>())
                    .unwrap_or_else(|| vec![[0; 4]; positions.len()]);

                let weights = reader
                    .read_weights(0)
                    .map(|iter| iter.into_f32().collect::<Vec<[f32; 4]>>())
                    .unwrap_or_else(|| vec![[1.0, 0.0, 0.0, 0.0]; positions.len()]);

                let mut vertices = Vec::new();
                for i in 0..positions.len() {
                    vertices.push(Vertex {
                        position: positions[i],
                        uv: uvs[i],
                        normal: normals[i],
                    });
                }

                let mut skin_idx = 0; // Default to first skin
                if let Some(mesh_node) = self
                    .gltf_document
                    .nodes()
                    .find(|n| n.mesh().is_some_and(|m| m.index() == mesh.index()))
                {
                    if let Some(skin) = mesh_node.skin() {
                        skin_idx = skin.index();
                    }
                }

                let indices = reader
                    .read_indices()
                    .map(|read_indices| {
                        read_indices
                            .into_u32()
                            .map(|i| i as u16)
                            .collect::<Vec<u16>>()
                    })
                    .unwrap_or_else(|| (0..positions.len() as u16).collect::<Vec<u16>>());

                self.skinning_data.push(SkinningData {
                    mesh_name: format!("{}_{}", mesh_name, primitive_index),
                    original_vertices: vertices.clone(),
                    indices: indices.clone(),
                    joints,
                    weights,
                    skin_idx,
                });

                let object_name = format!("{}_{}", mesh_name, primitive_index);

                if let Ok(mut object) = Object::new(
                    &object_name,
                    vertices,
                    indices,
                    ObjectSettings::default(),
                    &mut engine.renderer,
                ) {
                    let material = primitive.material();

                    let mut shade_shift = -0.3f32;
                    let mut shade_toony = 0.9f32;
                    let mut shade_color = [0.9f32, 0.8f32, 0.8f32, 1.0f32];
                    let mut emission_color = [0.0f32, 0.0f32, 0.0f32, 1.0f32];

                    if let Some(mat_name) = material.name() {
                        if let Some(mtoon_json) = self.mtoon_materials.get(mat_name) {
                            if let Some(float_props) = mtoon_json.get("floatProperties") {
                                if let Some(shift) =
                                    float_props.get("_ShadeShift").and_then(|v| v.as_f64())
                                {
                                    shade_shift = shift as f32;
                                }
                                if let Some(toony) =
                                    float_props.get("_ShadeToony").and_then(|v| v.as_f64())
                                {
                                    shade_toony = toony as f32;
                                }
                            }
                            if let Some(vec_props) = mtoon_json.get("vectorProperties") {
                                if let Some(sc) =
                                    vec_props.get("_ShadeColor").and_then(|v| v.as_array())
                                {
                                    if sc.len() >= 4 {
                                        shade_color = [
                                            sc[0].as_f64().unwrap_or(1.0) as f32,
                                            sc[1].as_f64().unwrap_or(1.0) as f32,
                                            sc[2].as_f64().unwrap_or(1.0) as f32,
                                            sc[3].as_f64().unwrap_or(1.0) as f32,
                                        ];
                                    }
                                }
                                if let Some(ec) =
                                    vec_props.get("_EmissionColor").and_then(|v| v.as_array())
                                {
                                    if ec.len() >= 4 {
                                        emission_color = [
                                            ec[0].as_f64().unwrap_or(0.0) as f32,
                                            ec[1].as_f64().unwrap_or(0.0) as f32,
                                            ec[2].as_f64().unwrap_or(0.0) as f32,
                                            ec[3].as_f64().unwrap_or(1.0) as f32,
                                        ];
                                    }
                                }
                            }
                        }
                    }

                    let mut shader_source =
                        include_str!("../rendering/toon_shader.wgsl").to_string();
                    shader_source = shader_source.replace(
                        "//@CAMERA_STRUCT",
                        r#"struct CameraUniforms {
                            camera_matrix: mat4x4<f32>,
                        };
                        @group(1) @binding(0)
                        var<uniform> camera_uniform: CameraUniforms;"#,
                    );
                    shader_source = shader_source.replace(
                        "//@CAMERA_VERTEX",
                        r#"out.position = camera_uniform.camera_matrix * model_matrix * 
(transform_uniform.transform_matrix * vec4<f32>(input.position, 1.0));"#,
                    );
                    shader_source = shader_source.replace(
                        "//@MTOON_CONSTANTS",
                        &format!(
                            "const SHADING_SHIFT: f32 = {:.5};\n\
                             const SHADING_TOONY: f32 = {:.5};\n\
                             const SHADE_COLOR: vec4<f32> = vec4<f32>({:.5}, {:.5}, {:.5}, {:.5});\n\
                             const EMISSION_COLOR: vec4<f32> = vec4<f32>({:.5}, {:.5}, {:.5}, {:.5});",
                            shade_shift, shade_toony,
                            shade_color[0], shade_color[1], shade_color[2], shade_color[3],
                            emission_color[0], emission_color[1], emission_color[2], emission_color[3]
                        )
                    );

                    let custom_shader = engine.renderer.build_shader(
                        format!("MToon_Shader_{}", object_name).as_str(),
                        shader_source.clone(),
                        Some(&dummy_uniform.1),
                        blue_engine::ShaderSettings::default(),
                    );

                    object.pipeline.shader = blue_engine::PipelineData::Data(custom_shader.clone());

                    // Prevent BLUE_ENGINE FROM Overwriting THE SHADER ON UPDATE:
                    object.shader_builder.shader = shader_source.clone();

                    let pbr = material.pbr_metallic_roughness();
                    if let Some(base_color_texture) = pbr.base_color_texture() {
                        let tex_index = base_color_texture.texture().source().index();
                        let gltf_image = &self.images[tex_index];

                        let dyn_img_opt = match gltf_image.format {
                            gltf::image::Format::R8G8B8 => image::RgbImage::from_raw(
                                gltf_image.width,
                                gltf_image.height,
                                gltf_image.pixels.clone(),
                            )
                            .map(image::DynamicImage::ImageRgb8),
                            gltf::image::Format::R8G8B8A8 => image::RgbaImage::from_raw(
                                gltf_image.width,
                                gltf_image.height,
                                gltf_image.pixels.clone(),
                            )
                            .map(image::DynamicImage::ImageRgba8),
                            _ => None,
                        };

                        if let Some(dyn_img) = dyn_img_opt {
                            if let Ok(built_texture) = engine.renderer.build_texture(
                                format!("{}_Tex", object_name),
                                blue_engine::TextureData::Image(dyn_img),
                                blue_engine::TextureMode::Clamp,
                            ) {
                                object.pipeline.texture =
                                    blue_engine::PipelineData::Data(built_texture);
                            }
                        }
                    } else {
                        let color = pbr.base_color_factor();
                        let _ = object.set_color(color[0], color[1], color[2], color[3]);
                    }

                    engine.objects.insert(object_name.into(), object);
                }
            }
        }

        if let Some(vrm) = &self.vrm0_data {
            crate::models::spring_bone_parser::parse_vrm0_spring_bones(
                vrm,
                &self.skinning_system.nodes,
                &self.skinning_system.skins,
                &self.skinning_data,
                &mut self.spring_bone_system,
            );
        }

        crate::models::spring_bone_parser::build_body_hull_colliders(
            &self.skinning_system.nodes,
            &self.skinning_system.skins,
            &self.skinning_data,
            &mut self.spring_bone_system,
        );

        for (node_idx, (vertices, indices)) in &self.spring_bone_system.hull_vis_meshes {
            let object_name = format!("HullVis_{}", node_idx);
            if let Ok(mut object) = Object::new(
                &object_name,
                vertices.clone(),
                indices.clone(),
                ObjectSettings::default(),
                &mut engine.renderer,
            ) {
                let _ = object.set_color(0.0, 1.0, 0.0, 0.8);
                object.is_visible = false;

                engine.objects.insert(object_name.into(), object);
            }
        }

        let body_hull_vis: Vec<_> = self.spring_bone_system.body_hull_vis_meshes.clone();
        for (node_idx, (vertices, indices)) in body_hull_vis {
            let object_name = format!("BodyHullVis_{}", node_idx);
            if let Ok(mut object) = Object::new(
                &object_name,
                vertices,
                indices,
                ObjectSettings::default(),
                &mut engine.renderer,
            ) {
                let _ = object.set_color(0.0, 0.8, 1.0, 0.7); // Cyan
                object.is_visible = false;
                engine.objects.insert(object_name.into(), object);
            }
        }

        let spring_col_vis: Vec<_> = self
            .spring_bone_system
            .spring_collider_vis_meshes
            .clone()
            .into_iter()
            .collect();
        for (idx, (vertices, indices)) in spring_col_vis {
            let object_name = format!("SpringColVis_{}", idx);
            if let Ok(mut object) = Object::new(
                &object_name,
                vertices,
                indices,
                ObjectSettings::default(),
                &mut engine.renderer,
            ) {
                let _ = object.set_color(1.0, 0.5, 0.0, 0.7); // Orange
                object.is_visible = false;
                engine.objects.insert(object_name.into(), object);
            }
        }

        Ok(())
    }

    pub fn print_scene_info(&self) {
        for scene in self.gltf_document.scenes() {
            println!("Scene: {}", scene.name().unwrap_or("Unnamed"));
            for node in scene.nodes() {
                println!("  Node: {}", node.name().unwrap_or("Unnamed"));
            }
        }
    }
}
