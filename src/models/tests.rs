#[cfg(test)]
mod tests {
    use crate::models::vrm_loader::SkinningData;

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
