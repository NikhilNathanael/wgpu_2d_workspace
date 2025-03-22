use std::sync::Arc;

use mouse_map::MouseMap;
use wgpu::*;

use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowId};
use winit::event::{DeviceEvent, WindowEvent};
use winit::keyboard::{Key, NamedKey};

use super::wgpu_context::*;
use super::input::*;
use super::shader_manager::*;

use crate::rendering::*;
use crate::timer::Timer;

use crate::key_char;

use rand::Rng;

pub struct App{
	title: &'static str,
	inner: Option<AppInner>,
}

impl App {
	pub fn new (title: &'static str) -> Self {
		Self {
			title,
			inner: None,
		}
	}
}

struct AppInner {
	window: Arc<Window>,
	render_context: WGPUContext,
	shader_manager: ShaderManager,
	scene_manager: SceneManager,
	timer: Timer,
	input: Input,
}

impl AppInner {
	pub fn init(window: Window) -> Self {
		let window = Arc::new(window);

		// Create shader_manager
		let shader_manager = ShaderManager::new(SHADER_DIRECTORY);

		// Create key map
		let input = Input::new();
		
		// Create WGPU context
		let render_context = WGPUContext::new(Arc::clone(&window));
		
		// Create Timer
		let timer = Timer::new();

		Self {
			window,
			scene_manager: SceneManager::new(&render_context, &shader_manager),
			render_context,
			shader_manager,
			timer,
			input,
		}
	}

	pub fn update_scene(&mut self) {
		let delta = self.timer.elapsed_reset();
		self.timer.reset();

		let mut move_dir = [0., 0.];

		if self.input.key_map.is_pressed(key_char!("w")) {move_dir[1] -= delta * 500.;}
		if self.input.key_map.is_pressed(key_char!("s")) {move_dir[1] += delta * 500.;}
		if self.input.key_map.is_pressed(key_char!("a")) {move_dir[0] -= delta * 500.;}
		if self.input.key_map.is_pressed(key_char!("d")) {move_dir[0] += delta * 500.;}

		let scene = self.scene_manager.get_scene_mut();

		if move_dir != [0., 0.] {
			scene.0.points_mut().iter_mut().for_each(|Point {position, ..}| {
				*position = [position[0] + move_dir[0], position[1] + move_dir[1]];
			});
			scene.0.update_points_buffer(&self.render_context);
		};

		scene.2.circles_mut()
			.iter_mut()
			.for_each(|Circle{position,..}| {
				let scroll = self.input.mouse_map.scroll_level();
				let mouse_position = self.input.mouse_map.mouse_position();
				*position = [(mouse_position[0] + scroll[0]) as f32, (mouse_position[1] + scroll[1]) as f32];
			});
		scene.2.update_circles(&self.render_context);
		scene.3.rect_mut().rotation = self.input.mouse_map.scroll_level()[1] as f32;
		scene.3.update_rect(&self.render_context);
	}
}

impl winit::application::ApplicationHandler for App {
	fn resumed(&mut self, event_loop: &ActiveEventLoop) {
		match &self.inner {
			None => {
				// Create window
				let window = event_loop.create_window(
					Window::default_attributes()
					.with_title(self.title.to_owned())
					).expect("Could not create window");
				self.inner = Some(AppInner::init(window));
			}	
			_ => (),
		}
	}

	fn device_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        device_id: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
		let inner = self.inner.as_mut().unwrap();
		match event {
			DeviceEvent::MouseMotion{delta} => inner.input.mouse_map.handle_raw_mouse_movement(delta),
			DeviceEvent::MouseWheel{delta} => inner.input.mouse_map.handle_raw_scroll(delta),
			_ => (),
		}
	}

	fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
		let inner = self.inner.as_mut().unwrap();
		match event {
			WindowEvent::CloseRequested => {
				println!("The close button was pressed; stopping");
				event_loop.exit();
			},
			WindowEvent::KeyboardInput{event, ..} => {
				match event.logical_key {
					Key::Named(NamedKey::Escape) => event_loop.exit(),
					Key::Named(NamedKey::Space) => inner.shader_manager.reload(),
					x => inner.input.key_map.handle_key(x, event.state),
				}
			}
			WindowEvent::CursorMoved{position,..} => {
				inner.input.mouse_map.handle_cursor_movement(position);
			}
			WindowEvent::MouseWheel{delta,..} => {
				inner.input.mouse_map.handle_mouse_scroll(delta);
			}
			WindowEvent::MouseInput{button, state, ..} => {
				inner.input.mouse_map.handle_button(button, state);
			}
			WindowEvent::Resized(new_size) => {
				inner.render_context.resize(new_size);
				inner.scene_manager.update_uniform(&inner.render_context);
				inner.window.request_redraw();
			},
			WindowEvent::RedrawRequested => {
				inner.update_scene();
				inner.scene_manager.render_all(&inner.render_context, &inner.shader_manager);
				inner.window.request_redraw();
			}
			_ => (),
		}
	}		
}

struct Input {
	key_map: KeyMap,
	mouse_map: MouseMap,
}

impl Input {
	pub fn new () -> Self{
		Self {
			key_map: KeyMap::new(),
			mouse_map: MouseMap::new(),
		}
	}
}

