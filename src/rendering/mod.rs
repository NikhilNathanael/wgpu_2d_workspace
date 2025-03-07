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
	#[derive(Zeroable, Pod, Clone, Copy)]
	pub struct Point {
		pub color: [f32;4],
		pub location: [f32;2],
	}

	static POINT_SHADER_SOURCE: LazyLock<String> = LazyLock::new(|| {
		std::fs::read_to_string(SHADER_DIRECTORY.to_owned() + "points.wgsl")
			.expect("Could not read shader source")
	});

	struct PointRenderer {
		points: VecAndBuffer<Point>,
		uniform: DataAndBuffer<Uniform>,
	}

	impl Point {
		pub fn render(&self, context: &WGPUContext, target: &TextureView) {
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
							array_stride: std::mem::size_of::<Self>() as u64,
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

			let vertex_buffer = context.device().create_buffer(&BufferDescriptor{
				label: Some("Point Vertex Buffer"),
				size: std::mem::size_of::<Self>() as u64,
				usage: BufferUsages::COPY_DST | BufferUsages::VERTEX,
				mapped_at_creation: false,
			});

			let uniform_buffer = context.device().create_buffer(&BufferDescriptor{
				label: Some("Point Uniform Buffer"),
				size: ((std::mem::size_of::<Uniform>() as u64 - 1) / 16 + 1) * 16,
				usage: BufferUsages::COPY_DST | BufferUsages::UNIFORM,
				mapped_at_creation: false,
			});
			
			let config = context.config();
			let uniform = Uniform {
				size: [config.width as f32, config.height as f32],
			};
			println!("{:?}", uniform.size);

			context.queue().write_buffer(&vertex_buffer, 0, bytemuck::bytes_of(self));
			context.queue().write_buffer(&uniform_buffer, 0, bytemuck::bytes_of(&uniform));

			let bind_group = context.device().create_bind_group(&BindGroupDescriptor{
				label: Some("Points Uniform Buffer"),
				layout: &render_pipeline.get_bind_group_layout(0),
				entries: &[
					BindGroupEntry{
						binding: 0,
						resource: uniform_buffer.as_entire_binding(),
					}
				],
			});
			
			let mut encoder = context.device().create_command_encoder(&CommandEncoderDescriptor{
				label: Some("Points command encoder"),
			});

			let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor{
				label: Some("Points Render pass"),
				color_attachments: &[
					Some(RenderPassColorAttachment{
						view: &target, 
						resolve_target: None,
						ops: {
							Operations{
								load: LoadOp::Clear(Color{r: 0., g:0., b:0., a:1.}),
								store: StoreOp::Store,
							}
						}
					}),
				],
				..Default::default()
			});


			render_pass.set_pipeline(&render_pipeline);
			render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
			render_pass.set_bind_group(0, &bind_group, &[]);
			render_pass.draw(0..1, 0..1);

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
