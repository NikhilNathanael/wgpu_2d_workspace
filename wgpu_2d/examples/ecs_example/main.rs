use my_ecs::ecs::commands::*;
use my_ecs::ecs::entity::*;
use my_ecs::ecs::query::*;
use my_ecs::ecs::resource::*;
use my_ecs::ecs::schedule::*;
use my_ecs::ecs::world::*;

use wgpu_2d::ecs::{WindowPlugin, WinitWindow};
use wgpu_2d::input::{KeyMap, MouseMap};
use wgpu_2d::key_char;
use wgpu_2d::math::{Vector2, Vector4};
use wgpu_2d::rendering::Circle;
use wgpu_2d::rendering::{CircleRenderer, Renderer2D};
use wgpu_2d::shader_manager::ShaderManager;
use wgpu_2d::timer::Timer;
use wgpu_2d::wgpu_context::WGPUContext;

use winit::keyboard::{Key, NamedKey};

pub fn main() {
    let mut world = World::new();
    world
		.add_resource(ShaderManager::new(""))
        .add_plugin(WindowPlugin::new("Test App"))
        .add_system(Startup, (spawn_character, set_window_type))
		.add_system(PreUpdate, reset_timer)
        .add_system(Update, (control_character, check_exit, control_position))
        .add_system(Render, render)
        .run();
}

struct Character(Vector2<f32>);
impl Component for Character {}

fn set_window_type (window: Res<WinitWindow>) {
	// window.0.set_fullscreen(Some(Fullscreen::Borderless(None)));
	window.0.set_cursor_visible(false);
}

fn spawn_character(
    commands: Commands,
    renderer_2d: Res<Renderer2D>,
    context: Res<WGPUContext>,
    shader_manager: Res<ShaderManager>,
) {
    commands.spawn_entity((
        Character(Vector2::<f32>::new([400., 400.])),
        CircleRenderer::new(
            vec![Circle {
                color: Vector4::new([1., 1., 1., 1.]),
				position: Vector2::new([context.config().width as f32 / 2., context.config().height as f32 / 2.]),
				radius: 100.,
            }],
            &renderer_2d.uniform_bind_group_layout(),
            &*context,
            &*shader_manager,
        ),
    ));
}

fn control_character(
    mut query: Query<(&mut Character, &mut CircleRenderer)>,
    mouse_map: Res<MouseMap>,
    context: Res<WGPUContext>,
) {
    let mouse_pos = mouse_map.mouse_position();
    for (chara, renderer) in query.iter_mut() {
        chara.0 = mouse_pos;
		renderer.circles_mut()[0].position = mouse_pos;
		renderer.update_circles(&*context);
    }
}

fn control_position(
	mut renderer: ResMut<Renderer2D>,
	context: Res<WGPUContext>,
	key_map: Res<KeyMap>,
	timer: Res<Timer>,
) {
	const SPEED: f32 = 1000.;
	let time_delta = timer.elapsed_reset();

	let mut delta = renderer.get_uniform().view_port_origin;

	if key_map.is_pressed(key_char!("w")) {delta[1] -= SPEED * time_delta;}
	if key_map.is_pressed(key_char!("s")) {delta[1] += SPEED * time_delta;}
	if key_map.is_pressed(key_char!("a")) {delta[0] -= SPEED * time_delta;}
	if key_map.is_pressed(key_char!("d")) {delta[0] += SPEED * time_delta;}

	renderer.get_uniform().view_port_origin = delta;
	renderer.update_uniform(&*context);
}

fn reset_timer(mut timer: ResMut<Timer>) {
	timer.reset();
}

fn render(
    mut renderer: ResMut<Renderer2D>,
    context: Res<WGPUContext>,
    shader_manager: Res<ShaderManager>,
    query: Query<&CircleRenderer, With<Character>>,
) {
	// if screen_size is out of date, then update the uniform
	if [context.config().width as f32, context.config().height as f32] != *renderer.get_uniform().screen_size {
		*renderer.get_uniform().screen_size = [context.config().width as f32, context.config().height as f32];
		renderer.update_uniform(&*context);
	}
    renderer.render(query.iter(), &*context, &*shader_manager);
}

fn check_exit(key_map: Res<KeyMap>, commands: Commands, mut shader_manager: ResMut<ShaderManager>) {
    if key_map.is_pressed(Key::Named(NamedKey::Space)) {
		shader_manager.reload();
    }
    if key_map.is_pressed(Key::Named(NamedKey::Escape)) {
        commands.exit();
    }
}
