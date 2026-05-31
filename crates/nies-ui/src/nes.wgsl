// Fullscreen-triangle palette-LUT shader.
// Vertex stage emits an oversized triangle covering the viewport.
// Fragment stage reads the palette index from an R8Unorm texture (the
// normalized byte is recovered as round(v * 255)) and looks the RGB up in
// a 64-entry uniform LUT.

struct Palette {
    colors: array<vec4<f32>, 64>,
};

@group(0) @binding(0) var index_tex: texture_2d<f32>;
@group(0) @binding(1) var<uniform> palette: Palette;

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VsOut {
    // Oversized triangle: clip-space corners (-1,-1),(3,-1),(-1,3).
    let x = f32((vi << 1u) & 2u) * 2.0 - 1.0;
    let y = f32(vi & 2u) * 2.0 - 1.0;
    var out: VsOut;
    out.pos = vec4<f32>(x, y, 0.0, 1.0);
    // UV with row 0 at the top of the image.
    out.uv = vec2<f32>((x + 1.0) * 0.5, (1.0 - y) * 0.5);
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let dim = vec2<f32>(256.0, 240.0);
    let coord = vec2<i32>(clamp(in.uv, vec2<f32>(0.0), vec2<f32>(1.0)) * dim);
    let c = clamp(coord, vec2<i32>(0), vec2<i32>(255, 239));
    // R8Unorm stores the palette index as a normalized byte; recover it.
    // Mask to 6 bits: NES palette indices are 0..=63, and WGSL out-of-bounds
    // array indexing is implementation-defined, so guard against a stray
    // framebuffer byte > 0x3F.
    let index = u32(round(textureLoad(index_tex, c, 0).r * 255.0)) & 63u;
    return palette.colors[index];
}
