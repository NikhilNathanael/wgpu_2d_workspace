pub mod point {
	use std::sync::LazyLock;
	use std::borrow::Cow;

	use crate::wgpu_context::*;
	use wgpu::*;

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
		render_pipeline: RenderPipeline,
		bind_group: BindGroup,
	}
	
	impl PointRenderer {
		pub fn new (points: Vec<Point>, context: &WGPUContext) -> Self {
			let shader_module = context.device().create_shader_module(ShaderModuleDescriptor{
				label: Some("Point shader module"),
				source: ShaderSource::Wgsl(Cow::Borrowed(&POINT_SHADER_SOURCE)),
			});

			let render_pipeline = context.device().create_render_pipeline(&RenderPipelineDescriptor{
				label: Some("Points Render Pipeline"),
				layout: None,
				vertex: VertexState{
					module: &shader_module,
					entry_point: None,
					compilation_options: Default::default(),
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
				fragment: Some(FragmentState{
					module: &shader_module,
					entry_point: None,
					compilation_options: Default::default(),
					targets: &[
						Some(ColorTargetState{
							format: context.config().format,
							blend: None,
							write_mask: ColorWrites::ALL,
						})
					],
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
			});

			let points = BufferAndData::new(points, context);

			let uniform = Uniform {
				size: [context.config().width as f32, context.config().height as f32],
			};
			let uniform = BufferAndData::new(uniform, context);
			context.queue().submit([]);

			let bind_group = context.device().create_bind_group(&BindGroupDescriptor{
				label: Some("Points Uniform Buffer"),
				layout: &render_pipeline.get_bind_group_layout(0),
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
				render_pipeline,
			}
		}

		pub fn update_uniform(&mut self, context: &WGPUContext) {
			self.uniform.data.size = [context.config().width as f32, context.config().height as f32];
			self.uniform.update_buffer(context);
		}

		pub fn render(&self, target: &TextureView, context: &WGPUContext) {
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
							load: LoadOp::Clear(Color{r:0., g:0., b:0., a:1.}),
							store: StoreOp::Store,
						}
					}),
				],
				..Default::default()
			});

			render_pass.set_pipeline(&self.render_pipeline);
			render_pass.set_bind_group(0, &self.bind_group, &[]);
			render_pass.set_vertex_buffer(0, self.points.buffers.0.slice(..));
			render_pass.set_vertex_buffer(1, self.points.buffers.1.slice(..));
			render_pass.draw(0..(self.points.data.len()) as u32, 0..1);
			std::mem::drop(render_pass);

			context.queue().submit([encoder.finish()]);
		}
	}
}

pub mod triangle {
	struct Triangle {

	}
}

pub use point::*;

use super::wgpu_context::WGPUContext;

pub trait Render {
	fn render(&self, context: WGPUContext);
}
