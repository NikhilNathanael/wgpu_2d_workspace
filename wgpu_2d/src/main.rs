//#![windows_subsystem = "windows"]
pub mod rendering;
pub mod shader_manager;
pub mod wgpu_context;
pub mod input;
pub mod timer;
pub mod math;
pub mod system;
use system::*;

pub mod application;
use application::*;

fn main () {
	let mut scheduler = Scheduler::new();
	scheduler.add_resource(25_i32);
	scheduler.run_schedule(Schedule::Startup);

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

// (Finished) : Timer struct
// 		- total time since start
// 		- time since last reset (usually last frame)
// 		- integer based (No floating point precision issues)
// (Finished) : Shader Compilation Manager
// 		- Automatically reads and compiles shaders in shader directory
// 		- Hot-reloading
// 		- caches pipelines as well as modules
// (Finished) : Add Include files to shader manager
// 		- read the shader input and check for any `#include "<file_name>"` fragments
// 		- read the file indicated by that name and include it in that location
// (Finished) : Add Transparency blending
// (Finished) : Add derive macros for Buffer data
// 		- One macro for Vertex data
// 		- One macro for Uniform data
// TODO : Unify the renderers for each type of primitive (point, triangle, 
//        center_rect and circle for now) into a single struct with a generic parameter
//      - Define a trait for each type of primitive
//      	- This trait should include
//      		- registering shaders
//      		- registering pipelines
//      		- creation of bind group layout
//      		- creation of bind groups
