//! WGSL shader source strings for clip transforms, compositing, and transition effects.

/// WGSL shader code for clip 2D transforms and opacity blending.
pub const TRANSFORM_WGSL: &str = r"
struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) tex_coords: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
};

struct TransformUniform {
    matrix: mat4x4<f32>,
    opacity: f32,
    pad0: f32,
    pad1: f32,
    pad2: f32,
};

@group(0) @binding(0)
var<uniform> u_transform: TransformUniform;

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = u_transform.matrix * vec4<f32>(model.position, 0.0, 1.0);
    out.tex_coords = model.tex_coords;
    return out;
}

@group(1) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(1) @binding(1)
var s_diffuse: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(t_diffuse, s_diffuse, in.tex_coords);
    return vec4<f32>(color.rgb, color.a * u_transform.opacity);
}
";

/// WGSL shader code for edit cut transitions across adjacent video layers.
pub const TRANSITION_WGSL: &str = r"
struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) tex_coords: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
};

struct TransitionUniform {
    progress: f32,
    transition_type: u32,
    aspect_ratio: f32,
    _pad: f32,
};

@group(0) @binding(0)
var<uniform> u_transition: TransitionUniform;

@vertex
fn vs_transition(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4<f32>(model.position, 0.0, 1.0);
    out.tex_coords = model.tex_coords;
    return out;
}

@group(1) @binding(0)
var t_from: texture_2d<f32>;
@group(1) @binding(1)
var s_from: sampler;

@group(2) @binding(0)
var t_to: texture_2d<f32>;
@group(2) @binding(1)
var s_to: sampler;

@fragment
fn fs_transition(in: VertexOutput) -> @location(0) vec4<f32> {
    let color_from = textureSample(t_from, s_from, in.tex_coords);
    let color_to = textureSample(t_to, s_to, in.tex_coords);
    let p = clamp(u_transition.progress, 0.0, 1.0);

    // 0: Crossfade, 1: Dissolve
    if (u_transition.transition_type == 0u || u_transition.transition_type == 1u) {
        return mix(color_from, color_to, p);
    }
    // 2: FadeToBlack
    if (u_transition.transition_type == 2u) {
        if (p < 0.5) {
            let fade = 1.0 - (p * 2.0);
            return vec4<f32>(color_from.rgb * fade, color_from.a);
        } else {
            let fade = (p - 0.5) * 2.0;
            return vec4<f32>(color_to.rgb * fade, color_to.a);
        }
    }
    // 3: DipToWhite
    if (u_transition.transition_type == 3u) {
        let white = vec3<f32>(1.0, 1.0, 1.0);
        if (p < 0.5) {
            let dip = p * 2.0;
            return vec4<f32>(mix(color_from.rgb, white, dip), color_from.a);
        } else {
            let dip = 1.0 - ((p - 0.5) * 2.0);
            return vec4<f32>(mix(color_to.rgb, white, dip), color_to.a);
        }
    }
    // 4: WipeLeft (incoming reveals from right towards left)
    if (u_transition.transition_type == 4u) {
        let cut = 1.0 - p;
        if (in.tex_coords.x >= cut) {
            return color_to;
        } else {
            return color_from;
        }
    }
    // 5: WipeRight (incoming reveals from left towards right)
    if (u_transition.transition_type == 5u) {
        if (in.tex_coords.x <= p) {
            return color_to;
        } else {
            return color_from;
        }
    }

    return mix(color_from, color_to, p);
}
";
