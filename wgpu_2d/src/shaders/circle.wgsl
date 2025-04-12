#include<common.wgsl>

struct Circle {
	@location(0) color: vec4<f32>,
	@location(1) center: vec2<f32>,
	@location(2) radius: f32,
}

struct V2F {
	@builtin(position) position: vec4<f32>,
	@location(0) color: vec4<f32>,
	@location(1) uv: vec2<f32>,
}


// Vertex Shader outputs a quad along with the UV coordinates
@vertex 
fn v_main (circle: Circle, @builtin(vertex_index) v_id: u32) -> V2F {
	let pos = quad_strip[v_id] * circle.radius + circle.center;

	let clip_space = worldspace_to_clipspace(pos);

	var output: V2F;
	output.color = circle.color;
	output.uv = quad_strip[v_id];
	output.position = vec4<f32>(clip_space, 0., 1.);
	/* output.position = vec4<f32>(0., 0., 0., 1.); */
	return output;
}

// Actual Circle is rendered in the fragment shader
@fragment
fn f_main(v2f: V2F) -> @location(0) vec4<f32> {
	let mag = clamp(dot(v2f.uv, v2f.uv) * (-50.) + 50, 0., 1.);

	if mag == 0. {
		discard;
	}
	return v2f.color * mag;
}
