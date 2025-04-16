pub use winit;

pub mod input;
pub mod math;
pub mod rendering;
pub mod shader_manager;
pub mod timer;
pub mod wgpu_context;

#[cfg(feature = "ecs")]
pub mod ecs;
#[cfg(feature = "ecs")]
pub use my_ecs;

mod maybe_thread;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}

// (Finished): Finish Gamepad map and gamepad aim-assist
// TODO: Remove winit as dependancy of lib and make users directly
//       use winit instead
//      - Move input handling to separate crate with winit as a dependency
//
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
// TODO (Changed) : Unify the renderers for each type of primitive (point, triangle,
//        center_rect and circle for now) into a single struct with a generic parameter
//      - Define a trait for each type of primitive
//      	- This trait should include
//      		- registering shaders
//      		- registering pipelines
//      		- creation of bind group layout
//      		- creation of bind groups
