pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

pub mod application;
pub mod input;
pub mod rendering;
pub mod shader_manager;
pub mod wgpu_context;
pub mod timer;
pub mod math;
pub mod system;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}

// TODO: Finish Gamepad map and gamepad aim-assist
// TODO: Remove winit as dependancy of lib and make users directly 
//       use winit instead
