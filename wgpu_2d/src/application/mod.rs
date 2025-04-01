use std::sync::Arc;

use crate::input::*;

use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowId};
use winit::event::{DeviceEvent, WindowEvent};
use winit::keyboard::{Key, NamedKey};

use crate::wgpu_context::*;
use crate::shader_manager::*;

use crate::rendering::*;
use crate::timer::Timer;
use crate::math::{Vector2, Vector4};

use crate::key_char;

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
	renderer: Renderer2D,
	timer: Timer,
	input: Input,
	scene: (RingRenderer, RectangleRenderer),
}

impl AppInner {
	pub fn init(window: Window) -> Self {
		let window = Arc::new(window);

		// Create shader_manager
		let shader_manager = ShaderManager::new(SHADER_DIRECTORY);

		// Create input manager
		let input = Input::new();
		
		// Create WGPU context
		let render_context = WGPUContext::new(Arc::clone(&window), [window.inner_size().width, window.inner_size().height]);
		
		// Create Timer
		let timer = Timer::new();
		
		// Create Renderer
		let renderer = Renderer2D::new(&render_context);

		// Create scene
		//  - Ring
		let center = Vector2::new([render_context.config().width as f32 / 2., render_context.config().height as f32 / 2.]);
		const RADIUS: f32 = 200.;
		let rings = vec![
			Ring {
				color: Vector4::new([1., 1., 1., 1.]),
				position: center,
				outer_radius: RADIUS,
				inner_radius: RADIUS * 0.9, 
			}
		];
		let rings = RingRenderer::new(rings, renderer.uniform_bind_group_layout(), &render_context, &shader_manager);

		// - Aim Bar
		const START_ANGLE: f32 = - std::f32::consts::PI / 2.;
		let rects = vec![
			CenterRect{
				color : Vector4::new([1., 1., 1., 1.]),
				center : center + Vector2::rotation(START_ANGLE) * RADIUS / 2. * 0.98,
				size : Vector2::new([RADIUS * 0.95, 10.]),
				rotation : START_ANGLE,
			}
		];
		let rects = RectangleRenderer::new(rects, renderer.uniform_bind_group_layout(), &render_context, &shader_manager);

		Self {
			window,
			scene: (rings, rects),
			renderer,
			render_context,
			shader_manager,
			timer,
			input,
		}
	}

	pub fn update_scene(&mut self) {
		let delta = self.timer.elapsed_reset();
		self.timer.reset();
		
		let center = Vector2::new([self.render_context.config().width as f32 / 2., self.render_context.config().height as f32 / 2.]);

		let cursor_pos = self.input.mouse_map.mouse_position();
		let len = (cursor_pos - center).mag().min(200.);
		let angle = (cursor_pos - center).angle();
		// println!("{:?}", len);

		self.scene.0.rings_mut()[0].position = center;

		self.scene.1.rects_mut()[0].center = center + (Vector2::rotation(angle) * len) / 2. * 0.98;
		self.scene.1.rects_mut()[0].size[0] = len;
		self.scene.1.rects_mut()[0].rotation = angle;

		self.scene.0.update_rings(&self.render_context);
		self.scene.1.update_rects(&self.render_context);
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

	fn device_event(&mut self, _event_loop: &ActiveEventLoop, _device_id: winit::event::DeviceId, event: winit::event::DeviceEvent) {
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
			WindowEvent::MouseWheel{delta, ..} => {
				inner.input.mouse_map.handle_mouse_scroll(delta);
			}
			WindowEvent::MouseInput{button, state, ..} => {
				inner.input.mouse_map.handle_button(button, state);
			}
			WindowEvent::Resized(new_size) => {
				// inner.render_context.resize(winit::dpi::PhysicalSize::new(8, 8));
				inner.render_context.resize([new_size.width, new_size.height]);
				inner.renderer.update_uniform(&inner.render_context);
				inner.window.request_redraw();
			},
			WindowEvent::RedrawRequested => {
				inner.input.gamepad_map.update();
				inner.update_scene();
				inner.renderer.render(
					[&mut inner.scene.1 as &mut dyn Render, &mut inner.scene.0 as &mut dyn Render], 
					&inner.render_context, &inner.shader_manager
				);
				inner.window.request_redraw();
			}
			_ => (),
		}
	}		
}

struct Input {
	key_map: KeyMap,
	mouse_map: MouseMap,
	gamepad_map: GamepadMap
}

impl Input {
	pub fn new () -> Self{
		Self {
			key_map: KeyMap::new(),
			mouse_map: MouseMap::new(),
			gamepad_map: GamepadMap::new(),
		}
	}
}
