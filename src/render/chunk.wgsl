struct View {
    view_proj: mat4x4<f32>;
    world_position: vec3<f32>;
};
[[group(0), binding(0)]]
var<uniform> view: View;

struct VertexInput {
    [[location(0)]] position: vec3<f32>;
    [[location(1)]] uv: vec2<f32>;
};

struct Transform {
    [[location(2)]] row_0: vec4<f32>;
    [[location(3)]] row_1: vec4<f32>;
    [[location(4)]] row_2: vec4<f32>;
    [[location(5)]] row_3: vec4<f32>;
};

struct VertexOutput {
    [[builtin(position)]] position: vec4<f32>;
    [[location(0)]] uv: vec2<f32>;
};

[[stage(vertex)]]
fn vertex(in: VertexInput, transform: Transform) -> VertexOutput {
    let world_position = mat4x4<f32>(
        transform.row_0,
        transform.row_1,
        transform.row_2,
        transform.row_3
    ) * vec4<f32>(in.position, 1.0);

    var out: VertexOutput;
    out.position = view.view_proj * world_position;
    out.uv = in.uv;

    return out;
}

[[group(1), binding(0)]]
var texture_array: texture_2d_array<f32>;
[[group(1), binding(1)]]
var texture_sampler: sampler;

[[stage(fragment)]]
fn fragment(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    return vec4<f32>(in.uv, 0.0, 1.0);
}