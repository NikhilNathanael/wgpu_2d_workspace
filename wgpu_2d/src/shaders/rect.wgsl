#include<common.wgsl>
struct Rect {
	@location(0) color: vec4<f32>,
	@location(1) center: vec2<f32>,
	@location(2) size: vec2<f32>,
	@location(3) rotation: f32,
}

struct V2F {
	@builtin(position) position: vec4<f32>,
	@location(0) color: vec4<f32>,
}

@vertex 
fn v_main (rect: Rect, @builtin(vertex_index) v_id: u32) -> V2F {
	let rotation_matrix = mat2x2<f32> (
		vec2<f32>(cos(rect.rotation), -sin(rect.rotation)),
		vec2<f32>(sin(rect.rotation), cos(rect.rotation)),
	);
	let pos = quad_strip[v_id] * rect.size / 2. * rotation_matrix + rect.center;

	let clip_space = worldspace_to_clipspace(pos);

	var output: V2F;
	output.color = rect.color;
	output.position = vec4<f32>(clip_space, 0., 1.);
	/* output.position = vec4<f32>(0., 0., 0., 1.); */
	return output;
}

@fragment
fn f_main(v2f: V2F) -> @location(0) vec4<f32> {
	return v2f.color;
}
