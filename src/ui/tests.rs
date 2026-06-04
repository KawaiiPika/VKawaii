#![allow(clippy::module_inception)]
#[cfg(test)]
mod tests {
    use crate::ui::overlay::{HullConfig, OverlayState};

    #[test]
    fn test_overlay_state_default() {
        let state = OverlayState::default();
        assert!(state.show_spring_bone_editor);
        assert!(!state.show_material_editor);
        assert!(state.show_body_hulls);
        assert!(state.show_spring_bone_hulls);
        assert!(state.show_spring_colliders);
        assert_eq!(state.global_gravity, -9.81);
    }

    #[test]
    fn test_hull_config_default() {
        let config = HullConfig::default();
        assert_eq!(config.x_squash, 1.0);
        assert_eq!(config.y_squash, 1.0);
        assert_eq!(config.z_squash, 1.0);
        assert_eq!(config.shrink_factor, 0.88);
    }
}
