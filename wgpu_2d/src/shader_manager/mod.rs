use wgpu::*;
use std::collections::HashMap;
use crate::wgpu_context::WGPUContext;

use std::borrow::Cow;
use std::sync::Mutex;
use std::num::NonZeroU32;
use std::collections::hash_map::Entry;

// Manages loading and compilation of shaders from disk
//
// Uses unsafe code to allow taking shared references into the data while 
// mutating the HashMaps. This use of unsafe has been thought through but 
// it has not been fully verified. Specific safety comments can be found 
// at the site of the unsafe blocks
//
// Shared references to the shader modules and render pipelines can be obtained
// with the same lifetime as the shared reference to self instead of to the MutexGuard. 
// This should be okay as long as
// 	a) The actual heap allocation of the box is never moved or modified
// 	b) The hashmap never removes or replaces a shader module or render pipeline through 
// 	   a shared reference
//
// Mutable references are allowed to modify the data in any way they want. This is used to 
// clear the data to allow for hot-reloading
//
// Unanswered questions about safety
// 	- The code below creates mutable references to the box (&mut Box<T>). Care is taken to
// 	  avoid a shared reference to the actual box (i.e. &Box<T>) as that would immediately be UB. 
// 	  However, the returned reference (&'a T) is derived from such a shared reference. It is 
// 	  unclear if this is also UB
//
// 	- A simplified version of the shader manager which requires no OS code (file opening) 
// 	was tested with miri in the test module below. It detected no UB in the current configuration
// 		- A sanity check was performed by returning a static refernce from get_module and it 
// 		  correctly identified the UB
//  - This seems to suggest that this use of unsafe is indeed sound, but further research is needed
// 	  

// Note: The shader manager uses 'static strs as the keys to the maps currently,
// In all examples so far, a hardcoded string has been sufficient to specify the 
// shader, so there is no overhead.
// However, if shader paths need to be determined at runtime, the only way to use 
// this structure is by leaking a string (with String::leak or Box::leak).
// It can be changed to use a Box<str> as the key in that case

enum Include<'a> {
	Absolute(&'a str),
	// Relative(&'a str),
}

pub struct ShaderManager {
	directory_path: Box<str>,
	shader_source: Mutex<HashMap<Box<str>, (String, Vec<Box<str>>)>>,
	shader_modules: Mutex<HashMap<Box<str>, Box<ShaderModule>>>,
	render_pipelines: Mutex<HashMap<Box<str>, (RenderPipelineDescriptorTemplate, Option<Box<RenderPipeline>>)>>,
}

impl ShaderManager {
	pub fn new(directory_path: &str) -> Self {
		Self {
			directory_path: directory_path.into(),
			shader_source: Mutex::new(HashMap::new()),
			shader_modules: Mutex::new(HashMap::new()),
			render_pipelines: Mutex::new(HashMap::new()),
		}
	}

	fn resolve_source(&self, path: &str) -> (String, Vec<Box<str>>) {
		log::trace!("resolving source for file: {:?}", path);
		use std::io::BufRead;
		// Open file
		let mut file = std::io::BufReader::new(std::fs::File::open(self.directory_path.to_string() + path)
			.expect("Could not open file"));
		let mut includes = Vec::new();
		let mut source = String::new();

		let mut line = String::new();
		
		while let Ok(x) = file.read_line(&mut line) {
			// read_line returns Ok(0) after EOF
			if x == 0 {break;}
			match get_include_path(&*line) {
				None => source.push_str(&line),
				Some(Include::Absolute(path)) => {
					let include_source = self.get_source(path);
					let include_includes = self.get_includes(path);
					includes.extend_from_slice(include_includes);
					source.push_str(include_source);
				}
			}
			line.clear();
		}
		log::trace!("finished resolving source for file: {:?}", path);
		return (source, includes);

		// Turns a line like 
		// | #include <<path/to/file>> into `Some(Include::Absolute("path/to/file"))`
		fn get_include_path(line: &str) -> Option<Include> {
			let path_container = line.trim()
				.split_once("#include")?.1
				.trim();
			(|| Some(Include::Absolute(
					path_container
						.split_once('<')?.1
						.rsplit_once('>')?.0
			)))().or(
				None
			)
		}
	}

	fn get_includes<'a> (&self, path: &str) -> &'a [Box<str>] {
		match self.shader_source.lock().unwrap().get(path) {
			None => (),
			Some((_, includes)) => return unsafe{&*(&**includes as *const [Box<str>])},
		}
		log::debug!("source file not already loaded: {:?}", path);

		let (source, includes) = self.resolve_source(path);
		if includes.iter().find(|x| &***x == path).is_some() {
			// Only works if the dependancy was already loaded, 
			// otherwise it will just overflow the stack ¯\_(ツ)_/¯ 
			//
			// It is guaranteed to crash so its not really a safety problem
			log::error!(
				"Shader error: Circular Dependancy in source file {:?}\n Resolved Includes: {:?}", 
				path, includes);
			panic!();
		}
		use std::collections::hash_map::Entry;
		match self.shader_source.lock().unwrap().entry(path.into()) {
			Entry::Occupied(x) => unsafe{&*(&*x.get().1 as *const [Box<str>])},
			Entry::Vacant(x) => unsafe{&*(&*x.insert((source, includes)).1 as *const [Box<str>])}
		}
	}

	fn get_source<'a>(&'a self, path: &str) -> &'a str {
		match self.shader_source.lock().unwrap().get(path) {
			None => (),
			Some((source, _)) => return unsafe{&*(&**source as *const str)},
		}
		log::debug!("source file not already loaded: {:?}", path);

		let (source, includes) = self.resolve_source(path);
		if includes.iter().find(|x| &***x == path).is_some() {
			// Only works if the dependancy was already loaded, 
			// otherwise it will just overflow the stack ¯\_(ツ)_/¯ 
			//
			// It is guaranteed to crash so its not really a safety problem
			log::error!(
				"Shader error: Circular Dependancy in source file {:?}\n Resolved Includes: {:?}", 
				path, includes);
			panic!();
		}
		use std::collections::hash_map::Entry;
		match self.shader_source.lock().unwrap().entry(path.into()) {
			Entry::Occupied(x) => unsafe{&*(&*x.get().0 as *const str)},
			Entry::Vacant(x) => unsafe{&*(&*x.insert((source, includes)).0 as *const str)}
		}
	}

	fn read_and_get_module(&self, path: &str, context: &WGPUContext) -> ShaderModule {
		let file = Cow::Borrowed(self.get_source(path));
		context.device().create_shader_module(ShaderModuleDescriptor{
			label: Some(path),
			source: ShaderSource::Wgsl(file),
		})
	}

	fn get_module<'a>(&'a self, path: &str, context: &WGPUContext) -> &'a ShaderModule {
		// SAFETY: The only thing that can invalidate the lifetime of the returned reference 
		// is if the backing Box is deallocated (moving a box does not invalidate pointers into it)
		//
		// The returned reference's lifetime is tied to the shared borrow of self and we do not
		// allow any operations with a shared reference to self to drop or remove any element 
		// from the map
		unsafe {&*(&**self.shader_modules.lock().unwrap()
			.entry(path.into())
			// putting a new module into the map could invalidate old references 
			// but we ensure that this is never done to an existing module
			.or_insert(Box::new(self.read_and_get_module(path, context)))

			// BE VERY CAREFUL ADDING ANY EXTRA LINES OF CODE HERE
			as *const ShaderModule)
		}
	}

	fn compile_pipeline(&self, template: &RenderPipelineDescriptorTemplate, context: &WGPUContext) -> RenderPipeline {
		let paths = template.get_module_paths();
		let modules = (
			self.get_module(paths.0, context),
			paths.1.map(|x| self.get_module(x, context))
		);
		let descriptor: RenderPipelineDescriptor = template.resolve(modules.0, modules.1);
		
		context.device().create_render_pipeline(&descriptor)
	}

	pub fn get_render_pipeline<'a>(&'a self, label: &str, context: &WGPUContext) -> &'a RenderPipeline {
		match self.render_pipelines.lock().unwrap()
			.get_mut(label)
			.expect("Tried to access a render pipeline that wasn't registered")
			{
				// SAFETY: The only thing that can invalidate the lifetime of the returned reference 
				// is if the backing Box is deallocated (moving a box does not invalidate pointers into it)
				//
				// The returned reference's lifetime is tied to the shared borrow of self and we do not
				// allow any operations with a shared reference to self to drop or remove an element 
				// from the map
				(template, x) => unsafe{&*(
					// putting a new pipeline into the map could invalidate old references, 
					// but we ensure that this is only done if there wasn't already a pipeline there
					&**x.get_or_insert_with(
						|| Box::new(self.compile_pipeline(template, context))

					// BE VERY CAREFUL ADDING ANY EXTRA LINES OF CODE HERE
					) as *const RenderPipeline
				)},
		}
	}

	pub fn register_render_pipeline(&self, label: &str, template: RenderPipelineDescriptorTemplate) {
		match self.render_pipelines.lock().unwrap()
			.entry(label.into()) {
			// we only have shared access to self here so there may be borrows into 
			// any existing pipeline here.
			// we must take care not to remove any existing render pipelines
			Entry::Occupied(_) => (),

			// this insertion is fine because there is not render pipeline to 
			// invalidate here
			Entry::Vacant(x) => {x.insert((template, None));},
		}
	}

	pub fn reload(&mut self) {
		// These mutable operations are fine because we have mutable access to self
		// so there are no borrows of this data
		self.shader_source.lock().unwrap().clear();
		self.shader_modules.lock().unwrap().clear();
		self.render_pipelines.lock().unwrap().iter_mut().for_each(|(_, (_, x))| *x = None);
	}
}

#[derive(Debug, Clone, PartialEq)]
pub struct RenderPipelineDescriptorTemplate {
	pub label: Label<'static>,
	pub layout: Option<PipelineLayout>,
	pub vertex: VertexStateTemplate,
	pub primitive: PrimitiveState,
	pub depth_stencil: Option<DepthStencilState>,
	pub multisample: MultisampleState,
	pub fragment: Option<FragmentStateTemplate>,
	pub multiview: Option<NonZeroU32>,
	pub cache: Option<&'static PipelineCache>,
}

impl RenderPipelineDescriptorTemplate {
	fn resolve<'a> (
		&'a self, 
		v_module: &'a ShaderModule, 
		f_module: Option<&'a ShaderModule>
	) -> RenderPipelineDescriptor<'a> {
		RenderPipelineDescriptor {
			label: self.label,
			layout: self.layout.as_ref(),
			vertex: self.vertex.resolve(v_module),
			primitive: self.primitive,
			depth_stencil: self.depth_stencil.clone(),
			multisample: self.multisample,
			fragment: self.fragment.as_ref().map(|x| x.resolve(f_module.unwrap())),
			multiview: self.multiview,
			cache: self.cache,
		}
	}

	fn get_module_paths(&self) -> (&'static str, Option<&'static str>) {
		(
			self.vertex.get_module_path(),
			self.fragment.as_ref().map(|x| x.get_module_path()),
		)
	}
}

#[derive(Debug, Clone, PartialEq)]
pub struct VertexStateTemplate {
	pub module_path: &'static str,
	pub entry_point: Option<&'static str>,
	// We do not support overridable constants here
	pub buffers: &'static [VertexBufferLayout<'static>],
}

impl VertexStateTemplate {
	fn resolve<'a>(&self, module: &'a ShaderModule) -> VertexState<'a> {
		VertexState {
			module,
			entry_point: self.entry_point,
			// We do not support overridable constants here
			compilation_options: Default::default(),
			buffers: self.buffers,
		}
	}

	fn get_module_path(&self) -> &'static str {
		self.module_path
	}
}

#[derive(Debug, Clone, PartialEq)]
pub struct FragmentStateTemplate {
	pub module_path: &'static str,
	pub entry_point: Option<&'static str>,
	// We do not support overridable constants here
	pub targets: Box<[Option<ColorTargetState>]>,
}

impl FragmentStateTemplate {
	fn resolve<'a>(&'a self, module: &'a ShaderModule) -> FragmentState<'a> {
		FragmentState {
			module,
			entry_point: self.entry_point,
			// We do not support overridable constants here
			compilation_options: Default::default(),
			targets: &self.targets,
		}
	}

	fn get_module_path(&self) -> &'static str {
		self.module_path
	}
}

#[cfg(test)]
mod test {
	use std::collections::HashMap;
	use std::sync::Mutex;
	pub struct ShaderManager {
		directory_path: &'static str,
		shader_modules: Mutex<HashMap<&'static str, Box<str>>>,
		render_pipelines: Mutex<HashMap<&'static str, (Vec<String>, Option<Box<str>>)>>,
	}

	impl ShaderManager {
		pub fn new(directory_path: &'static str) -> Self {
			Self {
				directory_path,
				shader_modules: Mutex::new(HashMap::new()),
				render_pipelines: Mutex::new(HashMap::new()),
			}
		}

		fn read_and_get_module(&self, path: &str) -> Box<str> {
			path.to_owned().into()
		}

		pub fn get_module<'a>(&'a self, path: &'static str) -> &'a str {
			// SAFETY: The only thing that can invalidate the lifetime of the returned reference 
			// is if the backing Box is deallocated (moving a box does not invalidate pointers into it)
			//
			// The returned reference's lifetime is tied to the shared borrow of self and we do not
			// allow any operations with a shared reference to self to drop or remove any element 
			// from the map
			unsafe {&*((&**self.shader_modules.lock().unwrap()
				.entry(path)
				// putting a new module into the map could invalidate old references 
				// but we ensure that this is never done to an existing module
				.or_insert(self.read_and_get_module(path)))

				// BE VERY CAREFUL ADDING ANY EXTRA LINES OF CODE HERE
				as *const str)
			}
		}

		// fn compile_pipeline(&self, template: &RenderPipelineDescriptorTemplate, context: &WGPUContext) -> RenderPipeline {
		// 	let paths = template.get_module_paths();
		// 	let modules = (
		// 		self.get_module(paths.0, context),
		// 		paths.1.map(|x| self.get_module(x, context))
		// 	);
		// 	let descriptor: RenderPipelineDescriptor = template.resolve(modules.0, modules.1);
			
		// 	context.device().create_render_pipeline(&descriptor)
		// }

		//pub fn get_render_pipeline<'a>(&'a self, label: &str, context: &WGPUContext) -> &'a RenderPipeline {
		//	match self.render_pipelines.lock().unwrap()
		//		.get_mut(label)
		//		.expect("Tried to access a render pipeline that wasn't registered")
		//		{
		//			// SAFETY: The only thing that can invalidate the lifetime of the returned reference 
		//			// is if the backing Box is deallocated (moving a box does not invalidate pointers into it)
		//			//
		//			// The returned reference's lifetime is tied to the shared borrow of self and we do not
		//			// allow any operations with a shared reference to self to drop or remove an element 
		//			// from the map
		//			(template, x) => unsafe{&*(
		//				// putting a new pipeline into the map could invalidate old references, 
		//				// but we ensure that this is only done if there wasn't already a pipeline there
		//				&**x.get_or_insert_with(
		//					|| Box::new(self.compile_pipeline(template, context))

		//				// BE VERY CAREFUL ADDING ANY EXTRA LINES OF CODE HERE
		//				) as *const RenderPipeline
		//			)},
		//	}
		//}

		// pub fn register_render_pipeline(&self, label: &'static str, template: RenderPipelineDescriptorTemplate) {
		// 	match self.render_pipelines.lock().unwrap()
		// 		.entry(label) {
		// 		// we only have shared access to self here so there may be borrows into 
		// 		// any existing pipeline here.
		// 		// we must take care not to remove any existing render pipelines
		// 		Entry::Occupied(_) => (),

		// 		// this insertion is fine because there is not render pipeline to 
		// 		// invalidate here
		// 		Entry::Vacant(x) => {x.insert((template, None));},
		// 	}
		// }

		pub fn reload(&mut self) {
			// These mutable operations are fine because we have mutable access to self
			// so there are no borrows of this data
			self.shader_modules.lock().unwrap().clear();
			self.render_pipelines.lock().unwrap().iter_mut().for_each(|(_, (_, x))| *x = None);
		}
	}

	#[test] 
	fn basic_test () {
		let mut x = ShaderManager::new(concat!(env!("CARGO_MANIFEST_DIR"), "/src/shaders/"));
		let mut vec = Vec::new();
		for i in (0..50) {
			vec.push(x.get_module("points.wgsl"));
		}
		for i in (0..50) {
			vec.push(x.get_module("rect.wgsl"));
		}
		println!("{:?}", vec);
		vec = Vec::new();
		x.reload();
		for i in (0..50) {
			vec.push(x.get_module("points.wgsl"));
			vec.push(x.get_module("rect.wgsl"));
		}
		for i in (0..50) {
			vec.push(x.get_module("triangle.wgsl"));
		}
		println!("{:?}", vec);
	}
}

#[cfg(test)]
mod test2 {
	use crate::wgpu_context;
	use super::*;
	#[test]
	fn include_resolution_test() {
		let x = ShaderManager::new(concat!(env!("CARGO_MANIFEST_DIR"), "/src/shaders/"));
		println!("{:?}", x.directory_path);
		x.get_source("circle.wgsl");
	}
}
