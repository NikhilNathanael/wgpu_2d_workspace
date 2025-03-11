use wgpu::*;
use winit::dpi::PhysicalSize;

pub const SHADER_DIRECTORY: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/src/shaders/");
pub use buffers::*;

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
				self.destroy();
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
