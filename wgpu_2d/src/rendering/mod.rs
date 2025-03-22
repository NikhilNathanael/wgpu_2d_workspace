mod point {
	use crate::wgpu_context::*;
	use wgpu::*;
	use super::Uniform;

	use crate::shader_manager::*;

	use derive::VertexBufferData;

	use bytemuck::{Zeroable, Pod};
	use crate::vertex_buffer_layout;

	#[repr(C)]
	#[derive(Zeroable, Pod, Clone, Copy, Debug, VertexBufferData)]
	pub struct Point {
		pub color: [f32;4],
		pub position: [f32;2],
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
							blend: Some(BlendState{
								color: BlendComponent{
									src_factor: BlendFactor::One,
									dst_factor: BlendFactor::OneMinusSrcAlpha,
									operation: BlendOperation::Add,
								},
								alpha: BlendComponent{
									src_factor: BlendFactor::One,
									dst_factor: BlendFactor::OneMinusSrcAlpha,
									operation: BlendOperation::Add,
								},
							}),
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
				screen_size: [context.config().width as f32, context.config().height as f32],
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
			self.uniform.data.screen_size = [context.config().width as f32, context.config().height as f32];
			self.uniform.update_buffer(context);
		}

		pub fn render(&mut self, render_pass: &mut RenderPass, context: &WGPUContext, shader_manager: &ShaderManager) {
			render_pass.set_pipeline(shader_manager.get_render_pipeline("Point Renderer Pipeline", context));
			render_pass.set_bind_group(0, &self.bind_group, &[]);
			render_pass.set_vertex_buffer(0, self.points.buffers.0.slice(..));
			render_pass.set_vertex_buffer(1, self.points.buffers.1.slice(..));
			render_pass.draw(0..(self.points.data.len()) as u32, 0..1);
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
	use crate::wgpu_context::*;
	use super::Uniform;

	use wgpu::*;

	use crate::shader_manager::*;

	use super::point::Point;
	use crate::vertex_buffer_layout;

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
							blend: Some(BlendState{
								color: BlendComponent{
									src_factor: BlendFactor::One,
									dst_factor: BlendFactor::OneMinusSrcAlpha,
									operation: BlendOperation::Add,
								},
								alpha: BlendComponent{
									src_factor: BlendFactor::One,
									dst_factor: BlendFactor::OneMinusSrcAlpha,
									operation: BlendOperation::Add,
								},
							}),
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

		pub fn render(&mut self, render_pass: &mut RenderPass, context: &WGPUContext, shader_manager: &ShaderManager) {
			render_pass.set_pipeline(shader_manager.get_render_pipeline("triangles", context));
			render_pass.set_bind_group(0, &self.bind_group, &[]);
			render_pass.set_vertex_buffer(0, self.triangles.buffers.0.slice(..));
			render_pass.set_vertex_buffer(1, self.triangles.buffers.1.slice(..));
			render_pass.draw(0..(self.triangles.data.len() * 3) as u32, 0..1);
		}
	}
}

mod rect {
	use derive::UniformBufferData;

	use wgpu::*;

	use super::Uniform;
	use crate::shader_manager::*;
	use crate::wgpu_context::*;
	use crate::vertex_buffer_layout;

	use bytemuck::{Pod, Zeroable};
	#[derive(Clone, Copy, Pod, Zeroable, UniformBufferData)]
	#[repr(C)]
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
							blend: Some(BlendState{
								color: BlendComponent{
									src_factor: BlendFactor::One,
									dst_factor: BlendFactor::OneMinusSrcAlpha,
									operation: BlendOperation::Add,
								},
								alpha: BlendComponent{
									src_factor: BlendFactor::One,
									dst_factor: BlendFactor::OneMinusSrcAlpha,
									operation: BlendOperation::Add,
								},
							}),
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

		pub fn render(&mut self, render_pass: &mut RenderPass, context: &WGPUContext, shader_manager: &ShaderManager) {
			render_pass.set_pipeline(shader_manager.get_render_pipeline("rects", context));
			render_pass.set_bind_group(0, &self.bind_group, &[]);
			render_pass.set_vertex_buffer(0, self.rectangles.buffers.0.slice(..));
			render_pass.set_vertex_buffer(1, self.rectangles.buffers.1.slice(..));
			render_pass.set_vertex_buffer(2, self.rectangles.buffers.2.slice(..));
			render_pass.set_vertex_buffer(3, self.rectangles.buffers.3.slice(..));
			render_pass.draw(0..4 as u32, 0..self.rectangles.data.len() as u32);
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
	use derive::VertexBufferData;
	use wgpu::*;
	use crate::wgpu_context::{BufferAndData, WGPUContext};
	use super::Uniform;
	use crate::shader_manager::*;
	use crate::vertex_buffer_layout;

	use bytemuck::{Pod, Zeroable};

	#[derive(Pod, Zeroable, Clone, Copy, VertexBufferData)]
	#[repr(C)]
	pub struct Circle {
		pub color: [f32;4],
		pub position: [f32;2],
		pub radius: f32,
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
							blend: Some(BlendState{
								color: BlendComponent{
									src_factor: BlendFactor::One,
									dst_factor: BlendFactor::OneMinusSrcAlpha,
									operation: BlendOperation::Add,
								},
								alpha: BlendComponent{
									src_factor: BlendFactor::One,
									dst_factor: BlendFactor::OneMinusSrcAlpha,
									operation: BlendOperation::Add,
								},
							}),
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
			self.uniform.update_buffer(context);
		}

		pub fn render(&mut self, render_pass: &mut RenderPass, context: &WGPUContext, shader_manager: &ShaderManager) {
			render_pass.set_pipeline(shader_manager.get_render_pipeline("circle", context));
			render_pass.set_bind_group(0, &self.bind_group, &[]);
			render_pass.set_vertex_buffer(0, self.circles.buffers.0.slice(..));
			render_pass.set_vertex_buffer(1, self.circles.buffers.1.slice(..));
			render_pass.set_vertex_buffer(2, self.circles.buffers.2.slice(..));
			render_pass.draw(0..4 as u32, 0..self.circles.data.len() as u32);		
		}

		pub fn circles_mut(&mut self) -> &mut Vec<Circle> {
			&mut self.circles.data
		}

		pub fn update_circles(&mut self, context: &WGPUContext) {
			self.circles.update_buffer(context);
		}
	}
}

mod texture {
	use derive::UniformBufferData;
	use crate::rendering::{CenterRect, Uniform};
	use crate::wgpu_context::{WGPUContext, BufferAndData};
	use crate::shader_manager::{RenderPipelineDescriptorTemplate, VertexStateTemplate, FragmentStateTemplate, ShaderManager};
	use wgpu::*;
	use bytemuck::{Pod, Zeroable};

	pub struct TextureRenderer {
		rect: BufferAndData<CenterRect>,
		uniform: BufferAndData<Uniform>,
		texture: Texture,
		view: TextureView,
		sampler: Sampler,
		bind_group: BindGroup,
	}

	impl TextureRenderer {
		pub fn new (context: &WGPUContext, shader_manager: &ShaderManager) -> Self {
			let uniform = BufferAndData::new(Uniform {
				screen_size: [context.config().width as f32, context.config().height as f32],
			}, context);
			let rect = BufferAndData::new(CenterRect{
				color: [0., 0., 0., 1.],
				center: [400., 300.],
				size: [300., 250.], 
				rotation: 0.,
			}, context);

			// Texture data
			let x: [u8;4] = [255, 0, 0, 255];
			let y: [u8;4] = [255, 255, 0, 255];
			let b: [u8;4] = [0, 0, 255, 255];
			let texture_data = &[
				[b, x, x, x, x,],
				[x, y, y, y, x,],
				[x, y, x, x, x,],
				[x, y, y, x, x,],
				[x, y, x, x, x,],
				[x, y, x, x, x,],
				[x, x, x, x, x,],
			];

			// Create Texture
			let texture = context.device().create_texture(&TextureDescriptor{
				label: Some("Test Texture"),
				size: Extent3d{height: texture_data.len() as u32, width: texture_data[0].len() as u32, depth_or_array_layers: 1},
				mip_level_count: 1,
				sample_count: 1,
				dimension: TextureDimension::D2,
				format: TextureFormat::Rgba8Unorm,
				usage: TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING,
				view_formats: &[TextureFormat::Rgba8Unorm],
			});

			let texture_view = texture.create_view(&TextureViewDescriptor{
				label: Some("Texture View"),
				format: None,
				dimension: None,
				usage: None,
				aspect: TextureAspect::All,
				base_mip_level: 0,
				mip_level_count: None,
				base_array_layer: 0,
				array_layer_count: None,
			});

			// Copy data to texture
			context.queue().write_texture(
				TexelCopyTextureInfo{
					texture: &texture,
					mip_level: 0,
					origin: Origin3d{x: 0, y: 0, z: 0},
					aspect: TextureAspect::All,
				},
				bytemuck::cast_slice(texture_data),
				TexelCopyBufferLayout {
					offset: 0,
					bytes_per_row: Some((std::mem::size_of_val(texture_data) / texture_data.len()) as u32),
					rows_per_image: Some(texture_data.len() as u32),
				},
				Extent3d{
					width: texture_data[0].len() as u32, 
					height: texture_data.len() as u32, 
					depth_or_array_layers: 1
				},
			);

			// Create Sampler
			let sampler = context.device().create_sampler(&SamplerDescriptor{
				label: Some("Test Sampler"),
				address_mode_u:AddressMode::Repeat,
				address_mode_v:AddressMode::Repeat,
				address_mode_w:AddressMode::Repeat,
				mag_filter:FilterMode::Nearest,
				min_filter:FilterMode::Linear,
				mipmap_filter:FilterMode::Nearest,
				lod_min_clamp:0.,
				lod_max_clamp:0.,
				compare:None,
				anisotropy_clamp:1,
				border_color:None,
			});

			let bind_group_layout = context.device().create_bind_group_layout(&BindGroupLayoutDescriptor{
				label: Some("Texture bind group layout"),
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
					},
					BindGroupLayoutEntry {
						binding: 1,
						visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
						ty: BindingType::Buffer{
							ty: BufferBindingType::Uniform,
							has_dynamic_offset: false,
							min_binding_size: None,
						},
						count: None,
					},
					BindGroupLayoutEntry {
						binding: 2,
						visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
						ty: BindingType::Texture{
							sample_type: TextureSampleType::Float{filterable: true},
							view_dimension: TextureViewDimension::D2,
							multisampled: false,
						},
						count: None,
					},
					BindGroupLayoutEntry {
						binding: 3,
						visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
						ty: BindingType::Sampler(SamplerBindingType::Filtering),
						count: None,
					},
				],
			});

			let pipeline_layout = context.device().create_pipeline_layout(&PipelineLayoutDescriptor{
				label: Some("Texture pipeline layout"),
				bind_group_layouts: &[
					&bind_group_layout,
				],
				push_constant_ranges: &[],
			});
			
			let render_pipeline_template = RenderPipelineDescriptorTemplate{
				label: Some("Texture quad Pipeline"),
				layout: Some(pipeline_layout),
				vertex: VertexStateTemplate{
					module_path: "texture.wgsl",
					entry_point: None,
					buffers: &[],
				},
				primitive: PrimitiveState {
					topology: PrimitiveTopology::TriangleStrip,
					..Default::default()
				},
				depth_stencil: None,
				multisample: Default::default(),
				fragment: Some(FragmentStateTemplate{
					module_path: "texture.wgsl",
					entry_point: None,
					targets: Box::new([
						Some(ColorTargetState{
							format: context.config().format,
							blend: Some(BlendState{
								color: BlendComponent{
									src_factor: BlendFactor::One,
									dst_factor: BlendFactor::OneMinusSrcAlpha,
									operation: BlendOperation::Add,
								},
								alpha: BlendComponent{
									src_factor: BlendFactor::One,
									dst_factor: BlendFactor::OneMinusSrcAlpha,
									operation: BlendOperation::Add,
								},
							}),
							write_mask: ColorWrites::ALL,
						})
					]),
				}),
				multiview: None,
				cache: None,
			};

			shader_manager.register_render_pipeline("texture", render_pipeline_template);

			let bind_group = context.device().create_bind_group(&BindGroupDescriptor{
				label: Some("Texture bind group"),
				layout: &bind_group_layout,
				entries: &[
					BindGroupEntry{
						binding: 0,
						resource: uniform.buffers.as_entire_binding(),
					},
					BindGroupEntry{
						binding: 1,
						resource: rect.buffers.as_entire_binding(),
					},
					BindGroupEntry{
						binding: 2,
						resource: BindingResource::TextureView(&texture_view),
					},
					BindGroupEntry{
						binding: 3,
						resource: BindingResource::Sampler(&sampler),
					},
				],
			});

			Self {
				uniform, 
				rect,
				texture,
				view: texture_view,
				sampler,
				bind_group,
			}
		}

		pub fn set_uniform(&mut self, context: &WGPUContext) {
			self.uniform.data.screen_size = [context.config().width as f32, context.config().height as f32];
			self.uniform.update_buffer(context);
		}

		pub fn rect_mut(&mut self) -> &mut CenterRect {
			&mut self.rect.data
		}

		pub fn update_rect(&mut self, context: &WGPUContext) {
			self.rect.update_buffer(context);
		}

		pub fn render(&mut self, render_pass: &mut RenderPass, context: &WGPUContext, shader_manager: &ShaderManager) {
			render_pass.set_pipeline(shader_manager.get_render_pipeline("texture", context));
			render_pass.set_bind_group(0, &self.bind_group, &[]);
			render_pass.draw(0..4, 0..1);
		}
	}
}

use derive::UniformBufferData;
use bytemuck::{Pod, Zeroable};
#[derive(Pod, Zeroable, Clone, Copy, UniformBufferData)]
#[repr(C)]
pub struct Uniform {
	screen_size: [f32;2],
}

pub use point::*;
pub use triangle::*;
pub use rect::*;
pub use circle::*;
pub use texture::*;
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

pub use scene_manager::*;
mod scene_manager {
	use super::*;
	use crate::wgpu_context::{WGPUContext, BufferAndData};
	use crate::shader_manager::ShaderManager;
	use rand::{Rng, thread_rng};

	use wgpu::*;

	pub struct SceneManager {
		scene: (PointRenderer, TriangleListRenderer, CircleRenderer, TextureRenderer),
		uniform: BufferAndData<Uniform>,
		uniform_bind_group: BindGroup,
	}

	impl SceneManager {
		pub fn new (context: &WGPUContext, shader_manager: &ShaderManager) -> Self {
			let _2d_uniform_bind_group_descriptor = BindGroupLayoutDescriptor{
				label: Some("Texture bind group layout"),
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
					},
				],
			};

			let uniform = BufferAndData::new(Uniform {
				screen_size: [context.config().width as f32, context.config().height as f32],
			}, context);

			// Create scene
			//  - Points
			let points = create_circle_point_list(200, 50.,[50. , 400.]);
			let points = PointRenderer::new(points, &context, &shader_manager);

			//  - Triangle
			let triangle = vec![
				Triangle {
					points: [
						Point {
							position: [400., 200.],
							color: [1., 0., 0., 1.],
						},
						Point {
							position: [300., 400.],
							color: [0., 1., 0., 1.],
						},
						Point {
							position: [500., 400.],
							color: [0., 0., 1., 1.],
						},
					],
				}
			];
			let triangle = TriangleListRenderer::new(triangle, &context, &shader_manager);

			let mut rng = rand::rng();

			//  - Circles
			let circles = vec![
				Circle {
					color: [rng.random_range(0.0..1.0), rng.random_range(0.0..1.0), rng.random_range(0.0..1.0), 1.],
					position: [
						0., 0.,
					],
					radius: 100.,
				}
			];
			let circles = CircleRenderer::new(circles, &context, &shader_manager);

			// Texture Renderer
			let texture_renderer = TextureRenderer::new(&context, &shader_manager);

			let bind_group_layout = context.device().create_bind_group_layout(&_2d_uniform_bind_group_descriptor);

			let uniform_bind_group = context.device().create_bind_group(&BindGroupDescriptor{
				label: Some("Texture bind group"),
				layout: &bind_group_layout,
				entries: &[
					BindGroupEntry{
						binding: 0,
						resource: uniform.buffers.as_entire_binding(),
					},
				],
			});

			Self {
				scene: (points, triangle, circles, texture_renderer),
				uniform,
				uniform_bind_group,
			}
		}

		pub fn render_all(&mut self, context: &WGPUContext, shader_manager: &ShaderManager) {
			// log::trace!("Frame Delta: {}", self.timer.elapsed_reset());
			// self.timer.reset();

			let surface_texture = context.surface().get_current_texture()
				.expect("Could not get current texture");

			let texture_view = surface_texture.texture.create_view(&TextureViewDescriptor{
				label: Some("Render Texture"),
				format: Some(surface_texture.texture.format()),
				dimension: Some(TextureViewDimension::D2),
				usage: Some(TextureUsages::RENDER_ATTACHMENT),
				aspect: TextureAspect::All,
				base_mip_level: 0,
				mip_level_count: None,
				base_array_layer: 0,
				array_layer_count: None,
			});

			let mut encoder = context.get_encoder();
			let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor{
				label: None,
				color_attachments: &[
					Some(RenderPassColorAttachment{
						view: &texture_view,
						resolve_target: None,
						ops: Operations {
							load: LoadOp::Clear(Color{r: 0.05, g: 0.05, b: 0.05, a: 1.0}),
							store: StoreOp::Store,
						}
					})
				],
				..Default::default()
			});

			self.scene.0.render(&mut render_pass, &context, &shader_manager);
			self.scene.1.render(&mut render_pass, &context, &shader_manager);
			self.scene.2.render(&mut render_pass, &context, &shader_manager);
			self.scene.3.render(&mut render_pass, &context, &shader_manager);

			std::mem::drop(render_pass);
			context.queue().submit([encoder.finish()]);
			surface_texture.present();
		}

		pub fn update_uniform(&mut self, context: &WGPUContext) {
			self.uniform.data.screen_size = [context.config().width as f32, context.config().height as f32];
			self.uniform.update_buffer(context);
		}

		pub fn get_scene_mut(&mut self) -> &mut (PointRenderer, TriangleListRenderer, CircleRenderer, TextureRenderer) {
			&mut self.scene
		}
	}
}
