mod rendering;

mod application;
use application::*;

mod wgpu_context;

mod input;

fn main () {
	let event_loop = winit::event_loop::EventLoop::new()
		.expect("Could not create event loop");
	let mut app = App::new("My Application");
	_ = event_loop.run_app(&mut app);
}

