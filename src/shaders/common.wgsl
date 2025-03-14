
struct Uniform {
	screen_size: vec2<f32>,
}

@group(0) @binding(0) var<uniform> uni: Uniform;

const quad = array(
	vec2<f32>(-1., -1.),
	vec2<f32>( 1., -1.),
	vec2<f32>(-1.,  1.),
	vec2<f32>( 1.,  1.),
);
