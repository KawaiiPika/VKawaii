use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Read;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct SpringBoneConfig {
    pub name: String,
    pub stiffness: f32,
    pub radius: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConstraintConfig {
    pub bone: String,
    pub r#type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtarget: Option<String>,
    pub influence: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BlendshapeDriver {
    pub shape_key: String,
    pub bone: String,
    pub axis: String,
    pub coefficient: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AvatarConfig {
    #[serde(default)]
    pub constraints: Vec<ConstraintConfig>,
    #[serde(default)]
    pub spring_bones: Vec<SpringBoneConfig>,
    #[serde(default)]
    pub blendshape_drivers: Vec<BlendshapeDriver>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VkwManifest {
    pub version: String,
    pub r#type: String, // "avatar", "prop", etc.
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_config: Option<AvatarConfig>,
    // Add more fields as we figure them out!
}

pub struct VkwModel {
    pub manifest: VkwManifest,
    pub glb_bytes: Vec<u8>,
}

impl VkwModel {
    /// Loads a `.vkw` file (which is a ZIP archive), parses its `manifest.json`,
    /// and extracts the inner `model.glb` and shaders.
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path).context("Failed to open .vkw file")?;
        Self::load_from_reader(file)
    }

    pub fn load_from_reader<R: std::io::Read + std::io::Seek>(reader: R) -> Result<Self> {
        let mut archive =
            zip::ZipArchive::new(reader).context("Failed to read .vkw as ZIP archive")?;

        // The manifest contains critical metadata like format versions and layout types.
        // It must be parsed first to validate the archive before allocating heavy mesh data.
        let manifest: VkwManifest = {
            let mut manifest_file = archive
                .by_name("manifest.json")
                .context("Missing manifest.json in .vkw archive")?;

            let mut manifest_str = String::new();
            manifest_file.read_to_string(&mut manifest_str)?;
            serde_json::from_str(&manifest_str).context("Failed to parse manifest.json")?
        };

        println!(
            "[VKW Loader] Successfully parsed manifest for: {}",
            manifest.name
        );

        // The raw glTF binary contains all the geometry and basic PBR materials.
        // We load it into a contiguous byte buffer so the `gltf` crate can process it lazily.
        let glb_bytes = {
            let mut glb_file = archive
                .by_name("model.glb")
                .context("Missing model.glb in .vkw archive")?;

            let mut glb_bytes = Vec::new();
            glb_file.read_to_end(&mut glb_bytes)?;
            glb_bytes
        };

        println!(
            "[VKW Loader] Extracted model.glb ({} bytes)",
            glb_bytes.len()
        );

        Ok(Self {
            manifest,
            glb_bytes,
        })
    }
}
