mod rendering;

mod application;
use application::*;

mod wgpu_context;

mod input;

fn main () {
	init_logger();
	let event_loop = winit::event_loop::EventLoop::new()
		.expect("Could not create event loop");
	let mut app = App::new("My Application");
	_ = event_loop.run_app(&mut app);
}

fn init_logger() {
	simple_logger::SimpleLogger::new()
		.with_level(log::LevelFilter::Off)
		.with_module_level("wgpu_2d", log::LevelFilter::Trace)
		.init()
		.unwrap();
}
