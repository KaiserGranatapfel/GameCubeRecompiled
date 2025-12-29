// Splash screen shader

struct Uniforms {
    fade: f32,
    rotation: f32,
    _padding: vec2<f32>,
}

@group(0) @binding(0) var texture: texture_2d<f32>;
@group(0) @binding(1) var texture_sampler: sampler;
@group(0) @binding(2) var<uniform> uniforms: Uniforms;

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) tex_coord: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coord: vec2<f32>,
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    
    // Apply rotation
    let cos_r = cos(uniforms.rotation);
    let sin_r = sin(uniforms.rotation);
    let rotated_pos = vec2<f32>(
        input.position.x * cos_r - input.position.y * sin_r,
        input.position.x * sin_r + input.position.y * cos_r
    );
    
    output.clip_position = vec4<f32>(rotated_pos, 0.0, 1.0);
    output.tex_coord = input.tex_coord;
    
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(texture, texture_sampler, input.tex_coord);
    return vec4<f32>(color.rgb, color.a * uniforms.fade);
}

