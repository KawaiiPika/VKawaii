pub struct MToonMaterial {
    pub base_color: [f32; 4],
    pub shade_color: [f32; 4],
    pub receive_shadow_rate: f32,
    pub shading_toony_factor: f32,
    pub shading_shift_factor: f32,
    pub light_color_attenuation: f32,
    pub global_illumination_compensation: f32,

    pub rim_color: [f32; 4],
    pub rim_fresnel_power: f32,
    pub rim_lift_factor: f32,

    pub outline_width_mode: u32,
    pub outline_width_factor: f32,
    pub outline_color: [f32; 4],
}

impl Default for MToonMaterial {
    fn default() -> Self {
        Self {
            base_color: [1.0, 1.0, 1.0, 1.0],
            shade_color: [0.9, 0.9, 0.9, 1.0],
            receive_shadow_rate: 1.0,
            shading_toony_factor: 0.9,
            shading_shift_factor: 0.0,
            light_color_attenuation: 0.5,
            global_illumination_compensation: 1.0,

            rim_color: [0.0, 0.0, 0.0, 0.0],
            rim_fresnel_power: 1.0,
            rim_lift_factor: 0.0,

            outline_width_mode: 0,
            outline_width_factor: 0.0,
            outline_color: [0.0, 0.0, 0.0, 1.0],
        }
    }
}
