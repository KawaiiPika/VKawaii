#![allow(clippy::module_inception)]
#[cfg(test)]
mod tests {
    use crate::models::vkw_loader::VkwModel;
    use crate::models::vrm_loader::SkinningData;
    use std::io::{Cursor, Write};
    use zip::write::FileOptions;
    use zip::ZipWriter;

    #[test]
    fn test_vkw_loader_basic() {
        let mut buf = Vec::new();
        let cursor = Cursor::new(&mut buf);
        let mut zip = ZipWriter::new(cursor);

        let options: FileOptions<'_, ()> = FileOptions::default()
            .compression_method(zip::CompressionMethod::Stored)
            .unix_permissions(0o755);

        zip.start_file("manifest.json", options).unwrap();
        zip.write_all(b"{\"name\": \"Test Model\", \"version\": \"1.0\", \"type\": \"avatar\"}")
            .unwrap();

        zip.start_file("model.glb", options).unwrap();
        zip.write_all(b"dummy glb content").unwrap();

        zip.finish().unwrap();

        let loader = VkwModel::load_from_reader(Cursor::new(buf)).expect("Failed to load .vkw");

        assert_eq!(loader.manifest.name, "Test Model");
        assert_eq!(loader.manifest.version, "1.0");
        assert_eq!(loader.manifest.r#type, "avatar");
        assert_eq!(loader.glb_bytes, b"dummy glb content");
    }

    #[test]
    fn test_skinning_data_creation() {
        let data = SkinningData {
            mesh_name: "test_mesh".to_string(),
            original_vertices: vec![],
            indices: vec![],
            joints: vec![],
            weights: vec![],
            skin_idx: 0,
        };
        assert_eq!(data.mesh_name, "test_mesh");
        assert_eq!(data.skin_idx, 0);
    }
}
