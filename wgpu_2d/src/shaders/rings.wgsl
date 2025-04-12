#include<common.wgsl>

struct Ring {
	@location(0) color: vec4<f32>,
	@location(1) center: vec2<f32>,
	@location(2) outer_radius: f32,
	@location(3) inner_radius: f32,
}

struct V2F {
	@builtin(position) position: vec4<f32>,
	@location(0) color: vec4<f32>,
	@location(1) uv: vec2<f32>,
	@location(2) radius_ratio: f32,
}


// Vertex Shader outputs a quad along with the UV coordinates
@vertex 
fn v_main (ring: Ring, @builtin(vertex_index) v_id: u32) -> V2F {
	let pos = quad_strip[v_id] * ring.outer_radius + ring.center;

	let clip_space = worldspace_to_clipspace(pos);

	var output: V2F;
	output.color = ring.color;
	output.uv = quad_strip[v_id];
	output.position = vec4<f32>(clip_space, 0., 1.);
	output.radius_ratio = ring.inner_radius / ring.outer_radius;
	/* output.position = vec4<f32>(0., 0., 0., 1.); */
	return output;
}

// Actual Circle is rendered in the fragment shader
@fragment
fn f_main(v2f: V2F) -> @location(0) vec4<f32> {
	let mag1 = clamp(dot(v2f.uv, v2f.uv) * -100 + 100, 0., 1.);
	let mag2 = clamp(dot(v2f.uv, v2f.uv) * 100 - 100 * v2f.radius_ratio, 0., 1.);

	let mag = min(mag1, mag2);
	
	return v2f.color * mag;
	/* return vec4<f32>(1.); */
}
