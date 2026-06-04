use blue_engine::{Engine, Vector3};

pub struct OrbitCamera {
    // Target parameters (driven by input)
    pub target_radius: f32,
    pub target_theta: f32,
    pub target_phi: f32,
    pub target_focus: Vector3,

    // Current parameters (smoothed over time)
    pub current_radius: f32,
    pub current_theta: f32,
    pub current_phi: f32,
    pub current_focus: Vector3,

    // Settings
    pub rotation_speed: f32,
    pub zoom_speed: f32,
    pub pan_speed_multiplier: f32,
    pub smooth_speed: f32, // How fast the camera interpolates
}

impl Default for OrbitCamera {
    fn default() -> Self {
        Self::new()
    }
}

impl OrbitCamera {
    pub fn new() -> Self {
        let default_focus = Vector3::new(0.0, 1.0, 0.0);
        let default_radius = 3.0;
        let default_theta = std::f32::consts::FRAC_PI_2;
        let default_phi = std::f32::consts::FRAC_PI_2;

        Self {
            target_radius: default_radius,
            target_theta: default_theta,
            target_phi: default_phi,
            target_focus: default_focus,

            current_radius: default_radius,
            current_theta: default_theta,
            current_phi: default_phi,
            current_focus: default_focus,

            rotation_speed: 0.01,
            zoom_speed: 1.0,
            pan_speed_multiplier: 0.0013,
            smooth_speed: 0.2, // ~60fps equivalent smoothing factor
        }
    }
}

impl blue_engine::Signal for OrbitCamera {
    fn frame(
        &mut self,
        engine: &mut Engine,
        _encoder: &mut blue_engine::CommandEncoder,
        _view: &blue_engine::TextureView,
    ) {
        let (mouse_dx, mouse_dy) = engine.simple_input.mouse_diff();

        let (_, scroll_y) = engine.simple_input.scroll_diff();

        let mut input_active = false;

        // Map scroll wheel input to Exponential Zoom to maintain smooth scaling at large Distances
        let ui_wants_pointer = crate::ui::overlay::OVERLAY_STATE
            .lock()
            .unwrap()
            .ui_wants_pointer;
        if scroll_y != 0.0 && !ui_wants_pointer {
            // Scale zoom Speed based on Proximity to the target
            let zoom_amt = scroll_y * self.zoom_speed * (self.target_radius * 0.1).max(0.1);
            self.target_radius = (self.target_radius - zoom_amt).clamp(0.1, 50.0);
            input_active = true;
        }

        let is_orbiting = (engine
            .simple_input
            .mouse_held(blue_engine::MouseButton::Right)
            || (engine
                .simple_input
                .mouse_held(blue_engine::MouseButton::Middle)
                && !engine
                    .simple_input
                    .key_held(blue_engine::KeyCode::ShiftLeft)))
            && !ui_wants_pointer;

        let is_panning = engine
            .simple_input
            .mouse_held(blue_engine::MouseButton::Middle)
            && !is_orbiting
            && !ui_wants_pointer;

        if is_orbiting && (mouse_dx != 0.0 || mouse_dy != 0.0) {
            self.target_theta += mouse_dx * self.rotation_speed;
            self.target_phi -= mouse_dy * self.rotation_speed;

            // Constrain pitch to Prevent gimbal lock at the poles
            let epsilon = 0.01;
            self.target_phi = self
                .target_phi
                .clamp(epsilon, std::f32::consts::PI - epsilon);
            input_active = true;
        }

        // Camera Pan
        if is_panning && (mouse_dx != 0.0 || mouse_dy != 0.0) {
            // Project the 2D mouse delta onto the 3D viewing plane to allow intuitive panning
            let forward = Vector3::new(
                self.current_phi.sin() * self.current_theta.cos(),
                self.current_phi.cos(),
                self.current_phi.sin() * self.current_theta.sin(),
            )
            .normalize();

            let up = Vector3::new(0.0, 1.0, 0.0);
            let right = forward.cross(up).normalize();
            let cam_up = right.cross(forward).normalize();

            let dynamic_pan = self.target_radius * self.pan_speed_multiplier;
            self.target_focus += right * (mouse_dx * dynamic_pan); // Inverted left-right for panning only
            self.target_focus += cam_up * (mouse_dy * dynamic_pan);
            input_active = true;
        }

        let lerp_factor = self.smooth_speed;

        let diff_radius = (self.target_radius - self.current_radius).abs();
        let diff_theta = (self.target_theta - self.current_theta).abs();
        let diff_phi = (self.target_phi - self.current_phi).abs();
        let diff_focus = (self.target_focus - self.current_focus).length();

        // Only update if there's a meaningful difference
        if input_active
            || diff_radius > 0.001
            || diff_theta > 0.001
            || diff_phi > 0.001
            || diff_focus > 0.001
        {
            self.current_radius += (self.target_radius - self.current_radius) * lerp_factor;
            self.current_theta += (self.target_theta - self.current_theta) * lerp_factor;
            self.current_phi += (self.target_phi - self.current_phi) * lerp_factor;
            self.current_focus += (self.target_focus - self.current_focus) * lerp_factor;

            if let Some(main_camera) = engine.camera.get_mut("main") {
                let x = self.current_focus.x
                    + self.current_radius * self.current_phi.sin() * self.current_theta.cos();
                let y = self.current_focus.y + self.current_radius * self.current_phi.cos();
                let z = self.current_focus.z
                    + self.current_radius * self.current_phi.sin() * self.current_theta.sin();

                main_camera.position = Vector3::new(x, y, z);
                main_camera.target = self.current_focus;
                main_camera.build_view_projection_matrix();
            }
        }
    }
}
