// blocks

//@CAMERA_STRUCT

//@MTOON_CONSTANTS

struct TransformationUniforms {
    transform_matrix: mat4x4<f32>,
};
@group(2) @binding(0)
var<uniform> transform_uniform: TransformationUniforms;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) texture_coordinates: vec2<f32>,
    @location(2) normal: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) texture_coordinates: vec2<f32>,
    @location(1) normal: vec3<f32>,
};

struct InstanceInput {
    @location(3) model_matrix_0: vec4<f32>,
    @location(4) model_matrix_1: vec4<f32>,
    @location(5) model_matrix_2: vec4<f32>,
    @location(6) model_matrix_3: vec4<f32>,
};

// Vertex Stage
@vertex
fn vs_main(input: VertexInput, instance: InstanceInput) -> VertexOutput {
    let model_matrix = mat4x4<f32>(
        instance.model_matrix_0,
        instance.model_matrix_1,
        instance.model_matrix_2,
        instance.model_matrix_3,
    );

    var out: VertexOutput;
    out.texture_coordinates = input.texture_coordinates;
    
    // Passing normal to the Fragment shader (Assuming uniform Scale for Simplicity)
    let world_normal = (model_matrix * vec4<f32>(input.normal, 0.0)).xyz;
    out.normal = normalize(world_normal);

    //@CAMERA_VERTEX
    return out;
}

// Fragment Stage
struct FragmentUniforms {
    color: vec4<f32>,
};
@group(2) @binding(1)
var<uniform> fragment_uniforms: FragmentUniforms;

@group(0) @binding(0)
var texture_diffuse: texture_2d<f32>;

@group(0) @binding(1)
var sampler_diffuse: sampler;

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    var tex_color = textureSample(texture_diffuse, sampler_diffuse, input.texture_coordinates);
    if fragment_uniforms.color.w != 0.0 {
        tex_color = tex_color * fragment_uniforms.color;
    }
    
    // Hardcoded light Direction pointing from Top-left-front
    let light_dir = normalize(vec3<f32>(-1.0, 0.5, 1.0));
    
    // Half-Lambert reflectance (MToon standard maps -1..1 to 0..1 before Shifting)
    let n_dot_l = dot(normalize(input.normal), light_dir);
    let half_lambert = n_dot_l * 0.5 + 0.5;
    
    // Applying the Shading shift
    let shading = half_lambert + SHADING_SHIFT;
    
    // Using a super tight Smoothstep for a perfectly Flat, hard-edged cel Shading look
    // The tiny 0.01 Width provides just enough Anti-aliasing to prevent Jagged edges
    let lit_factor = smoothstep(-0.01, 0.01, shading);
    
    // Artificially darkening the Shade color so Shadows are much more Visible
    let dark_shade = SHADE_COLOR.rgb * 0.4;
    
    // Mixing the base Texture color and the Shade color
    var final_color_rgb = mix(dark_shade * tex_color.rgb, tex_color.rgb, lit_factor);
    
    // Add on the MToon emission Color
    final_color_rgb = final_color_rgb + EMISSION_COLOR.rgb;
    
    return vec4<f32>(final_color_rgb, tex_color.a);
}
