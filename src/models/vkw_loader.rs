use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Read;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct VkwManifest {
    pub version: String,
    pub r#type: String, // "avatar", "prop", etc.
    pub name: String,
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
        let mut archive = zip::ZipArchive::new(file).context("Failed to read .vkw as ZIP archive")?;

        // 1. Read manifest.json
        let manifest: VkwManifest = {
            let mut manifest_file = archive.by_name("manifest.json")
                .context("Missing manifest.json in .vkw archive")?;
            
            let mut manifest_str = String::new();
            manifest_file.read_to_string(&mut manifest_str)?;
            serde_json::from_str(&manifest_str)
                .context("Failed to parse manifest.json")?
        };

        println!("[VKW Loader] Successfully parsed manifest for: {}", manifest.name);

        // 2. Read model.glb
        let glb_bytes = {
            let mut glb_file = archive.by_name("model.glb")
                .context("Missing model.glb in .vkw archive")?;
            
            let mut glb_bytes = Vec::new();
            glb_file.read_to_end(&mut glb_bytes)?;
            glb_bytes
        };

        println!("[VKW Loader] Extracted model.glb ({} bytes)", glb_bytes.len());

        // 3. (Optional) In the future, we will extract .dxbc files from a "shaders/" directory 
        // inside the zip, and pass them to our dxbc_parser here!

        Ok(Self {
            manifest,
            glb_bytes,
        })
    }
}
