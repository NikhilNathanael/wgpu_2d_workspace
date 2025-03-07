use std::sync::mpsc::{channel, Sender, SendError};
use std::sync::Arc;
use std::thread;

use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowId};
use winit::event::WindowEvent;

use super::wgpu_context::WGPUContext;
use super::input::key_map::KeyMap;

use winit::keyboard::{Key, NamedKey};
use winit::event::ElementState;

use worker_thread::create_worker_thread;

pub struct App{
	title: &'static str,
	key_map: KeyMap,
	inner: Option<AppInner>,
}

struct AppInner {
	window: Arc<Window>,
	sender: Sender<()>,
	render_context: Arc<WGPUContext>,
}

impl App {
	pub fn new (title: &'static str) -> Self {
		Self {
			title,
			inner: None,
			key_map: KeyMap::new(),
		}
	}

	pub fn window(&self) -> &winit::window::Window {
		&*self.inner.as_ref().unwrap().window
	}

	pub fn render_context (&self) -> &WGPUContext {
		&*self.inner.as_ref().unwrap().render_context
	}

	pub fn send (&self) -> Result<(), SendError<()>> {
		self.inner.as_ref().unwrap().sender.send(())
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
				let render_context = Arc::new(WGPUContext::new(Arc::clone(&window)));
				let (sender, rcv) = channel();
				let key_map_send = sender.clone();

				self.key_map.register_callback(
					Key::Named(NamedKey::Space), 
					ElementState::Pressed,
					move || {_ = key_map_send.send(());}, 
				);

				thread::spawn(create_worker_thread(rcv, Arc::clone(&render_context)));
				self.inner = Some(AppInner{
					window,
					sender,
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
				self.key_map.handle_key(event.logical_key.clone(), event.state);
				match event.logical_key {
					Key::Named(NamedKey::Escape) => event_loop.exit(),
					// Key::Named(NamedKey::Space) => self.send().unwrap(),
					_ => (),
				}
			}
			WindowEvent::Resized(new_size) => {
				self.render_context().resize(new_size);
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
				self.send().unwrap();
				self.window().request_redraw();
			}
			_ => (),
		}
	}		
}

mod worker_thread {
	use super::super::wgpu_context::WGPUContext;
	use wgpu::*;
	use std::sync::Arc;
	use std::sync::mpsc::Receiver;

	use std::borrow::Cow;

	const SHADER_DIRECTORY: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/src/shaders/");

	pub fn create_worker_thread (rcv: Receiver<()>, context: Arc<WGPUContext>) -> impl FnOnce() {
		let shader_path = SHADER_DIRECTORY.to_owned() + "triangle.wgsl";

		println!("{:?}", shader_path);
		let shader_source = std::fs::read_to_string(&shader_path)
			.expect("Could not read file");

		let shader_module = context.device().create_shader_module(ShaderModuleDescriptor{
			label: Some(&shader_path),
			source: ShaderSource::Wgsl(Cow::Borrowed(&shader_source)),
		});

		let render_pipeline = context.device().create_render_pipeline(&RenderPipelineDescriptor{
			label: Some("render pipeline"),
			layout: None,
			vertex: VertexState{
				module: &shader_module,
				entry_point: None,
				compilation_options: Default::default(),
				buffers: &[],
			},
			fragment: Some(FragmentState{
				module: &shader_module,
				entry_point: None,
				compilation_options: Default::default(),
				targets: &[
					Some(ColorTargetState{
						format: context.config().format,
						blend: None,
						write_mask: ColorWrites::ALL,
					})
				],
			}),
			primitive: PrimitiveState {
				topology: PrimitiveTopology::TriangleStrip,
				strip_index_format: None,
				front_face: FrontFace::Ccw,
				cull_mode: None,
				..Default::default()
			},
			depth_stencil: None,
			multisample: Default::default(),
			multiview: None,
			cache: None,
		});

		move || {
			// wait until main thread sends signal to start rendering
			while let Ok(()) = rcv.recv() {
				// consume all sent signals if rendering took too long
				rcv.try_iter().for_each(|_| {});

				let surface_texture = context.surface().get_current_texture()
					.expect("Could not create surface texture");
				let current_texture = &surface_texture.texture;
				let texture_view = current_texture.create_view(&TextureViewDescriptor{
					label: Some("Render Texture"),
					format: Some(current_texture.format()),
					dimension: Some(TextureViewDimension::D2),
					usage: Some(TextureUsages::RENDER_ATTACHMENT),
					aspect: TextureAspect::All,
					base_mip_level: 0,
					mip_level_count: None,
					base_array_layer: 0,
					array_layer_count: None,
				});

				let mut command_encoder = context.device().create_command_encoder(&CommandEncoderDescriptor{
					label: Some("Command Encoder"),
				});

				let mut render_pass = command_encoder.begin_render_pass(&RenderPassDescriptor{
					label: Some("Render pass"),
					color_attachments: &[
						Some(RenderPassColorAttachment{
							view: &texture_view,
							resolve_target: None,
							ops: Operations {
								load: LoadOp::Clear(Color{r: 1., g: 0., b:1., a:1.}),
								store: StoreOp::Store,
							}
						}),
					],
					..Default::default()
				});
				render_pass.set_pipeline(&render_pipeline);
				render_pass.draw(0..4, 0..1);
				std::mem::drop(render_pass);
				context.queue().submit([command_encoder.finish()]);
				surface_texture.present();
				
				println!("{}", "hello");
			}
			println!("{}", "thread exiting");
		}
	}
}
