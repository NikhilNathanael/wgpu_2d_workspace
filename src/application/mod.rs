use std::sync::mpsc::{channel, Sender, SendError};
use std::sync::Arc;
use std::thread;

use wgpu::*;

use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowId};
use winit::event::{ElementState, WindowEvent};
use winit::dpi::PhysicalSize;
use winit::keyboard::{Key, NamedKey};

use super::wgpu_context::WGPUContext;
use super::input::key_map::KeyMap;



pub struct App{
	title: &'static str,
	key_map: KeyMap,
	inner: Option<AppInner>,
}

struct AppInner {
	window: Arc<Window>,
	render_context: WGPUContext,
}

impl App {
	pub fn new (title: &'static str) -> Self {
		Self {
			title,
			key_map: KeyMap::new(),
			inner: None,
		}
	}

	pub fn window(&self) -> &winit::window::Window {
		&*self.inner.as_ref().unwrap().window
	}

	pub fn render_context (&self) -> &WGPUContext {
		&self.inner.as_ref().unwrap().render_context
	}

	pub fn render_context_mut(&mut self) -> &mut WGPUContext {
		&mut self.inner.as_mut().unwrap().render_context
	}
}

impl winit::application::ApplicationHandler for App {
	fn resumed(&mut self, event_loop: &ActiveEventLoop) {
		match &self.inner {
			None => {
				let window = Arc::new(event_loop.create_window(
					Window::default_attributes()
						.with_title(self.title.to_owned())
					).expect("Could not create window"));
				let render_context = WGPUContext::new(Arc::clone(&window));
				let key_map = KeyMap::new();

				self.inner = Some(AppInner{
					window,
					render_context,
				});
			}	
			_ => (),
		}
	}

	fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
		match event {
			WindowEvent::CloseRequested => {
				println!("The close button was pressed; stopping");
				event_loop.exit();
			},
			WindowEvent::KeyboardInput{event, ..} => {
				match event.logical_key {
					Key::Named(NamedKey::Escape) => event_loop.exit(),
					x => self.key_map.handle_key(x, event.state),
				}
			}
			WindowEvent::Resized(new_size) => {
				self.render_context_mut().resize(new_size);
				self.window().request_redraw();
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
				let surface_texture = self.render_context().surface().get_current_texture()
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

				surface_texture.present();


				self.window().request_redraw();
			}
			_ => (),
		}
	}		
}

