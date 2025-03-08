use wgpu::*;
use winit::dpi::PhysicalSize;
use std::sync::Arc;

use bytemuck::Pod;

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
				wgpu::Error::OutOfMemory{..} => log::error!("Out of memory"),
				wgpu::Error::Validation{description, ..} => log::error!("Validation Error: {description}"),
				wgpu::Error::Internal{description, ..} => log::error!("Internal Error: {description}"),
			}
			std::process::exit(1);
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

pub struct VecAndBuffer<T> {
	pub data: Vec<T>,
	pub buffer: Buffer,
}

impl<T: Pod> VecAndBuffer<T> {
	pub fn new(data: Vec<T>, usage: BufferUsages, context: &WGPUContext) -> Self {
		let buffer = create_buffer_with_size(std::mem::size_of_val(&*data) as u64, usage, context);
		buffer.slice(..)
			.get_mapped_range_mut()
			.copy_from_slice(bytemuck::cast_slice(&*data));
		buffer.unmap();
		Self {
			data,
			buffer,
		}
	}

	pub fn update_buffer(&mut self, context: &WGPUContext) {
		if self.buffer.size() < std::mem::size_of_val(&*self.data) as u64 {
			self.buffer = create_buffer_with_size(
				std::mem::size_of_val(&*self.data) as u64, 
				self.buffer.usage(), 
				context
			);
		}
		context.queue().write_buffer(&self.buffer, 0, bytemuck::cast_slice(&*self.data));
		context.queue().submit([]);
	}
}

pub struct DataAndBuffer<T> {
	pub data: T,
	pub buffer: Buffer,
}

impl<T: Pod> DataAndBuffer<T> {
	pub fn new(data: T, usage: BufferUsages, context: &WGPUContext) -> Self {
		const UNIFORM_BUFFER_ALIGNMENT: u64 = 16;
		let buffer = create_buffer_with_size(
			((std::mem::size_of::<T>() as u64 - 1) / UNIFORM_BUFFER_ALIGNMENT + 1) * UNIFORM_BUFFER_ALIGNMENT,
			usage,
			context,
		);
		buffer.slice(..(std::mem::size_of::<T>() as u64))
			.get_mapped_range_mut()
			.copy_from_slice(bytemuck::bytes_of(&data));
		buffer.unmap();
		Self {
			data,
			buffer,
		}
	}

	pub fn update_buffer(&mut self, context: &WGPUContext) {
		context.queue().write_buffer(&self.buffer, 0, bytemuck::bytes_of(&self.data));
		context.queue().submit([]);
	}
}

fn create_buffer_with_size(size: u64, usage: BufferUsages, context: &WGPUContext) -> Buffer {
	context.device().create_buffer(&BufferDescriptor{
		label: None,
		size: size,
		usage: BufferUsages::COPY_DST | usage,
		mapped_at_creation: true, 
	})
}

pub trait BufferData {
	// If a type requires filling multiple buffers, this should a tuple of compatible buffers
	type Buffers;
	fn create_buffers(&self, context: &WGPUContext) -> Self::Buffers;
	fn fill_buffers(&self, buffers: &mut Self::Buffers, context: &WGPUContext);
	fn resize_buffers(&self, buffers: &mut Self::Buffers, context:&WGPUContext);
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
		fn resize(&mut self, new_size: u64, context: &WGPUContext) {
			self.destroy();
			*self = Self::create(new_size, context);
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

// TODO: Create trait that describes type which can be put into a GPU buffer
//
// Implement this trait for any uniform variables directly and implement it 
// on slices or Vecs of vertex and storage data
//
// Question: 
// 	Should there be separate types which represent Uniform, vertex, index and storage buffers, 
// 	or should you just create them with the required usage manually?
// 	What about textures?

