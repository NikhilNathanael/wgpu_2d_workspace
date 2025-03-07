@vertex 
fn v_main (@builtin (vertex_index) v_id: u32) -> @builtin (position) vec4<f32> {
	let array = array(
		vec2<f32>(-.5,  .5),
		vec2<f32>(-.5, -.5),
		vec2<f32>( .5,  .5),
		vec2<f32>( .5, -.5),
	);
	return vec4<f32>(array[v_id], 0.0, 1.0);
}

@fragment 
fn f_main (@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
	return position;
}
