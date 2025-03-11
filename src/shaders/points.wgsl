struct Point {
	@location(0) color: vec4<f32>,
	@location(1) position: vec2<f32>,
}

struct V2F {
	@builtin(position) position: vec4<f32>,
	@location(0) color: vec4<f32>,
}

struct Uniform {
	size: vec2<f32>,
	time: f32,
}

@group(0) @binding(0) var<uniform> uni: Uniform;

@vertex
fn v_main(point: Point) -> V2F {
	let position_time = (point.position + vec2<f32>(uni.time * 500., 0.)) % (uni.size * 2);
	let clip_space = point.position / (uni.size) * vec2<f32>(1., -1.) + vec2<f32>(-1., 1.);

	var output: V2F;
	output.color = point.color;
	output.position = vec4<f32>(clip_space, 0., 1.);
	/* output.position = vec4<f32>(0., 0., 0., 1.); */
	return output;
}

@fragment 
fn f_main(v2f: V2F) -> @location(0) vec4<f32> {
	return v2f.color;
}
