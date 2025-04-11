use my_ecs::ecs::commands::*;
use my_ecs::ecs::entity::*;
use my_ecs::ecs::query::*;
use my_ecs::ecs::resource::*;
use my_ecs::ecs::schedule::*;
use my_ecs::ecs::world::*;

use wgpu_2d::ecs::WindowPlugin;
use wgpu_2d::input::KeyMap;
use wgpu_2d::math::Vector2;
use wgpu_2d::rendering::Circle;
use wgpu_2d::rendering::{CircleRenderer, Renderer2D};
use wgpu_2d::shader_manager::ShaderManager;
use wgpu_2d::wgpu_context::WGPUContext;
use wgpu_2d::wgpu_context::SHADER_DIRECTORY;
use winit::keyboard::Key;
use winit::keyboard::NamedKey;

pub fn main() {
    let mut world = World::new();
    world
        .add_plugin(WindowPlugin::new("Test App", SHADER_DIRECTORY))
        .add_system(Startup, spawn_character)
        .add_system(Update, render)
        .add_system(Update, exit)
        .run();
}

struct Character(Vector2<f32>);
impl Component for Character {}

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

fn render(
    mut renderer: ResMut<Renderer2D>,
    context: Res<WGPUContext>,
    shader_manager: Res<ShaderManager>,
    query: Query<&CircleRenderer, With<Character>>,
) {
    renderer.render([query.iter().next().unwrap()], &*context, &*shader_manager);
}

fn exit(key_map: Res<KeyMap>, commands: Commands) {
    if key_map.is_pressed(Key::Named(NamedKey::Space)) {
        commands.exit();
    }
}
