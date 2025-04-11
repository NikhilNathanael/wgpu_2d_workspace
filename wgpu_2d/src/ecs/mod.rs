use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;

use my_ecs::ecs::commands::*;
use my_ecs::ecs::entity::*;
use my_ecs::ecs::plugin::*;
use my_ecs::ecs::resource::*;
use my_ecs::ecs::schedule::*;

use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::platform::windows::EventLoopBuilderExtWindows;
use winit::window::{Window, WindowId};

use crate::input::{GamepadMap, KeyMap, MouseMap};
use crate::rendering::{CircleRenderer, PointRenderer, RectangleRenderer, Renderer2D, RingRenderer, TextureRenderer, TriangleListRenderer};
use crate::shader_manager::ShaderManager;
use crate::timer::Timer;
use crate::wgpu_context::WGPUContext;

// struct used by winit to run window
struct WinitApp {
	title: &'static str,
	initialized: bool,
	window_and_context_sender: Sender<(Arc<Window>, WGPUContext)>,
	window_event_sender: Sender<(WindowId, WindowEvent)>,
	close_reciever: Receiver<()>,
	window: Option<Arc<Window>>,
}

// Event loop for window. This is run in a separate thread
impl ApplicationHandler for WinitApp {
	fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
		if !self.initialized {
			// Create window
			let window = Arc::new(event_loop.create_window(
				Window::default_attributes()
				.with_title(self.title.to_owned())
				).expect("Could not create window"));

			let render_context = WGPUContext::new(Arc::clone(&window), [window.inner_size().width, window.inner_size().height]);
			self.window_and_context_sender.send((Arc::clone(&window), render_context)).unwrap();
			self.window = Some(window);
			self.initialized = true;
		}
	}

	fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
		if let Ok(_) = self.close_reciever.try_recv() {
			event_loop.exit();
		}
		self.window_event_sender.send((window_id, event)).unwrap();
	}
}

pub struct WindowPlugin {
	title: &'static str,
	shader_directory: &'static str,
}

impl WindowPlugin {
	pub fn new (title: &'static str, shader_directory: &'static str) -> Self{
		Self {
			title,
			shader_directory,
		}
	}
}

impl Plugin for WindowPlugin {
	fn build(&self, _world: &mut my_ecs::ecs::world::World) {
		// Oneshot channel to recieve Window and WGPUContext 
		let (window_tx, window_rx) = channel();
		// Window event reciever
		let (event_tx, event_rx) = channel();
		// Oneshot channel which closes window when anything is recieved
		let (close_tx, close_rx) = channel();
		let title = &*self.title;
		std::thread::spawn(move || {
			let event_loop = winit::event_loop::EventLoop::builder()
				.with_any_thread(true)
				.build()
				.expect("Could not create event loop");
			let mut app = WinitApp {
				title,
				initialized: false,
				window_and_context_sender: window_tx,
				window_event_sender: event_tx,
				close_reciever: close_rx,
				window: None,
			};
			_ = event_loop.run_app(&mut app);
		});

		// Window Event reciever
		_world.add_resource(WindowEvents(event_rx))
			.add_resource(ShaderManager::new(self.shader_directory))
			.add_resource(KeyMap::new())
			.add_resource(MouseMap::new())
			.add_resource(GamepadMap::new())
			.add_resource(Timer::new())
			.add_resource(WindowClose(close_tx))
			;

		let (window, render_context) = window_rx.recv().unwrap();
		let renderer_2d = Renderer2D::new(&render_context);
		
		_world.add_resource(WinitWindow(window))
			.add_resource(render_context)
			.add_resource(renderer_2d)
			;

		_world.add_system(Update, handle_window_events);
	}
}

// A channel receiver is a foreign type so Resource cannot 
// be directly implemented for it
struct WindowEvents(Receiver<(WindowId, WindowEvent)>);
impl Resource for WindowEvents {}

// WindowClose transmitter
struct WindowClose(Sender<()>);
impl Resource for WindowClose {}

impl Resource for ShaderManager {}

impl Resource for KeyMap {}
impl Resource for MouseMap {}
impl Resource for GamepadMap {}

impl Resource for Timer {}

impl Resource for Renderer2D {}
impl Resource for WGPUContext {}

// Window from winit is a foriegn type so Resource cannot 
// be implemented for it directly

pub struct WinitWindow(pub Arc<Window>);
impl Resource for WinitWindow {}

impl Component for CircleRenderer {}
impl Component for PointRenderer {}
impl Component for TriangleListRenderer {}
impl Component for RingRenderer {}
impl Component for TextureRenderer {}
impl Component for RectangleRenderer {}

// System that handles window events from the window thread
fn handle_window_events(
	window_events: Res<WindowEvents>, 
	window: Res<WinitWindow>, 
	mut key_map: ResMut<KeyMap>,
	mut mouse_map: ResMut<MouseMap>,
	commands: Commands,

) {
	for (_window_id, event) in window_events.0.try_iter().filter(|(id,_)| *id == window.0.id()) {
		match event {
			WindowEvent::KeyboardInput{event,..} => {
				key_map.handle_key(event.logical_key, event.state);
			}
			WindowEvent::MouseInput{state, button,..} => {
				mouse_map.handle_button(button, state);
			}
			WindowEvent::CursorMoved{position,..} => {
				mouse_map.handle_cursor_movement(position);
			}
			WindowEvent::CloseRequested => {
				commands.exit();
			}
			_ => ()
		}
	}
}
