use wgpu::*;
use winit::dpi::PhysicalSize;
use std::sync::Arc;

use bytemuck::Pod;

pub const SHADER_DIRECTORY: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/src/shaders/");

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
	pub fn new(context: &WGPUContext, data: Vec<T>, usage: BufferUsages) -> Self {
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
	pub fn new(context: &WGPUContext, data: T, usage: BufferUsages) -> Self {
		const UNIFORM_BUFFER_ALIGNMENT: u64 = 16;
		let buffer = create_buffer_with_size(
			((std::mem::size_of::<T>() as u64 - 1) / UNIFORM_BUFFER_ALIGNMENT + 1) * UNIFORM_BUFFER_ALIGNMENT,
			usage,
			context,
		);
		buffer.slice(..)
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
