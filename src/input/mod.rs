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
		callbacks : HashMap<Key, Callbacks>,
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
			match (state, self.pressed_keys.contains(&key)) {
				(ElementState::Pressed, false) => {self.call_callback(&key, state); self.pressed_keys.insert(key);}
				(ElementState::Released, true) => {self.call_callback(&key, state); self.pressed_keys.remove(&key);}
				_ => (),
			}
		}

		pub fn register_callback<F: FnMut() + 'static + Send> (&mut self, key: Key, state: ElementState, callback: F) {
			let entry = self.callbacks.entry(key)
				.or_default();
			match state {
				ElementState::Pressed => entry.on_press = Box::new(callback),
				ElementState::Released => entry.on_release = Box::new(callback),
			}
		}

		pub fn is_pressed(&self, key: Key) -> bool {
			self.pressed_keys.contains(&key)
		}

		fn call_callback(&mut self, key: &Key, state: ElementState) {
			self.callbacks.get_mut(key)
				.map(|callbacks| {
					match state {
						ElementState::Pressed  => (callbacks.on_press)(),
						ElementState::Released => (callbacks.on_release)()
					}
				});
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

	#[cfg(test)]
	mod keu_map_tests {
		use winit::event::ElementState;

	use super::KeyMap;
		#[test]
		fn simple_test () {
			let mut map = KeyMap::new();
			map.register_callback(key_char!("w"), ElementState::Pressed, || eprintln!("called w pressed callback"));
			map.handle_key(key_char!("w"), ElementState::Pressed);
			map.handle_key(key_char!("w"), ElementState::Pressed);
			map.handle_key(key_char!("w"), ElementState::Released);
			map.handle_key(key_char!("w"), ElementState::Pressed);
		}
	}

	struct Callbacks {
		on_press: Box<dyn FnMut() + Send>,
		on_release: Box<dyn FnMut() + Send>,
	}


	impl Default for Callbacks {
		fn default() -> Self {
			Self::empty()
		}
	}

	impl Callbacks {
		fn empty () -> Self {
			fn empty() {}
			Self {
				on_press: Box::new(empty),
				on_release: Box::new(empty),
			}
		}
	}
}

pub use key_map::*;
