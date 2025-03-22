//#![windows_subsystem = "windows"]
mod rendering;
mod shader_manager;
mod wgpu_context;
mod input;
mod timer;

mod application;
use application::*;

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
