use wgpu::*;

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
    pub fn new(window: impl Into<SurfaceTarget<'static>>, size: [u32; 2]) -> Self {
        let instance = Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN,
            flags: InstanceFlags::DEBUG | InstanceFlags::VALIDATION,
            ..Default::default()
        });
        let surface = instance
            .create_surface(window)
            .expect("Could not create surface");

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: Some(&surface),
            ..Default::default()
        }))
        .expect("Could not create adapter");

        let capabilities = surface.get_capabilities(&adapter);

        let config = wgpu::SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: capabilities.formats[0],
            width: size[0],
            height: size[1],
            present_mode: wgpu::PresentMode::Fifo,
            desired_maximum_frame_latency: 1,
            alpha_mode: CompositeAlphaMode::Auto,
            view_formats: vec![capabilities.formats[0]],
        };

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("Device"),
                required_features: Features::all_webgpu_mask() & !Features::BGRA8UNORM_STORAGE,
                memory_hints: MemoryHints::Performance,
                ..Default::default()
            },
            None,
        ))
        .expect("Could not create device and queue");

        device.on_uncaptured_error(Box::new(|error| {
            match error {
                wgpu::Error::OutOfMemory { .. } => log::error!("Out of memory"),
                wgpu::Error::Validation { description, .. } => {
                    log::error!("Validation Error: {description}")
                }
                wgpu::Error::Internal { description, .. } => {
                    log::error!("Internal Error: {description}")
                }
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

    pub fn resize(&mut self, new_size: [u32; 2]) {
        self.config.width = new_size[0];
        self.config.height = new_size[1];
        self.surface.configure(&self.device, &self.config);
    }

    pub fn get_encoder(&self) -> CommandEncoder {
        self.device
            .create_command_encoder(&CommandEncoderDescriptor { label: None })
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
        Self { data, buffers }
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

    pub struct WGPUBuffer {
        buffer: Buffer,
    }

    impl WGPUBuffer {
        pub fn new_uniform(size: u64, context: &WGPUContext) -> Self {
            const UNIFORM_BUFFER_ALIGNMENT: u64 = 16;
            Self {
                buffer: Self::new(
                    (((size - 1) / UNIFORM_BUFFER_ALIGNMENT) + 1) * UNIFORM_BUFFER_ALIGNMENT,
                    BufferUsages::COPY_DST | BufferUsages::UNIFORM,
                    context,
                ),
            }
        }

        pub fn new_storage(size: u64, context: &WGPUContext) -> Self {
            Self {
                buffer: Self::new(
                    size,
                    BufferUsages::COPY_DST | BufferUsages::STORAGE,
                    context,
                ),
            }
        }

        pub fn new_vertex(size: u64, context: &WGPUContext) -> Self {
            Self {
                buffer: Self::new(size, BufferUsages::COPY_DST | BufferUsages::VERTEX, context),
            }
        }

        pub fn new_index(size: u64, context: &WGPUContext) -> Self {
            Self {
                buffer: Self::new(size, BufferUsages::COPY_DST | BufferUsages::INDEX, context),
            }
        }

        pub fn size(&self) -> u64 {
            self.buffer.size()
        }

        fn new(size: u64, usage: BufferUsages, context: &WGPUContext) -> Buffer {
            context.device().create_buffer(&BufferDescriptor {
                label: None,
                size,
                usage,
                mapped_at_creation: false,
            })
        }

        pub fn resize(&mut self, new_size: u64, context: &WGPUContext) {
            if self.size() < new_size {
                self.buffer.destroy();
                self.buffer = Self::new(new_size, self.buffer.usage(), context);
            }
        }

        pub fn destroy(&self) {
            self.buffer.destroy();
        }

        pub fn write_iter<'a, I, T>(&mut self, mut data: I, context: &WGPUContext)
        where
            I: Iterator<Item = &'a T>,
            T: Pod + Sized,
        {
            let mut buffer_slice = context
                .queue()
                .write_buffer_with(&self.buffer, 0, NonZero::new(self.size()).unwrap())
                .expect("Could not write to buffer");
            let mut buffer_iter = buffer_slice.chunks_mut(std::mem::size_of::<T>());
            loop {
                match (buffer_iter.next(), data.next()) {
                    (Some(buffer_slice), Some(data_elem)) => {
                        buffer_slice.copy_from_slice(bytemuck::bytes_of(data_elem))
                    }
                    (_, None) => break,
                    _ => panic!("Size of data is greater than size of buffer"),
                }
            }
        }

        pub fn write_data(&mut self, data: &[u8], context: &WGPUContext) {
            self.resize(data.len() as u64, context);
            context.queue().write_buffer(&self.buffer, 0, data);
        }
    }

    impl std::ops::Deref for WGPUBuffer {
        type Target = Buffer;
        fn deref(&self) -> &Self::Target {
            &self.buffer
        }
    }

    impl std::ops::DerefMut for WGPUBuffer {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.buffer
        }
    }

    impl Drop for WGPUBuffer {
        fn drop(&mut self) {
            self.destroy();
        }
    }
}
