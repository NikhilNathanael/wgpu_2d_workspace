pub struct App{
	title: &'static str,
	window: Option<winit::window::Window>,
	sender: std::sync::mpsc::Sender<()>,
}

impl App {
	pub fn new (title: &'static str) -> Self {
		let (snd, rcv) = std::sync::mpsc::channel();
		_ = std::thread::spawn(create_application_thread(rcv));
		Self {
			title,
			window: None,
			sender: snd,
		}
	}
}

use winit::event_loop::ActiveEventLoop;
use winit::window::WindowId;
use winit::event::WindowEvent;

impl winit::application::ApplicationHandler for App {
	fn resumed(&mut self, event_loop: &ActiveEventLoop) {
		match &self.window {
			None => self.window = Some(event_loop.create_window(
				winit::window::Window::default_attributes()
					.with_title(self.title.to_owned())
			).unwrap()),
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
				match event.logical_key {
					Key::Named(NamedKey::Escape) => event_loop.exit(),
					Key::Named(NamedKey::Space) => self.sender.send(()).unwrap(),
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
				// self.window.as_ref().unwrap().request_redraw();
			}
			_ => (),
		}
	}		
}

fn create_application_thread (rcv: std::sync::mpsc::Receiver<()>) -> impl FnOnce() {
	move || {
		while let Ok(()) = rcv.recv() {
			while let Ok(()) = rcv.try_recv() {}
			println!("{:?}", "hello");
		}
		println!("{:?}", "thread exiting");
	}
}
