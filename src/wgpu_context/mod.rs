use wgpu::*;
use winit::dpi::PhysicalSize;

pub const SHADER_DIRECTORY: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/src/shaders/");
pub use buffers::*;
pub use shader_manager::*;

pub struct WGPUContext {
	#[allow(dead_code)]
	instance: Instance,
	surface: Surface<'static>,
	#[allow(dead_code)]
	adapter: Adapter,
	device: Device,
	queue: Queue,
	config: SurfaceConfiguration,
}

impl WGPUContext {
	pub fn new(window: std::sync::Arc<winit::window::Window>) -> Self {
		let instance = Instance::new(&wgpu::InstanceDescriptor{
			backends: wgpu::Backends::VULKAN,
			flags: InstanceFlags::DEBUG | InstanceFlags::VALIDATION,
			..Default::default()
		});
		let size = window.inner_size();
		let surface = instance.create_surface(window)
			.expect("Could not create surface");

		let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions{
			compatible_surface: Some(&surface),
			..Default::default()
		})).expect("Could not create adapter");

		let capabilities = surface.get_capabilities(&adapter);

		let config = wgpu::SurfaceConfiguration {
			usage: TextureUsages::RENDER_ATTACHMENT,
			format: capabilities.formats[0],
			width: size.width,
			height: size.height,
			present_mode: wgpu::PresentMode::Fifo,
			desired_maximum_frame_latency: 3,
			alpha_mode: CompositeAlphaMode::Auto,
			view_formats: vec![capabilities.formats[0]],
		};

		let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor{
				label: Some("Device"),
				required_features: Features::all_webgpu_mask() & !Features::BGRA8UNORM_STORAGE,
				memory_hints: MemoryHints::Performance,
				..Default::default()
			},
			None,
		)).expect("Could not create device and queue");

		device.on_uncaptured_error(Box::new(|error| {
			match error {
				wgpu::Error::OutOfMemory{..} => println!("Out of memory"),
				wgpu::Error::Validation{description, ..} => println!("Validation Error: {description}"),
				wgpu::Error::Internal{description, ..} => println!("Internal Error: {description}"),
			}
			std::process::exit(25);
		}));

		surface.configure(&device, &config);
		Self {
			instance,
			surface,
			adapter,
			device,
			queue,
			config,
		}
	}

	pub fn surface(&self) -> &Surface {
		&self.surface
	}

	pub fn device(&self) -> &Device {
		&self.device
	}

	pub fn queue(&self) -> &Queue {
		&self.queue
	}

	pub fn config(&self) -> &SurfaceConfiguration {
		&self.config
	}

	pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
		self.config.width = new_size.width;
		self.config.height = new_size.height;
		self.surface.configure(&self.device, &self.config);
	}
}

pub trait BufferData {
	// If a type requires filling multiple buffers, this should a tuple of compatible buffers
	type Buffers;
	fn create_buffers(&self, context: &WGPUContext) -> Self::Buffers;
	fn fill_buffers(&self, buffers: &mut Self::Buffers, context: &WGPUContext);
}

pub struct BufferAndData<T: BufferData> {
	pub data: T,
	pub buffers: T::Buffers,
}

impl<T: BufferData> BufferAndData<T> {
	pub fn new(data: T, context: &WGPUContext) -> Self {
		let mut buffers = T::create_buffers(&data, context);
		T::fill_buffers(&data, &mut buffers, context);
		Self {
			data,
			buffers,
		}
	}

	pub fn update_buffer(&mut self, context: &WGPUContext) {
		self.data.fill_buffers(&mut self.buffers, context);
	}
}

mod buffers {
	use super::WGPUContext;

	use wgpu::*;

	use bytemuck::Pod;

	use std::num::NonZero;

	pub use uniform_buffer::*;
	pub use vertex_buffer::*;
	pub use storage_buffer::*;
	pub use index_buffer::*;

	pub trait WGPUBuffer: Sized {
		fn create(size: u64, context: &WGPUContext) -> Self;
		fn destroy(&self);
		fn size(&self) -> u64;
		fn resize(&mut self, new_size: u64, context: &WGPUContext) {
			if self.size() < new_size {
				self.destroy();
				*self = Self::create(new_size, context);
			}
		}
		fn write_iter<'a, I, T>(&self, data: I, context: &WGPUContext) where 
			I: Iterator<Item = &'a T> + ExactSizeIterator,
			T: Pod + Sized;
		fn write_data(&self, data: &[u8], context: &WGPUContext);
	}

	mod uniform_buffer {
		use super::*;
		pub struct UniformBuffer {
			pub buffer: Buffer
		}

		impl std::ops::Deref for UniformBuffer {
			type Target = Buffer;
			fn deref(&self) -> &Self::Target {
				&self.buffer
			}
		}
		
		impl std::ops::DerefMut for UniformBuffer {
			fn deref_mut(&mut self) -> &mut Self::Target {
				&mut self.buffer
			}
		}

		impl UniformBuffer {
			pub fn new(size: u64, context: &WGPUContext) -> Self {
				const UNIFORM_BUFFER_ALIGNMENT: u64 = 16;
				Self {
					buffer: context.device().create_buffer(&BufferDescriptor{
						label: Some("Uniform Buffer"),
						size: ((size - 1) / UNIFORM_BUFFER_ALIGNMENT + 1) * UNIFORM_BUFFER_ALIGNMENT,
						usage: BufferUsages::COPY_DST | BufferUsages::UNIFORM,
						mapped_at_creation: false,
					})
				}
			}
		}

		impl WGPUBuffer for UniformBuffer {
			fn create(size: u64, context: &WGPUContext) -> Self {
				Self::new(size, context)
			}

			fn size(&self) -> u64 {
				self.buffer.size()
			}

			fn destroy(&self) {self.buffer.destroy();}

			fn write_iter<'a, I, T>(&self, data: I, context: &WGPUContext) where 
				I: Iterator<Item = &'a T> + ExactSizeIterator,
				T: Pod + Sized 
			{
				let total_size = (std::mem::size_of::<T>() * data.len()) as u64;
				context.queue().write_buffer_with(&self.buffer, 0, NonZero::new(total_size).unwrap())
					.expect("Could nto write to buffer")
					.chunks_mut(std::mem::size_of::<T>())
					.zip(data)
					.for_each(|(buffer_slice, data_elem)| 
						buffer_slice.copy_from_slice(bytemuck::bytes_of(data_elem))
					);
			}

			fn write_data(&self, data: &[u8], context: &WGPUContext) {
				context.queue().write_buffer(&self.buffer, 0, data);
			}
		}

		impl Drop for UniformBuffer {
			fn drop(&mut self) {
				self.buffer.destroy();
			}
		}
	}

	mod vertex_buffer {
		use super::*;
		pub struct VertexBuffer {
			pub buffer: Buffer
		}

		impl std::ops::Deref for VertexBuffer {
			type Target = Buffer;
			fn deref(&self) -> &Self::Target {
				&self.buffer
			}
		}
		
		impl std::ops::DerefMut for VertexBuffer {
			fn deref_mut(&mut self) -> &mut Self::Target {
				&mut self.buffer
			}
		}

		impl VertexBuffer {
			pub fn new(size: u64, context: &WGPUContext) -> Self {
				Self {
					buffer: context.device().create_buffer(&BufferDescriptor{
						label: Some("Vertex Buffer"),
						size,
						usage: BufferUsages::COPY_DST | BufferUsages::VERTEX,
						mapped_at_creation: false,
					})
				}
			}
		}

		impl WGPUBuffer for VertexBuffer {
			fn create(size: u64, context: &WGPUContext) -> Self {
				Self::new(size, context)
			}

			fn size(&self) -> u64 {
				self.buffer.size()
			}

			fn destroy(&self) {self.buffer.destroy();}

			fn write_iter<'a, I, T>(&self, data: I, context: &WGPUContext) where 
				I: Iterator<Item = &'a T> + ExactSizeIterator,
				T: Pod + Sized 
			{
				let total_size = (std::mem::size_of::<T>() * data.len()) as u64;
				context.queue().write_buffer_with(&self.buffer, 0, NonZero::new(total_size).unwrap())
					.expect("Could nto write to buffer")
					.chunks_mut(std::mem::size_of::<T>())
					.zip(data)
					.for_each(|(buffer_slice, data_elem)| {
						buffer_slice.copy_from_slice(bytemuck::bytes_of(data_elem))
					});
			}

			fn write_data(&self, data: &[u8], context: &WGPUContext) {
				context.queue().write_buffer(&self.buffer, 0, data);
			}
		}

		impl Drop for VertexBuffer {
			fn drop(&mut self) {
				self.buffer.destroy();
			}
		}
	}

	mod storage_buffer {
		use super::*;
		pub struct StorageBuffer {
			pub buffer: Buffer
		}

		impl std::ops::Deref for StorageBuffer {
			type Target = Buffer;
			fn deref(&self) -> &Self::Target {
				&self.buffer
			}
		}
		
		impl std::ops::DerefMut for StorageBuffer {
			fn deref_mut(&mut self) -> &mut Self::Target {
				&mut self.buffer
			}
		}

		impl StorageBuffer {
			pub fn new(size: u64, context: &WGPUContext) -> Self {
				Self {
					buffer: context.device().create_buffer(&BufferDescriptor{
						label: Some("Storage Buffer"),
						size,
						usage: BufferUsages::COPY_DST | BufferUsages::STORAGE,
						mapped_at_creation: false,
					})
				}
			}
		}

		impl WGPUBuffer for StorageBuffer {
			fn create(size: u64, context: &WGPUContext) -> Self {
				Self::new(size, context)
			}

			fn size(&self) -> u64 {
				self.buffer.size()
			}

			fn destroy(&self) {self.buffer.destroy();}

			fn write_iter<'a, I, T>(&self, data: I, context: &WGPUContext) where 
				I: Iterator<Item = &'a T> + ExactSizeIterator,
				T: Pod + Sized 
			{
				let total_size = (std::mem::size_of::<T>() * data.len()) as u64;
				context.queue().write_buffer_with(&self.buffer, 0, NonZero::new(total_size).unwrap())
					.expect("Could nto write to buffer")
					.chunks_mut(std::mem::size_of::<T>())
					.zip(data)
					.for_each(|(buffer_slice, data_elem)| 
						buffer_slice.copy_from_slice(bytemuck::bytes_of(data_elem))
					);
			}

			fn write_data(&self, data: &[u8], context: &WGPUContext) {
				context.queue().write_buffer(&self.buffer, 0, data);
			}
		}

		impl Drop for StorageBuffer {
			fn drop(&mut self) {
				self.buffer.destroy();
			}
		}
	}

	mod index_buffer {
		use super::*;
		pub struct IndexBuffer {
			pub buffer: Buffer
		}

		impl std::ops::Deref for IndexBuffer {
			type Target = Buffer;
			fn deref(&self) -> &Self::Target {
				&self.buffer
			}
		}
		
		impl std::ops::DerefMut for IndexBuffer {
			fn deref_mut(&mut self) -> &mut Self::Target {
				&mut self.buffer
			}
		}

		impl IndexBuffer {
			pub fn new(size: u64, context: &WGPUContext) -> Self {
				Self {
					buffer: context.device().create_buffer(&BufferDescriptor{
						label: Some("Index Buffer"),
						size,
						usage: BufferUsages::COPY_DST | BufferUsages::INDEX,
						mapped_at_creation: false,
					})
				}
			}
		}

		impl WGPUBuffer for IndexBuffer {
			fn create(size: u64, context: &WGPUContext) -> Self {
				Self::new(size, context)
			}

			fn size(&self) -> u64 {
				self.buffer.size()
			}

			fn destroy(&self) {self.buffer.destroy();}

			fn write_iter<'a, I, T>(&self, data: I, context: &WGPUContext) where 
				I: Iterator<Item = &'a T> + ExactSizeIterator,
				T: Pod + Sized 
			{
				let total_size = (std::mem::size_of::<T>() * data.len()) as u64;
				context.queue().write_buffer_with(&self.buffer, 0, NonZero::new(total_size).unwrap())
					.expect("Could nto write to buffer")
					.chunks_mut(std::mem::size_of::<T>())
					.zip(data)
					.for_each(|(buffer_slice, data_elem)| 
						buffer_slice.copy_from_slice(bytemuck::bytes_of(data_elem))
					);
			}

			fn write_data(&self, data: &[u8], context: &WGPUContext) {
				context.queue().write_buffer(&self.buffer, 0, data);
			}
		}

		impl Drop for IndexBuffer {
			fn drop(&mut self) {
				self.buffer.destroy();
			}
		}
	}
}

mod shader_manager {
	use wgpu::*;
	use std::collections::HashMap;
	use super::WGPUContext;

	use std::borrow::Cow;
	use std::sync::Mutex;
	use std::num::NonZeroU32;
	use std::collections::hash_map::Entry;

	// Manages loading and compilation of shaders from disk
	//
	// uses unsafe code to allow taking shared references into the data while 
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
	// 	  However, the returned reference (&'a T) is derived from such a shared reference.
	// 	  

	pub struct ShaderManager {
		directory_path: &'static str,
		shader_modules: Mutex<HashMap<&'static str, Box<ShaderModule>>>,
		render_pipelines: Mutex<HashMap<&'static str, (RenderPipelineDescriptorTemplate, Option<Box<RenderPipeline>>)>>,
	}

	impl ShaderManager {
		pub fn new(directory_path: &'static str) -> Self {
			Self {
				directory_path,
				shader_modules: Mutex::new(HashMap::new()),
				render_pipelines: Mutex::new(HashMap::new()),
			}
		}

		fn read_and_get_module(&self, path: &'static str, context: &WGPUContext) -> ShaderModule {
			let file = Cow::Owned(std::fs::read_to_string(self.directory_path.to_owned() + path)
				.expect("Could not read shader file"));
			context.device().create_shader_module(ShaderModuleDescriptor{
				label: Some(path),
				source: ShaderSource::Wgsl(file),
			})
		}

		pub fn get_module<'a>(&'a self, path: &'static str, context: &WGPUContext) -> &'a ShaderModule {
			// SAFETY: The only thing that can invalidate the lifetime of the returned reference 
			// is if the backing Box is deallocated (moving a box does not invalidate pointers into it)
			//
			// The returned reference's lifetime is tied to the shared borrow of self and we do not
			// allow any operations with a shared reference to self to drop or remove any element 
			// from the map
			unsafe {&*(&**self.shader_modules.lock().unwrap()
				.entry(path)
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
			(match self.render_pipelines.lock().unwrap()
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
			})
		}

		pub fn register_render_pipeline(&self, label: &'static str, template: RenderPipelineDescriptorTemplate) {
			match self.render_pipelines.lock().unwrap()
				.entry(label) {
				// we only have shared access to self here so there may be borrows into 
				// any existing pipeline here.
				// we must take care not to remove any existing render pipelines
				Entry::Occupied(x) if x.get().0 == template => (),
				Entry::Occupied(_) => panic!("A pipeline has already been registered with the same label but different template"),
				// this insertion is fine because there is not render pipeline to 
				// invalidate here
				Entry::Vacant(x) => {x.insert((template, None));},
			}
		}

		pub fn reload(&mut self) {
			// These mutable operations are fine because we have mutable access to self
			// so there are no borrows of this data
			self.shader_modules.lock().unwrap().clear();
			self.render_pipelines.lock().unwrap().iter_mut().for_each(|(_, (_, x))| *x = None);
		}
	}

	#[derive(Debug, Clone, PartialEq)]
	pub struct RenderPipelineDescriptorTemplate {
		pub label: Label<'static>,
		pub layout: Option<&'static PipelineLayout>,
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
			&self, 
			v_module: &'a ShaderModule, 
			f_module: Option<&'a ShaderModule>
		) -> RenderPipelineDescriptor<'a> {
			RenderPipelineDescriptor {
				label: self.label,
				layout: self.layout,
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
		pub targets: &'static [Option<ColorTargetState>],
	}

	impl FragmentStateTemplate {
		fn resolve<'a>(&self, module: &'a ShaderModule) -> FragmentState<'a> {
			FragmentState {
				module,
				entry_point: self.entry_point,
				// We do not support overridable constants here
				compilation_options: Default::default(),
				targets: self.targets,
			}
		}

		fn get_module_path(&self) -> &'static str {
			self.module_path
		}
	}
}
