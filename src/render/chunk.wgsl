struct View {
    view_proj: mat4x4<f32>;
    world_position: vec3<f32>;
};
[[group(0), binding(0)]]
var<uniform> view: View;

struct Transform {
    [[location(0)]] row_0: vec4<f32>;
    [[location(1)]] row_1: vec4<f32>;
    [[location(2)]] row_2: vec4<f32>;
    [[location(3)]] row_3: vec4<f32>;
};

struct Chunk {
    [[location(4)]] chunk_size: vec2<u32>;
    [[location(5)]] tile_size: vec2<u32>;
};

struct VertexOutput {
    [[builtin(position)]] position: vec4<f32>;
    [[location(0)]] uv: vec2<f32>;
    [[location(1)]] tile_index: u32;
};

[[stage(vertex)]]
fn vertex([[builtin(vertex_index)]] index: u32, transform: Transform, chunk: Chunk) -> VertexOutput {
    let tile_index = index / 4u;
    let tile_position = vec2<u32>(tile_index % chunk.chunk_size.x, tile_index / chunk.chunk_size.y);

    let corner_index = index % 4u;
    let corner_position = vec2<u32>(corner_index / 2u, corner_index % 2u);

    let position = (tile_position + corner_position) * chunk.tile_size;

    let world_position = mat4x4<f32>(
        transform.row_0,
        transform.row_1,
        transform.row_2,
        transform.row_3
    ) * vec4<f32>(vec2<f32>(position), 0.0, 1.0);

    var out: VertexOutput;
    out.position = view.view_proj * world_position;
    out.uv = vec2<f32>(corner_position);
    out.tile_index = tile_index;

    return out;
}

struct Tile {
    idx: i32;
    transform: mat4x4<f32>;
    mask_color: vec4<f32>;
};

struct Tiles {
    data: array<Tile>;
};
[[group(1), binding(0)]]
var<storage, read> tiles: Tiles;

[[group(2), binding(0)]]
var texture_array: texture_2d_array<f32>;
[[group(2), binding(1)]]
var texture_sampler: sampler;

[[stage(fragment)]]
fn fragment(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    let tile = tiles.data[in.tile_index];

    let uv = tile.transform * vec4<f32>(in.uv, 1.0, 1.0);
    let color = textureSample(texture_array, texture_sampler, uv.xy, tile.idx);

    if (tile.idx == -2 || uv.x > 1.01 || uv.x < -0.01 || uv.y > 1.01 || uv.y < -0.01) {
        discard;
    } else if (tile.idx == -1) {
        return tile.mask_color;
    }

    return color * tile.mask_color;
}