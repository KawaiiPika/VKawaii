pub mod core;
pub mod models;
pub mod physics;
pub mod rendering;
pub mod tracking;
pub mod ui;

fn main() -> anyhow::Result<()> {
    let mut app = core::engine::VKawaiiEngine::new()?;

    // Setup Armature Test for CPU Skinning
    if std::path::Path::new("VRM/Vita_clothing.vrm").exists() {
        println!("Loading VRM/Vita_clothing.vrm...");
        let mut vrm = models::vrm_loader::VrmModel::load("VRM/Vita_clothing.vrm")?;
        println!("Loaded VRM, spawning into engine...");
        vrm.spawn_into_engine(&mut app.engine)?;

        let armature_test = crate::rendering::armature_test::ArmatureTest::new(&mut vrm);
        app.engine
            .signals
            .add_signal("armature_test", Box::new(armature_test));

        println!("Successfully spawned VRM meshes into Blue Engine!");
    } else {
        println!("No 'VRM/Vita_clothing.vrm' found. Check the path!");
    }

    let orbit_camera = ui::orbit_camera::OrbitCamera::new();

    if let Some(main_camera) = app.engine.camera.get_mut("main") {
        let x = orbit_camera.current_focus.x
            + orbit_camera.current_radius
                * orbit_camera.current_phi.sin()
                * orbit_camera.current_theta.cos();
        let y = orbit_camera.current_focus.y
            + orbit_camera.current_radius * orbit_camera.current_phi.cos();
        let z = orbit_camera.current_focus.z
            + orbit_camera.current_radius
                * orbit_camera.current_phi.sin()
                * orbit_camera.current_theta.sin();

        main_camera.position = blue_engine::Vector3::new(x, y, z);
        main_camera.target = orbit_camera.current_focus;
        main_camera.build_view_projection_matrix();
    }

    app.engine
        .signals
        .add_signal("orbit_camera", Box::new(orbit_camera));

    app.run()?;
    Ok(())
}
