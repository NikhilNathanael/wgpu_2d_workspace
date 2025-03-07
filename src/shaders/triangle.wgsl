struct V2F {
	@builtin (position) position: vec4<f32>,
	@location(0) actual_position: vec4<f32>,
};


@vertex 
fn v_main (@builtin (vertex_index) v_id: u32) -> V2F {
	let quad = array(
		vec2<f32>(-.5,  .5),
		vec2<f32>(-.5, -.5),
		vec2<f32>( .5,  .5),
		vec2<f32>( .5, -.5),
	);
	let position = vec4<f32>(quad[v_id], 0.0, 1.0);
	var output: V2F;
	output.position = position;
	output.actual_position = position;
	return output;
}

@fragment 
fn f_main (v2f: V2F) -> @location(0) vec4<f32> {
	return v2f.actual_position;
}
