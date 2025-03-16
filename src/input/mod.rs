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
		callbacks : HashMap<Box<str>, Box<dyn FnMut(&Key, ElementState) + 'static + Send>>,
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
			self.callbacks.iter_mut().for_each(|(_, callback)| callback(&key, state));
			self.pressed_keys.insert(key);
		}

		pub fn register_callback<F: FnMut(&Key, ElementState) + 'static + Send> (&mut self, label: &str, callback: F) {
			match self.callbacks.get(label) {
				None => self.callbacks.insert(label.into(), Box::new(callback)),
				Some(_) => panic!("callback already exists with this label: {:?}", label),
			};
		}

		pub fn remove_callback(&mut self, label: &str) {
			_ = self.callbacks.remove_entry(label)
				.expect(&format!("Callback with label '{}' was not registered or was already unregistered", label));
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

pub mod mouse_map {
    use std::collections::{HashMap, HashSet};

    use winit::{dpi::PhysicalPosition, event::{ElementState, MouseButton, MouseScrollDelta}};

	pub struct MouseMap {
		/// Position of Mouse from WindowEvent. This does not use the Raw mouse movement.
		/// If Raw movement is required (for camera control, for example), register a 
		/// raw movement callback which forwards the data to the required location
		position: [f64;2],
		/// Scroll level from WindowEvent. This does not use Raw Scroll event
		scroll_level: [f64; 2],
		/// A list of currently pressed mouse buttons
		pressed_buttons : HashSet<MouseButton>,
		/// Callbacks which are called when a raw movement device event is recieved
		raw_movement_callbacks: HashMap<Box<str>, Box<dyn FnMut(&(f64, f64)) + Send + 'static>>,
		/// Callbacks which are called when a raw scroll device event is recieved
		raw_scroll_callbacks: HashMap<Box<str>, Box<dyn FnMut(&MouseScrollDelta) + Send + 'static>>,
		/// Callbacks which are called when a button event is recieved
		button_callbacks: HashMap<Box<str>, Box<dyn FnMut(&MouseButton, ElementState) + Send + 'static>>,
	}

	impl MouseMap {
		pub fn new() -> Self{
			Self {
				position: [0.;2],
				scroll_level: [0.;2],
				pressed_buttons: HashSet::new(),
				raw_movement_callbacks: HashMap::new(),
				raw_scroll_callbacks: HashMap::new(),
				button_callbacks: HashMap::new(),
			}
		}

		// Cursor 
		pub fn handle_cursor_movement(&mut self, position: PhysicalPosition<f64>) {
			self.position = [position.x, position.y];
		}

		// Scroll
		pub fn handle_mouse_wheel(&mut self, delta: MouseScrollDelta) {
			const PIXELS_PER_LINE: f64 = 10.;
			let [x_s, y_s] = self.scroll_level;
			match delta {
				MouseScrollDelta::LineDelta(x_d, y_d) => self.scroll_level = [x_s + PIXELS_PER_LINE * x_d as f64, y_s + PIXELS_PER_LINE * y_d as f64],
				MouseScrollDelta::PixelDelta(delta) => self.scroll_level = [x_s + delta.x, y_s + delta.y],
			}
		}

		pub fn register_raw_scroll_callback<F: FnMut(&MouseScrollDelta) + Send + 'static>
			(&mut self, label: &str, callback: F)
		{
			match self.raw_scroll_callbacks.get(label) {
				None => self.raw_scroll_callbacks.insert(label.into(), Box::new(callback)),
				Some(_) => panic!("Callback already registered with label: {:?}", label)
			};
		}

		pub fn unregister_raw_scroll_callback(&mut self, label: &str) {
			_ = self.raw_scroll_callbacks.remove_entry(label)
				.expect(&format!("Callback with label '{}' was not registered or was already unregistered", label));
		}

		// Mouse Buttons
		pub fn handle_button(&mut self, button: MouseButton, state: ElementState) {
			self.button_callbacks.iter_mut().for_each(|(_, callback)| callback(&button, state));
			self.pressed_buttons.insert(button);
		}

		pub fn register_button_callback<F: FnMut(&MouseButton, ElementState) + Send + 'static>
			(&mut self, label: &str, callback: F)
		{
			match self.button_callbacks.get(label) {
				None => self.button_callbacks.insert(label.into(), Box::new(callback)),
				Some(_) => panic!("Callback already registered with label: {:?}", label)
			};
		}

		pub fn unregister_button_callback(&mut self, label: &str) {
			_ = self.button_callbacks.remove_entry(label)
				.expect(&format!("Callback with label '{}' was not registered or was already unregistered", label));
		}

		// Raw Movement
		pub fn handle_raw_mouse_movement(&mut self, delta: (f64, f64)) {
			self.raw_movement_callbacks.iter_mut().for_each(|(_, callback)| callback(&delta));
		}

		pub fn register_raw_movement_callback<F: FnMut(&(f64, f64)) + Send + 'static>
			(&mut self, label: &str, callback: F)
		{
			match self.raw_movement_callbacks.get(label) {
				None => self.raw_movement_callbacks.insert(label.into(), Box::new(callback)),
				Some(_) => panic!("Callback already registered with label: {:?}", label)
			};
		}

		pub fn unregister_raw_movement_callback(&mut self, label: &str) {
			_ = self.raw_movement_callbacks.remove_entry(label)
				.expect(&format!("Callback with label '{}' was not registered or was already unregistered", label));
		}
	}
}

pub use key_map::*;
