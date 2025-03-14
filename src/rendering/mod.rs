mod point {
	use crate::wgpu_context::*;
	use wgpu::*;

	use crate::shader_manager::*;

	use bytemuck::{Zeroable, Pod};
	use crate::vertex_buffer_layout;

	#[repr(C)]
	#[derive(Zeroable, Pod, Clone, Copy)]
	pub struct Uniform {
		pub size: [f32;2],
	}

	impl BufferData for Uniform {
		type Buffers = WGPUBuffer;
		fn create_buffers(&self, context: &WGPUContext) -> Self::Buffers {
			Self::Buffers::new_uniform(std::mem::size_of::<Self>() as u64, context)
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
		type Buffers = (WGPUBuffer, WGPUBuffer);
		fn create_buffers(&self, context: &WGPUContext) -> Self::Buffers {
			(
				WGPUBuffer::new_vertex((std::mem::size_of::<[f32;4]>() * self.len()) as u64, context),
				WGPUBuffer::new_vertex((std::mem::size_of::<[f32;2]>() * self.len()) as u64, context),
			)
		}
		fn fill_buffers(&self, buffers: &mut Self::Buffers, context: &WGPUContext) {
			buffers.0.write_iter(self.iter().map(|x| &x.color), context);
			buffers.1.write_iter(self.iter().map(|x| &x.position), context);
		}
	}

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
					buffers: &vertex_buffer_layout!(
						([f32;4], Vertex, &vertex_attr_array!(0 => Float32x4)),
						([f32;2], Vertex, &vertex_attr_array!(1 => Float32x2))
					)
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
			self.uniform.update_buffer(context);
		}

		pub fn render(&mut self, target: &TextureView, context: &WGPUContext, shader_manager: &ShaderManager) {
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

mod triangle {
	use bytemuck::{Pod, Zeroable};
	use crate::wgpu_context::*;

	use wgpu::*;

	use crate::shader_manager::*;

	use super::point::Point;
	use crate::vertex_buffer_layout;
	
	#[repr(C)]
	#[derive(Debug, Clone, Copy, Zeroable, Pod)]
	pub struct Uniform {
		screen_size: [f32;2],
	}

	impl BufferData for Uniform {
		type Buffers = WGPUBuffer;
		fn create_buffers(&self, context: &WGPUContext) -> Self::Buffers {
			WGPUBuffer::new_uniform(std::mem::size_of::<Self>() as u64, context)
		}
		fn fill_buffers(&self, buffers: &mut Self::Buffers, context: &WGPUContext) {
			buffers.write_data(bytemuck::bytes_of(self), context);
		}
	}

	pub struct Triangle {
		pub points: [Point;3]
	}

	impl BufferData for Vec<Triangle> {
		type Buffers = (WGPUBuffer, WGPUBuffer);
		fn create_buffers(&self, context: &WGPUContext) -> Self::Buffers {
			(
				WGPUBuffer::new_vertex((std::mem::size_of::<[f32;4]>() * self.len() * 3) as u64, context),
				WGPUBuffer::new_vertex((std::mem::size_of::<[f32;2]>() * self.len() * 3) as u64, context),
			)
		}
		fn fill_buffers(&self, buffers: &mut Self::Buffers, context: &WGPUContext) {
			buffers.0.write_iter(self.iter().flat_map(|x| x.points.iter().map(|x| &x.color)), context);
			buffers.1.write_iter(self.iter().flat_map(|x| x.points.iter().map(|x| &x.position)), context);
		}
	}

	pub struct TriangleListRenderer {
		triangles: BufferAndData<Vec<Triangle>>,
		uniform: BufferAndData<Uniform>,
		bind_group: BindGroup,
	}

	impl TriangleListRenderer {
		pub fn new(data: Vec<Triangle>, context: &WGPUContext, shader_manager: &ShaderManager) -> Self {
			let triangles = BufferAndData::new(data, context);
			let uniform = BufferAndData::new(
				Uniform {
					screen_size: [context.config().width as f32, context.config().height as f32],
				}
				, context
			);

			let bind_group_layout = context.device().create_bind_group_layout(&BindGroupLayoutDescriptor{
				label: None,
				entries: &[
					BindGroupLayoutEntry {
						binding: 0,
						visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
						ty: BindingType::Buffer{
							ty: BufferBindingType::Uniform,
							has_dynamic_offset: false,
							min_binding_size: None,
						},
						count: None,
					}
				],
			});

			let pipeline_layout = context.device().create_pipeline_layout(&PipelineLayoutDescriptor{
				label: None,
				bind_group_layouts: &[
					&bind_group_layout,
				],
				push_constant_ranges: &[],
			});
			
			let render_pipeline_template = RenderPipelineDescriptorTemplate{
				label: Some("Triangle Pipeline"),
				layout: Some(pipeline_layout),
				vertex: VertexStateTemplate{
					module_path: "points.wgsl",
					entry_point: None,
					buffers: &vertex_buffer_layout!(
						([f32;4], Vertex, &vertex_attr_array![0 => Float32x4]),
						([f32;2], Vertex, &vertex_attr_array![1 => Float32x2]),
					)
				},
				primitive: PrimitiveState {
					topology: PrimitiveTopology::TriangleList,
					..Default::default()
				},
				depth_stencil: None,
				multisample: Default::default(),
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
				multiview: None,
				cache: None,
			};
			shader_manager.register_render_pipeline("triangles", render_pipeline_template);

			let bind_group = context.device().create_bind_group(&BindGroupDescriptor{
				label: None,
				layout: &bind_group_layout,
				entries: &[
					BindGroupEntry{
						binding: 0,
						resource: uniform.buffers.as_entire_binding(),
					},
				],
			});

			Self {
				triangles,
				uniform,
				bind_group,
			}
		}

		pub fn set_uniform(&mut self, context: &WGPUContext) {
			self.uniform.data.screen_size = [context.config().width as f32, context.config().height as f32];
			self.uniform.update_buffer(context);
		}

		pub fn render(&mut self, target: &TextureView, context: &WGPUContext, shader_manager: &ShaderManager) {
			let mut encoder = context.get_encoder();
			let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor{
				label: None,
				color_attachments: &[
					Some(RenderPassColorAttachment{
						view: target,
						resolve_target: None,
						ops: Operations {
							load: LoadOp::Load,
							store: StoreOp::Store,
						}
					})
				],
				..Default::default()
			});

			let pipeline = shader_manager.get_render_pipeline("triangles", context);

			render_pass.set_pipeline(pipeline);
			render_pass.set_bind_group(0, &self.bind_group, &[]);
			render_pass.set_vertex_buffer(0, self.triangles.buffers.0.slice(..));
			render_pass.set_vertex_buffer(1, self.triangles.buffers.1.slice(..));
			render_pass.draw(0..(self.triangles.data.len() * 3) as u32, 0..1);

			std::mem::drop(render_pass);
			context.queue().submit([encoder.finish()]);
		}
	}
}

mod rect {
	use bytemuck::{Pod, Zeroable};
	use crate::wgpu_context::*;

	use wgpu::*;

	use crate::shader_manager::*;

	use crate::vertex_buffer_layout;
	
	#[repr(C)]
	#[derive(Debug, Clone, Copy, Zeroable, Pod)]
	pub struct Uniform {
		screen_size: [f32;2],
	}

	impl BufferData for Uniform {
		type Buffers = WGPUBuffer;
		fn create_buffers(&self, context: &WGPUContext) -> Self::Buffers {
			WGPUBuffer::new_uniform(std::mem::size_of::<Self>() as u64, context)
		}
		fn fill_buffers(&self, buffers: &mut Self::Buffers, context: &WGPUContext) {
			buffers.write_data(bytemuck::bytes_of(self), context);
		}
	}

	pub struct CenterRect {
		pub color: [f32;4],
		pub center: [f32;2],
		pub size: [f32; 2],
		pub rotation: f32,
	}

	impl BufferData for Vec<CenterRect> {
		type Buffers = (WGPUBuffer, WGPUBuffer, WGPUBuffer, WGPUBuffer);
		fn create_buffers(&self, context: &WGPUContext) -> Self::Buffers {
			(
				WGPUBuffer::new_vertex((std::mem::size_of::<[f32;4]>() * self.len()) as u64, context),
				WGPUBuffer::new_vertex((std::mem::size_of::<[f32;2]>() * self.len()) as u64, context),
				WGPUBuffer::new_vertex((std::mem::size_of::<[f32;2]>() * self.len()) as u64, context),
				WGPUBuffer::new_vertex((std::mem::size_of::<f32>() * self.len()) as u64, context),
			)
		}
		fn fill_buffers(&self, buffers: &mut Self::Buffers, context: &WGPUContext) {
			buffers.0.write_iter(self.iter().map(|x| &x.color), context);
			buffers.1.write_iter(self.iter().map(|x| &x.center), context);
			buffers.2.write_iter(self.iter().map(|x| &x.size), context);
			buffers.3.write_iter(self.iter().map(|x| &x.rotation), context);
		}
	}

	pub struct RectangleRenderer {
		rectangles: BufferAndData<Vec<CenterRect>>,
		uniform: BufferAndData<Uniform>,
		bind_group: BindGroup,
	}

	impl RectangleRenderer {
		pub fn new(data: Vec<CenterRect>, context: &WGPUContext, shader_manager: &ShaderManager) -> Self {
			let rectangles = BufferAndData::new(data, context);
			let uniform = BufferAndData::new(
				Uniform {
					screen_size: [context.config().width as f32, context.config().height as f32],
				}
				, context
			);

			let bind_group_layout = context.device().create_bind_group_layout(&BindGroupLayoutDescriptor{
				label: None,
				entries: &[
					BindGroupLayoutEntry {
						binding: 0,
						visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
						ty: BindingType::Buffer{
							ty: BufferBindingType::Uniform,
							has_dynamic_offset: false,
							min_binding_size: None,
						},
						count: None,
					}
				],
			});

			let pipeline_layout = context.device().create_pipeline_layout(&PipelineLayoutDescriptor{
				label: None,
				bind_group_layouts: &[
					&bind_group_layout,
				],
				push_constant_ranges: &[],
			});
			
			let render_pipeline_template = RenderPipelineDescriptorTemplate{
				label: Some("Rectangle Pipeline"),
				layout: Some(pipeline_layout),
				vertex: VertexStateTemplate{
					module_path: "rect.wgsl",
					entry_point: None,
					buffers: &vertex_buffer_layout!(
						([f32;4], Instance, &vertex_attr_array![0 => Float32x4]),
						([f32;2], Instance, &vertex_attr_array![1 => Float32x2]),
						([f32;2], Instance, &vertex_attr_array![2 => Float32x2]),
						(f32, Instance, &vertex_attr_array![3 => Float32]),
					)
				},
				primitive: PrimitiveState {
					topology: PrimitiveTopology::TriangleStrip,
					..Default::default()
				},
				depth_stencil: None,
				multisample: Default::default(),
				fragment: Some(FragmentStateTemplate{
					module_path: "rect.wgsl",
					entry_point: None,
					targets: Box::new([
						Some(ColorTargetState{
							format: context.config().format,
							blend: None,
							write_mask: ColorWrites::ALL,
						})
					]),
				}),
				multiview: None,
				cache: None,
			};
			shader_manager.register_render_pipeline("rects", render_pipeline_template);

			let bind_group = context.device().create_bind_group(&BindGroupDescriptor{
				label: None,
				layout: &bind_group_layout,
				entries: &[
					BindGroupEntry{
						binding: 0,
						resource: uniform.buffers.as_entire_binding(),
					},
				],
			});

			Self {
				rectangles,
				uniform,
				bind_group,
			}
		}

		pub fn set_uniform(&mut self, context: &WGPUContext) {
			self.uniform.data.screen_size = [context.config().width as f32, context.config().height as f32];
			self.uniform.update_buffer(context);
		}

		pub fn render(&mut self, target: &TextureView, context: &WGPUContext, shader_manager: &ShaderManager) {
			let mut encoder = context.get_encoder();
			let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor{
				label: None,
				color_attachments: &[
					Some(RenderPassColorAttachment{
						view: target,
						resolve_target: None,
						ops: Operations {
							load: LoadOp::Load,
							store: StoreOp::Store,
						}
					})
				],
				..Default::default()
			});

			let pipeline = shader_manager.get_render_pipeline("rects", context);

			render_pass.set_pipeline(pipeline);
			render_pass.set_bind_group(0, &self.bind_group, &[]);
			render_pass.set_vertex_buffer(0, self.rectangles.buffers.0.slice(..));
			render_pass.set_vertex_buffer(1, self.rectangles.buffers.1.slice(..));
			render_pass.set_vertex_buffer(2, self.rectangles.buffers.2.slice(..));
			render_pass.set_vertex_buffer(3, self.rectangles.buffers.3.slice(..));
			render_pass.draw(0..4 as u32, 0..self.rectangles.data.len() as u32);

			std::mem::drop(render_pass);
			context.queue().submit([encoder.finish()]);
		}

		pub fn rects_mut(&mut self) -> &mut Vec<CenterRect> {
			&mut self.rectangles.data
		}

		pub fn update_rects(&mut self, context: &WGPUContext) {
			self.rectangles.update_buffer(context);
		}
	}
}

mod circle {
	use wgpu::*;
	use crate::wgpu_context::{BufferAndData, BufferData, WGPUBuffer, WGPUContext};
	use bytemuck::{Pod, Zeroable};
	use std::mem::size_of;

	use crate::shader_manager::*;

	use crate::shader_manager::ShaderManager;
	use crate::vertex_buffer_layout;

	#[derive(Pod, Zeroable, Clone, Copy)]
	#[repr(C)]
	pub struct Uniform {
		screen_size: [f32;2],
	}
	
	impl BufferData for Uniform {
		type Buffers = WGPUBuffer;
		fn create_buffers(&self, context: &WGPUContext) -> Self::Buffers {
			WGPUBuffer::new_uniform(size_of::<Self>() as u64, context)
		}
		fn fill_buffers(&self, buffers: &mut Self::Buffers, context: &WGPUContext) {
			buffers.write_data(bytemuck::bytes_of(self), context);
		}
	}

	#[derive(Pod, Zeroable, Clone, Copy)]
	#[repr(C)]
	pub struct Circle {
		pub color: [f32;4],
		pub position: [f32;2],
		pub radius: f32,
	}

	impl BufferData for Vec<Circle> {
		type Buffers = (WGPUBuffer, WGPUBuffer, WGPUBuffer);
		fn create_buffers(&self, context: &WGPUContext) -> Self::Buffers {
			(
				WGPUBuffer::new_vertex((size_of::<[f32;4]>() * self.len()) as u64, context),
				WGPUBuffer::new_vertex((size_of::<[f32;2]>() * self.len()) as u64, context),
				WGPUBuffer::new_vertex((size_of::<f32>()     * self.len()) as u64, context),
			)
		}
		fn fill_buffers(&self, buffers: &mut Self::Buffers, context: &WGPUContext) {
			buffers.0.write_iter(self.iter().map(|x| &x.color   ), context);
			buffers.1.write_iter(self.iter().map(|x| &x.position), context);
			buffers.2.write_iter(self.iter().map(|x| &x.radius  ), context);
		}
	}

	pub struct CircleRenderer {
		circles: BufferAndData<Vec<Circle>>,
		uniform: BufferAndData<Uniform>,
		bind_group: BindGroup,
	}

	impl CircleRenderer {
		pub fn new(data: Vec<Circle>, context: &WGPUContext, shader_manager: &ShaderManager) -> Self {
			let circles = BufferAndData::new(data, context);
			let uniform = BufferAndData::new(
				Uniform {
					screen_size: [context.config().width as f32, context.config().height as f32],
				}
				, context
			);

			let bind_group_layout = context.device().create_bind_group_layout(&BindGroupLayoutDescriptor{
				label: None,
				entries: &[
					BindGroupLayoutEntry {
						binding: 0,
						visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
						ty: BindingType::Buffer{
							ty: BufferBindingType::Uniform,
							has_dynamic_offset: false,
							min_binding_size: None,
						},
						count: None,
					}
				],
			});

			let pipeline_layout = context.device().create_pipeline_layout(&PipelineLayoutDescriptor{
				label: None,
				bind_group_layouts: &[
					&bind_group_layout,
				],
				push_constant_ranges: &[],
			});
			
			let render_pipeline_template = RenderPipelineDescriptorTemplate{
				label: Some("Circle Pipeline"),
				layout: Some(pipeline_layout),
				vertex: VertexStateTemplate{
					module_path: "circle.wgsl",
					entry_point: None,
					buffers: &vertex_buffer_layout!(
						([f32;4], Instance, &vertex_attr_array![0 => Float32x4]),
						([f32;2], Instance, &vertex_attr_array![1 => Float32x2]),
						(f32, Instance, &vertex_attr_array![2 => Float32]),
					)
				},
				primitive: PrimitiveState {
					topology: PrimitiveTopology::TriangleStrip,
					..Default::default()
				},
				depth_stencil: None,
				multisample: Default::default(),
				fragment: Some(FragmentStateTemplate{
					module_path: "circle.wgsl",
					entry_point: None,
					targets: Box::new([
						Some(ColorTargetState{
							format: context.config().format,
							blend: None,
							write_mask: ColorWrites::ALL,
						})
					]),
				}),
				multiview: None,
				cache: None,
			};
			shader_manager.register_render_pipeline("circle", render_pipeline_template);

			let bind_group = context.device().create_bind_group(&BindGroupDescriptor{
				label: None,
				layout: &bind_group_layout,
				entries: &[
					BindGroupEntry{
						binding: 0,
						resource: uniform.buffers.as_entire_binding(),
					},
				],
			});

			Self {
				circles,
				uniform,
				bind_group,
			}
		}

		pub fn set_uniform(&mut self, context: &WGPUContext) {
			self.uniform.data.screen_size = [context.config().width as f32, context.config().height as f32];
			println!("uniform data: {:?}", self.uniform.data.screen_size);
			self.uniform.update_buffer(context);
		}

		pub fn render(&mut self, target: &TextureView, context: &WGPUContext, shader_manager: &ShaderManager) {
			let mut encoder = context.get_encoder();
			let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor{
				label: None,
				color_attachments: &[
					Some(RenderPassColorAttachment{
						view: target,
						resolve_target: None,
						ops: Operations {
							load: LoadOp::Load,
							store: StoreOp::Store,
						}
					})
				],
				..Default::default()
			});

			let pipeline = shader_manager.get_render_pipeline("circle", context);

			render_pass.set_pipeline(pipeline);
			render_pass.set_bind_group(0, &self.bind_group, &[]);
			render_pass.set_vertex_buffer(0, self.circles.buffers.0.slice(..));
			render_pass.set_vertex_buffer(1, self.circles.buffers.1.slice(..));
			render_pass.set_vertex_buffer(2, self.circles.buffers.2.slice(..));
			render_pass.draw(0..4 as u32, 0..self.circles.data.len() as u32);

			std::mem::drop(render_pass);
			context.queue().submit([encoder.finish()]);
		}

		pub fn circles_mut(&mut self) -> &mut Vec<Circle> {
			&mut self.circles.data
		}

		pub fn update_circles(&mut self, context: &WGPUContext) {
			self.circles.update_buffer(context);
		}
	}
}

pub use point::*;
pub use triangle::*;
pub use rect::*;
pub use circle::*;
#[macro_export]
macro_rules! vertex_buffer_layout {
	($(($stridetype: ty, $mode: ident, $attributes: expr)),+ $(,)?) => {
		[
		$(::wgpu::VertexBufferLayout {
			array_stride: ::std::mem::size_of::<$stridetype>() as u64,
			step_mode: ::wgpu::VertexStepMode::$mode,
			attributes: $attributes,
		},)+
		]
	}
}
