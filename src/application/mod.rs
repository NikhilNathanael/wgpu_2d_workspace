use std::sync::mpsc::{channel, Sender, Receiver, SendError};
use std::sync::Arc;
use std::thread;

use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowId};
use winit::event::WindowEvent;

use super::wgpu_context::WGPUContext;
use super::input::key_map::KeyMap;

use winit::keyboard::{Key, NamedKey};
use winit::event::ElementState;

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

				thread::spawn(create_application_thread(rcv, Arc::clone(&render_context)));
				self.inner = Some(AppInner{
					window,
					sender,
					render_context,
				});
			}	
			_ => (),
		}

	}

	fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
		match event {
			WindowEvent::CloseRequested => {
				println!("The close button was pressed; stopping");
				event_loop.exit();
			},
			WindowEvent::KeyboardInput{event, ..} => {
				use winit::keyboard::Key;
				use winit::keyboard::NamedKey;
				self.key_map.handle_key(event.logical_key.clone(), event.state);
				match event.logical_key {
					Key::Named(NamedKey::Escape) => event_loop.exit(),
					// Key::Named(NamedKey::Space) => self.send().unwrap(),
					_ => (),
				}
			}
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
				self.window().request_redraw();
			}
			_ => (),
		}
	}		
}

fn create_application_thread (rcv: Receiver<()>, context: Arc<WGPUContext>) -> impl FnOnce() {
	move || {
		// wait until main thread sends signal to start rendering
		while let Ok(()) = rcv.recv() {
			// consume all sent signals if rendering took too long
			while let Ok(()) = rcv.try_recv() {}
			println!("{}", "hello");
		}
		println!("{}", "thread exiting");
	}
}
