
// Common data such as uniforms and bindings for them
// TODO: Move bindings to separate files

struct Uniform {
	screen_size: vec2<f32>,
	view_port_origin: vec2<f32>,
}

// Requires uniform binding
fn worldspace_to_clipspace (input: vec2<f32>) -> vec2<f32> {
	return (input - uni.view_port_origin) / uni.screen_size * vec2<f32>(2., -2) + vec2<f32>(-1, 1.);
}

@group(0) @binding(0) var<uniform> uni: Uniform;

const quad_strip = array(
	vec2<f32>(-1., -1.),
	vec2<f32>( 1., -1.),
	vec2<f32>(-1.,  1.),
	vec2<f32>( 1.,  1.),
);

const triangle = array(
	vec2<f32>(   0., 0.5),
	vec2<f32>(-0.5, -0.5),
	vec2<f32>( 0.5, -0.5),
);
