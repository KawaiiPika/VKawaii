#![allow(clippy::module_inception)]
#[cfg(test)]
mod tests {
    use crate::physics::spring_bones::SpringBoneSystem;

    #[test]
    fn test_spring_particle_physics() {
        // Basic physics Test
        let system = SpringBoneSystem::new();
        // Verifying the System initializes Since there's no Full rig here
        assert_eq!(system.particles.len(), 0);
        assert_eq!(system.colliders.len(), 0);
        assert!(!system.enable_self_collision);
    }
}
