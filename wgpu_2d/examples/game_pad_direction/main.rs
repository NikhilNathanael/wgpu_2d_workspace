
use wgpu_2d::*;

mod application;
use application::App;

fn main () {
	simple_logger::SimpleLogger::new()
		.with_level(log::LevelFilter::Off)
		.with_module_level("wgpu_2d", log::LevelFilter::Info)
		.init()
		.unwrap();

	let event_loop = winit::event_loop::EventLoop::new()
		.expect("Could not create event loop");

	let mut app = App::new("My Application");
	_ = event_loop.run_app(&mut app);
}
