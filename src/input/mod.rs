pub mod key_map {
    use std::collections::{HashMap, HashSet};

    use winit::event::ElementState;
	use winit::keyboard::Key;

	#[macro_export]
	macro_rules! key_char {
		($name: literal) => {
			winit::keyboard::Key::Character(winit::keyboard::SmolStr::new_static($name))
		}
	}

	pub struct KeyMap {
		callbacks : HashMap<Box<str>, Box<dyn FnMut(Key, ElementState) + 'static + Send>>,
		pressed_keys: HashSet<Key>,
	}

	impl KeyMap {
		pub fn new () -> Self {
			Self {
				callbacks: HashMap::new(),
				pressed_keys: HashSet::new(),
			}
		}

		pub fn handle_key (&mut self, key: Key, state: ElementState) {
			// Call all callbacks
			self.callbacks.iter_mut().for_each(|(_, callback)| callback(key.clone(), state));
			match (state, self.pressed_keys.contains(&key)) {
				(ElementState::Pressed, false) => {self.pressed_keys.insert(key);}
				(ElementState::Released, true) => {self.pressed_keys.remove(&key);}
				_ => (),
			};
		}

		pub fn register_callback<F: FnMut(Key, ElementState) + 'static + Send> (&mut self, label: &str, callback: F) {
			match self.callbacks.get(label) {
				None => self.callbacks.insert(label.into(), Box::new(callback)),
				Some(_) => panic!("callback already exists with this label"),
			};
		}

		pub fn is_pressed(&self, key: Key) -> bool {
			self.pressed_keys.contains(&key)
		}
	}

	mod key_map_std_traits {
		use super::KeyMap;
		impl Default for KeyMap {
			fn default() -> Self {
				Self::new()
			}
		}
	}
}

pub use key_map::*;
