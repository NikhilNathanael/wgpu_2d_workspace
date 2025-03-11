use std::sync::mpsc::{channel, Sender, SendError};
use std::sync::Arc;
use std::thread;

use wgpu::*;

use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowId};
use winit::event::{ElementState, WindowEvent};
use winit::dpi::PhysicalSize;
use winit::keyboard::{Key, NamedKey};

use super::wgpu_context::*;
use super::input::KeyMap;
use super::shader_manager::*;

use crate::rendering::{PointRenderer, Point, create_circle_point_list};
use crate::timer::Timer;

use rand::Rng;

pub struct App{
	title: &'static str,
	key_map: KeyMap,
	inner: Option<AppInner>,
}

impl App {
	pub fn new (title: &'static str) -> Self {
		Self {
			title,
			key_map: KeyMap::new(),
			inner: None,
		}
	}
	
	pub fn init_inner(&mut self, event_loop: &ActiveEventLoop) {
		// Create window
		let window = Arc::new(event_loop.create_window(
			Window::default_attributes()
				.with_title(self.title.to_owned())
			).expect("Could not create window"));

		// Create shader_manager
		let shader_manager = ShaderManager::new(SHADER_DIRECTORY);
		
		// Create WGPU context
		let render_context = WGPUContext::new(Arc::clone(&window));
		
		// Create Timer
		let timer = Timer::new();

		// Create scene
		let points = create_circle_point_list(200, 50.,[50. , 400.]);
		let scene = PointRenderer::new(points, &render_context, &shader_manager);

		self.inner = Some(AppInner{
			window,
			render_context,
			scene,
			shader_manager,
			timer,
		});
	}
}

struct AppInner {
	window: Arc<Window>,
	render_context: WGPUContext,
	scene: PointRenderer,
	shader_manager: ShaderManager,
	timer: Timer,
}

impl AppInner {
	pub fn render_frame(&mut self) {
		let time = self.timer.elapsed_start();
		let delta = self.timer.elapsed_reset();
		log::trace!("Frame Delta: {}", self.timer.elapsed_reset());
		self.timer.reset();

		let surface_texture = self.render_context.surface().get_current_texture()
			.expect("Could not get current texture");

		let texture_view = surface_texture.texture.create_view(&TextureViewDescriptor{
			label: Some("Render Texture"),
			format: Some(surface_texture.texture.format()),
			dimension: Some(TextureViewDimension::D2),
			usage: Some(TextureUsages::RENDER_ATTACHMENT),
			aspect: TextureAspect::All,
			base_mip_level: 0,
			mip_level_count: None,
			base_array_layer: 0,
			array_layer_count: None,
		});

		let mut rng = rand::thread_rng();
		self.scene.update_time(time);
		self.scene.points_mut().iter_mut().enumerate().for_each(|(i, Point {color, position})| {
			*position = [
				400. + (i as f32 / 200. * 2. * std::f32::consts::PI).sin() * 100. + time.sin() * 200.,
				300. + (i as f32 / 200. * 2. * std::f32::consts::PI).cos() * 100. + time.cos() * 200.,
			];
		});
		self.scene.update_points_buffer(&self.render_context);
		
		self.scene.render(&texture_view, &self.render_context, &self.shader_manager);
		
		surface_texture.present();
		self.window.request_redraw();
	}
}

impl winit::application::ApplicationHandler for App {
	fn resumed(&mut self, event_loop: &ActiveEventLoop) {
		match &self.inner {
			None => {
				self.init_inner(event_loop);
			}	
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
					x => self.key_map.handle_key(x, event.state),
				}
			}
			WindowEvent::Resized(new_size) => {
				inner.render_context.resize(new_size);
				inner.scene.update_size(&inner.render_context);
				inner.window.request_redraw();
			},
			WindowEvent::RedrawRequested => {
				// Redraw the application.
				//
				// It's preferable for applications that do not render continuously to render in
				// this event rather than in AboutToWait, since rendering in here allows
				// the program to gracefully handle redraws requested by the OS.

				// Draw.

				// Queue a RedrawRequested event.
				//
				// You only need to call this if you've determined that you need to redraw in
				// applications which do not always need to. Applications that redraw continuously
				// can render here instead.
				inner.render_frame();
			}
			_ => (),
		}
	}		
}

