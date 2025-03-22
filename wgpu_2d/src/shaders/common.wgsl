
// Common data such as uniforms and bindings for them
// TODO: Move bindings to separate files

struct Uniform {
	screen_size: vec2<f32>,
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
