use std::sync::Arc;

use wgpu::*;

use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowId};
use winit::event::WindowEvent;
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
	scene: (PointRenderer, TriangleListRenderer, CircleRenderer),
	shader_manager: ShaderManager,
	timer: Timer,
	key_map: KeyMap,
}

impl AppInner {
	pub fn init(window: Window) -> Self {
		let window = Arc::new(window);

		// Create shader_manager
		let shader_manager = ShaderManager::new(SHADER_DIRECTORY);

		// Create key map
		let key_map = KeyMap::new();
		
		// Create WGPU context
		let render_context = WGPUContext::new(Arc::clone(&window));
		
		// Create Timer
		let timer = Timer::new();

		// Create scene
		//  - Points
		let points = create_circle_point_list(200, 50.,[50. , 400.]);
		let points = PointRenderer::new(points, &render_context, &shader_manager);

		//  - Triangle
		let triangle = vec![
			Triangle {
				points: [
					Point {
						position: [400., 200.],
						color: [1., 0., 0., 1.],
					},
					Point {
						position: [300., 400.],
						color: [0., 1., 0., 1.],
					},
					Point {
						position: [500., 400.],
						color: [0., 0., 1., 1.],
					},
				],
			}
		];
		let triangle = TriangleListRenderer::new(triangle, &render_context, &shader_manager);

		let mut rng = rand::rng();

		//  - Rectangles
		// let rects = (0..50).map(|_|
		// 	CenterRect{
		// 		color: [rng.random_range(0.0..1.0), rng.random_range(0.0..1.0), rng.random_range(0.0..1.0), 1.],
		// 		center: [
		// 			rng.random_range(0.0..1600.),
		// 			rng.random_range(0.0..1200.),
		// 		],
		// 		size: [rng.random_range(50.0..200.0), rng.random_range(50.0..200.0)],
		// 		rotation: rng.random_range(0.0..4.0),
		// 	}
		// ).collect();
		// let rects = RectangleRenderer::new(rects, &render_context, &shader_manager);

		//  - Circles
		let circles = vec![
			Circle {
				color: [rng.random_range(0.0..1.0), rng.random_range(0.0..1.0), rng.random_range(0.0..1.0), 1.],
				position: [
					0., 0.,
				],
				radius: 100.,
			}
		];
		let circles = CircleRenderer::new(circles, &render_context, &shader_manager);

		Self {
			window,
			render_context,
			scene: (points, triangle, circles),
			shader_manager,
			timer,
			key_map,
		}
	}

	pub fn render_frame(&mut self) {
		// log::trace!("Frame Delta: {}", self.timer.elapsed_reset());
		// self.timer.reset();
		self.update_scene();


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
		
		self.scene.0.render(&texture_view, &self.render_context, &self.shader_manager);
		self.scene.1.render(&texture_view, &self.render_context, &self.shader_manager);
		self.scene.2.render(&texture_view, &self.render_context, &self.shader_manager);
		
		surface_texture.present();
		self.window.request_redraw();
	}

	pub fn update_scene(&mut self) {
		let delta = self.timer.elapsed_reset();
		self.timer.reset();

		let mut move_dir = [0., 0.];

		if self.key_map.is_pressed(key_char!("w")) {move_dir[1] -= delta * 500.;}
		if self.key_map.is_pressed(key_char!("s")) {move_dir[1] += delta * 500.;}
		if self.key_map.is_pressed(key_char!("a")) {move_dir[0] -= delta * 500.;}
		if self.key_map.is_pressed(key_char!("d")) {move_dir[0] += delta * 500.;}

		if move_dir != [0., 0.] {
			self.scene.0.points_mut().iter_mut().for_each(|Point {position, ..}| {
				*position = [position[0] + move_dir[0], position[1] + move_dir[1]];
			});
			self.scene.0.update_points_buffer(&self.render_context);
		};

		// self.scene.2.rects_mut().iter_mut().for_each(|CenterRect{rotation, ..}| {
		// 	*rotation += delta;
		// });
		// self.scene.2.update_rects(&self.render_context);
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
		todo!();
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
					x => inner.key_map.handle_key(x, event.state),
				}
			}
			WindowEvent::Resized(new_size) => {
				inner.render_context.resize(new_size);
				inner.scene.0.update_size(&inner.render_context);
				inner.scene.1.set_uniform(&inner.render_context);
				inner.scene.2.set_uniform(&inner.render_context);
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
			WindowEvent::CursorMoved{position: mouse_position,..} => {
				inner.scene.2.circles_mut()
					.iter_mut()
					.for_each(|Circle{position,..}| 
						 *position = [mouse_position.x as f32, mouse_position.y as f32]
					);
				inner.scene.2.update_circles(&inner.render_context);
			}
			_ => (),
		}
	}		
}

