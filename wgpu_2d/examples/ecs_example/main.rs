use my_ecs::ecs::commands::*;
use my_ecs::ecs::entity::*;
use my_ecs::ecs::query::*;
use my_ecs::ecs::resource::*;
use my_ecs::ecs::schedule::*;
use my_ecs::ecs::world::*;

use wgpu_2d::ecs::WindowPlugin;
use wgpu_2d::ecs::WinitWindow;
use wgpu_2d::input::KeyMap;
use wgpu_2d::input::MouseMap;
use wgpu_2d::math::Vector2;
use wgpu_2d::rendering::Circle;
use wgpu_2d::rendering::{CircleRenderer, Renderer2D};
use wgpu_2d::shader_manager::ShaderManager;
use wgpu_2d::wgpu_context::WGPUContext;
use wgpu_2d::wgpu_context::SHADER_DIRECTORY;
use winit::keyboard::Key;
use winit::keyboard::NamedKey;
use winit::window::Fullscreen;

pub fn main() {
    let mut world = World::new();
    world
        .add_plugin(WindowPlugin::new("Test App", SHADER_DIRECTORY))
        .add_system(Startup, spawn_character)
        .add_system(Startup, set_window_type)
        .add_system(Update, control_character)
        .add_system(Update, check_exit)
        .add_system(Render, render)
        .run();
}

struct Character(Vector2<f32>);
impl Component for Character {}

fn set_window_type (window: Res<WinitWindow>) {
	// window.0.set_fullscreen(Some(Fullscreen::Borderless(None)));
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
                color: [1., 1., 1., 1.],
                position: [400., 400.],
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
        renderer.circles_mut()[0].position = mouse_pos.into_inner();
        renderer.update_circles(&*context);
    }
}

fn render(
    mut renderer: ResMut<Renderer2D>,
    context: Res<WGPUContext>,
    shader_manager: Res<ShaderManager>,
    query: Query<&CircleRenderer, With<Character>>,
) {
    renderer.render(query.iter(), &*context, &*shader_manager);
}

fn check_exit(key_map: Res<KeyMap>, commands: Commands) {
    if key_map.is_pressed(Key::Named(NamedKey::Escape)) {
        commands.exit();
    }
}
