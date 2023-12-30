@group(0) @binding(0) var texture: texture_storage_2d<rgba8unorm, read_write>;

fn hash(value: u32) -> u32 {
    var state = value;
    state = state ^ 2747636419u;
    state = state * 2654435769u;
    state = state ^ state >> 16u;
    state = state * 2654435769u;
    state = state ^ state >> 16u;
    state = state * 2654435769u;
    return state;
}

fn randomFloat(value: u32) -> f32 {
    return f32(hash(value)) / 4294967295.0;
}

@compute
@workgroup_size(8, 8)
fn init(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(num_workgroups) num_workgroups: vec3<u32>,
) {
    let location = vec2<u32>(global_id.xy);
    let alive = randomFloat(location.x * num_workgroups.x + location.y) > 0.9;
    let color = vec4f(f32(alive));
    textureStore(texture, location, color);
}

fn is_alive(location: vec2<i32>, offset_x: i32, offset_y: i32) -> i32 {
    return i32(textureLoad(texture, location + vec2<i32>(offset_x, offset_y)).x);
}

@compute
@workgroup_size(8, 8)
fn update(
    @builtin(global_invocation_id) global_id: vec3<u32>,
) {
    let location = vec2<i32>(global_id.xy);

    // Set alive if there are 2 or 3 alive neighbors.
    let alive_neighbors: i32 =
        is_alive(location, -1, -1) +
        is_alive(location, -1, 0) +
        is_alive(location, -1, 1) +
        is_alive(location, 0, -1) +
        is_alive(location, 0, 1) +
        is_alive(location, 1, -1) +
        is_alive(location, 1, 0) +
        is_alive(location, 1, 1);

    var alive: bool;
    if (alive_neighbors == 3) {
        alive = true;
    } else if (alive_neighbors == 2) {
        alive = is_alive(location, 0, 0) == 1;
    } else {
        alive = false;
    }

    let color = vec4f(f32(alive));
    textureStore(texture, location, color);
}
 
struct VSOutput {
  @builtin(position) position: vec4f,
};
 
@vertex
fn set_cells_vs(@location(0) position: vec2f) -> VSOutput {
  var vsOut: VSOutput;
  vsOut.position = vec4f(position, 0.0, 1.0);
  return vsOut;
}
 
@fragment fn set_cells_fs(vsOut: VSOutput) -> @location(0) vec4f {
  return vec4f(1.0); // Black
}