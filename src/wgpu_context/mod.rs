use wgpu::{Adapter, CompositeAlphaMode, Device, Features, Instance, InstanceFlags, MemoryHints, Queue, Surface, SurfaceConfiguration, TextureUsages};

pub struct WGPUContext {
	instance: Instance,
	surface: Surface<'static>,
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
			config
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
}
