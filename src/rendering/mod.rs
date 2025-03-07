pub mod point {
	use std::sync::LazyLock;
	use std::borrow::Cow;

	use crate::wgpu_context::{WGPUContext, SHADER_DIRECTORY, VecAndBuffer, DataAndBuffer};
	use wgpu::*;

	use bytemuck::{Zeroable, Pod};

	#[repr(C)]
	#[derive(Zeroable, Pod, Clone, Copy)]
	pub struct Uniform{
		pub size: [f32;2],
	}

	#[repr(C)]
	#[derive(Zeroable, Pod, Clone, Copy, Debug)]
	pub struct Point {
		pub color: [f32;4],
		pub position: [f32;2],
	}

	static POINT_SHADER_SOURCE: LazyLock<String> = LazyLock::new(|| {
		std::fs::read_to_string(SHADER_DIRECTORY.to_owned() + "points.wgsl")
			.expect("Could not read shader source")
	});

	pub struct PointRenderer {
		points: VecAndBuffer<Point>,
		uniform: DataAndBuffer<Uniform>,
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
							array_stride: std::mem::size_of::<Point>() as u64,
							step_mode: VertexStepMode::Vertex,
							attributes: &vertex_attr_array![0 => Float32x4, 1 => Float32x2],
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

			let points = VecAndBuffer::new(points, BufferUsages::VERTEX, context);

			let uniform = Uniform {
				size: [context.config().width as f32, context.config().height as f32],
			};
			let uniform = DataAndBuffer::new(uniform, BufferUsages::UNIFORM, context);

			let bind_group = context.device().create_bind_group(&BindGroupDescriptor{
				label: Some("Points Uniform Buffer"),
				layout: &render_pipeline.get_bind_group_layout(0),
				entries: &[
					BindGroupEntry{
						binding: 0,
						resource: uniform.buffer.as_entire_binding(),
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
			// println!("{:?}", (self.points.buffer.size(), self.points.data.len()));

			render_pass.set_pipeline(&self.render_pipeline);
			render_pass.set_bind_group(0, &self.bind_group, &[]);
			render_pass.set_vertex_buffer(0, self.points.buffer.slice(..));
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

use super::wgpu_context::WGPUContext;

pub trait Render {
	fn render(&self, context: WGPUContext);
}
