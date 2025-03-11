pub mod point {
	use std::sync::LazyLock;
	use std::borrow::Cow;

	use crate::wgpu_context::*;
	use wgpu::*;

	use crate::shader_manager::*;

	use bytemuck::{Zeroable, Pod};

	#[repr(C)]
	#[derive(Zeroable, Pod, Clone, Copy)]
	pub struct Uniform {
		pub size: [f32;2],
		pub time: f32,
	}

	impl BufferData for Uniform {
		type Buffers = UniformBuffer;
		fn create_buffers(&self, context: &WGPUContext) -> Self::Buffers {
			Self::Buffers::create(std::mem::size_of::<Self>() as u64, context)
		}
		fn fill_buffers(&self, buffers: &mut Self::Buffers, context: &WGPUContext) {
			buffers.write_data(bytemuck::bytes_of(self), context);
		}
	}

	#[repr(C)]
	#[derive(Zeroable, Pod, Clone, Copy, Debug)]
	pub struct Point {
		pub color: [f32;4],
		pub position: [f32;2],
	}

	impl BufferData for Vec<Point> {
		type Buffers = (VertexBuffer, VertexBuffer);
		fn create_buffers(&self, context: &WGPUContext) -> Self::Buffers {
			(
				VertexBuffer::create((std::mem::size_of::<[f32;4]>() * self.len()) as u64, context),
				VertexBuffer::create((std::mem::size_of::<[f32;2]>() * self.len()) as u64, context),
			)
		}
		fn fill_buffers(&self, buffers: &mut Self::Buffers, context: &WGPUContext) {
			buffers.0.write_iter(self.iter().map(|x| &x.color), context);
			buffers.1.write_iter(self.iter().map(|x| &x.position), context);
		}
	}

	static POINT_SHADER_SOURCE: LazyLock<String> = LazyLock::new(|| {
		std::fs::read_to_string(SHADER_DIRECTORY.to_owned() + "points.wgsl")
			.expect("Could not read shader source")
	});

	pub struct PointRenderer {
		points: BufferAndData<Vec<Point>>,
		uniform: BufferAndData<Uniform>,
		bind_group: BindGroup,
	}
	
	impl PointRenderer {
		pub fn new (points: Vec<Point>, context: &WGPUContext, shader_manager: &ShaderManager) -> Self {
			let bind_group_layout = context.device().create_bind_group_layout(&BindGroupLayoutDescriptor{
				label: Some("point renderer bind group layout 1"),
				entries : &[
					BindGroupLayoutEntry {
						binding: 0,
						visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
						ty: BindingType::Buffer {
							ty: BufferBindingType::Uniform,
							has_dynamic_offset: false,
							min_binding_size:None
						},
						count: None,
					}
				],
			});
			let pipeline_layout = context.device().create_pipeline_layout(&PipelineLayoutDescriptor{
				label: Some("Points pipeline layout"),
				bind_group_layouts: &[&bind_group_layout],
				push_constant_ranges: &[]
			});

			let descriptor_template = RenderPipelineDescriptorTemplate{
				label: Some("Points Render Pipeline"),
				layout: Some(pipeline_layout.clone()),
				vertex: VertexStateTemplate{
					module_path: "points.wgsl",
					entry_point: None,
					buffers: &[
						VertexBufferLayout{
							array_stride: std::mem::size_of::<[f32;4]>() as u64,
							step_mode: VertexStepMode::Vertex,
							attributes: &vertex_attr_array![0 => Float32x4],
						},
						VertexBufferLayout{
							array_stride: std::mem::size_of::<[f32;2]>() as u64,
							step_mode: VertexStepMode::Vertex,
							attributes: &vertex_attr_array![1 => Float32x2],
						}
					],
				},
				fragment: Some(FragmentStateTemplate{
					module_path: "points.wgsl",
					entry_point: None,
					targets: Box::new([
						Some(ColorTargetState{
							format: context.config().format,
							blend: None,
							write_mask: ColorWrites::ALL,
						})
					]),
				}),
				primitive: PrimitiveState {
					topology: PrimitiveTopology::PointList,
					strip_index_format: None,
					front_face: FrontFace::Ccw,
					cull_mode: None,
					..Default::default()
				},
				depth_stencil: None,
				multisample: Default::default(),
				multiview: None,
				cache: None,
			};
			shader_manager.register_render_pipeline("Point Renderer Pipeline", descriptor_template);

			let points = BufferAndData::new(points, context);

			let uniform = Uniform {
				size: [context.config().width as f32, context.config().height as f32],
				time: 0.
			};
			let uniform = BufferAndData::new(uniform, context);
			context.queue().submit([]);

			let bind_group = context.device().create_bind_group(&BindGroupDescriptor{
				label: Some("Points Uniform Buffer"),
				layout: &shader_manager.get_render_pipeline("Point Renderer Pipeline", context).get_bind_group_layout(0),
				entries: &[
					BindGroupEntry{
						binding: 0,
						resource: uniform.buffers.as_entire_binding(),
					}
				],
			});
			
			Self {
				points,
				uniform,
				bind_group,
			}
		}

		pub fn update_size(&mut self, context: &WGPUContext) {
			self.uniform.data.size = [context.config().width as f32, context.config().height as f32];
		}

		pub fn update_time(&mut self, time: f32) {
			self.uniform.data.time = time;
		}

		pub fn render(&mut self, target: &TextureView, context: &WGPUContext, shader_manager: &ShaderManager) {
			self.uniform.update_buffer(context);
			let mut encoder = context.device().create_command_encoder(&CommandEncoderDescriptor{
				label: Some("Points command encoder"),
			});
			let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor{
				label: Some("Points render pass"),
				color_attachments: &[
					Some(RenderPassColorAttachment{
						view: target,
						resolve_target: None,
						ops: Operations {
							load: LoadOp::Load,
							store: StoreOp::Store,
						}
					}),
				],
				..Default::default()
			});

			render_pass.set_pipeline(shader_manager.get_render_pipeline("Point Renderer Pipeline", context));
			render_pass.set_bind_group(0, &self.bind_group, &[]);
			render_pass.set_vertex_buffer(0, self.points.buffers.0.slice(..));
			render_pass.set_vertex_buffer(1, self.points.buffers.1.slice(..));
			render_pass.draw(0..(self.points.data.len()) as u32, 0..1);
			std::mem::drop(render_pass);

			context.queue().submit([encoder.finish()]);
		}

		pub fn points_mut(&mut self) -> &mut Vec<Point> {
			&mut self.points.data
		}

		pub fn update_points_buffer(&mut self, context: &WGPUContext) {
			self.points.update_buffer(context);
		}
	}

	pub fn create_circle_point_list (num_points: usize, radius: f32, center_position: [f32;2]) -> Vec<Point> {
		(0..num_points).map(|i| {
			let angle = i as f32 * 2. * std::f32::consts::PI / num_points as f32;
			Point{
				position: [angle.cos() * radius + center_position[0], angle.sin() * radius + center_position[1]],
				color: [1., 1., 1., 1.],
			}
		}).collect::<Vec<_>>()
	}
}

pub mod triangle {
	use super::*;
	use crate::wgpu_context::*;
	use bytemuck::{Zeroable, Pod};

	#[repr(C)]
	#[derive(Zeroable, Pod, Clone, Copy)]
	pub struct Uniform {
		pub size: [f32;2],
	}

	impl BufferData for Uniform {
		type Buffers = UniformBuffer;
		fn create_buffers(&self, context: &WGPUContext) -> Self::Buffers {
			Self::Buffers::create(std::mem::size_of::<Self>() as u64, context)
		}
		fn fill_buffers(&self, buffers: &mut Self::Buffers, context: &WGPUContext) {
			buffers.write_data(bytemuck::bytes_of(self), context);
		}
	}
	
	impl Uniform {

	}

	struct Triangle {
		points: [Point;3],
		uniform: BufferAndData<Uniform>,
	}
}

pub use point::*;

use super::wgpu_context::WGPUContext;

pub trait Render {
	fn render(&self, context: WGPUContext);
}
