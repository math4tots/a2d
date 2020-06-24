use crate::shaders;
use crate::Instance;
use crate::Result;
use crate::Scaling;
use crate::SpriteBatch;
use crate::SpriteSheet;
use crate::TextGrid;
use std::rc::Rc;

pub struct Graphics2D {
    surface: wgpu::Surface,
    #[allow(dead_code)]
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    sc_desc: wgpu::SwapChainDescriptor,
    swap_chain: wgpu::SwapChain,
    scale_uniform_bind_group_layout: wgpu::BindGroupLayout,
    translation_uniform_bind_group_layout: wgpu::BindGroupLayout,
    render_pipeline: wgpu::RenderPipeline,
    texture_bind_group_layout: wgpu::BindGroupLayout,

    scale: Scaling,
    scale_uniform_buffer: wgpu::Buffer,

    courier_sprite_sheet: Option<Rc<SpriteSheet>>,
}

impl Graphics2D {
    pub async fn from_winit_window(window: &winit::window::Window) -> Result<Self> {
        let size = window.inner_size();
        let surface = wgpu::Surface::create(window);
        let adapter = match wgpu::Adapter::request(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::Default,
                compatible_surface: Some(&surface),
            },
            wgpu::BackendBit::PRIMARY,
        )
        .await
        {
            Some(adapter) => adapter,
            None => err!(""),
        };

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                extensions: wgpu::Extensions {
                    anisotropic_filtering: false,
                },
                limits: Default::default(),
            })
            .await;
        let sc_desc = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
        };
        let swap_chain = device.create_swap_chain(&surface, &sc_desc);

        // compile shaders
        let vs_data = wgpu::read_spirv(std::io::Cursor::new(shaders::VERT))?;
        let fs_data = wgpu::read_spirv(std::io::Cursor::new(shaders::FRAG))?;
        let vs_module = device.create_shader_module(&vs_data);
        let fs_module = device.create_shader_module(&fs_data);

        // sheet bind layout
        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                bindings: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::SampledTexture {
                            multisampled: false,
                            dimension: wgpu::TextureViewDimension::D2,
                            component_type: wgpu::TextureComponentType::Uint,
                        },
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::Sampler { comparison: false },
                    },
                ],
                label: Some("texture_bind_group_layout"),
            });

        // scale uniform bind layout
        let scale_uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                bindings: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStage::VERTEX,
                    ty: wgpu::BindingType::UniformBuffer { dynamic: false },
                }],
                label: Some("scale_uniform_bind_group_layout"),
            });

        // translation uniform bind layout
        let translation_uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                bindings: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStage::VERTEX,
                    ty: wgpu::BindingType::UniformBuffer { dynamic: false },
                }],
                label: Some("translation_uniform_bind_group_layout"),
            });

        // build the pipeline
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                bind_group_layouts: &[
                    &texture_bind_group_layout,
                    &scale_uniform_bind_group_layout,
                    &translation_uniform_bind_group_layout,
                ],
            });
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            layout: &render_pipeline_layout,
            vertex_stage: wgpu::ProgrammableStageDescriptor {
                module: &vs_module,
                entry_point: "main",
            },
            fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
                module: &fs_module,
                entry_point: "main",
            }),
            rasterization_state: Some(wgpu::RasterizationStateDescriptor {
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: wgpu::CullMode::Back,
                depth_bias: 0,
                depth_bias_slope_scale: 0.0,
                depth_bias_clamp: 0.0,
            }),
            color_states: &[wgpu::ColorStateDescriptor {
                format: sc_desc.format,
                color_blend: wgpu::BlendDescriptor {
                    src_factor: wgpu::BlendFactor::SrcAlpha,
                    dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                    operation: wgpu::BlendOperation::Add,
                },
                alpha_blend: wgpu::BlendDescriptor::REPLACE,
                write_mask: wgpu::ColorWrite::ALL,
            }],
            primitive_topology: wgpu::PrimitiveTopology::TriangleList,
            depth_stencil_state: None,
            vertex_state: wgpu::VertexStateDescriptor {
                index_format: wgpu::IndexFormat::Uint16,
                vertex_buffers: &[Instance::desc()],
            },
            sample_count: 1,
            sample_mask: !0,
            alpha_to_coverage_enabled: false,
        });

        let scale = [1.0, 1.0];
        let scale_uniform_buffer = device
            .create_buffer_with_data(bytemuck::cast_slice(&scale), wgpu::BufferUsage::UNIFORM);

        Ok(Self {
            surface,
            adapter,
            device,
            queue,
            sc_desc,
            swap_chain,
            scale_uniform_bind_group_layout,
            translation_uniform_bind_group_layout,
            render_pipeline,
            texture_bind_group_layout,
            scale,
            scale_uniform_buffer,
            courier_sprite_sheet: None,
        })
    }

    fn courier_sprite_sheet(&mut self) -> Result<Rc<SpriteSheet>> {
        if self.courier_sprite_sheet.is_none() {
            self.courier_sprite_sheet = Some(TextGrid::courier_sprite_sheet(self)?);
        }
        Ok(self.courier_sprite_sheet.as_ref().unwrap().clone())
    }

    /// Creates a new TextGrid instance with the builtin courier font
    /// given the width of a character block and [num_rows, num_cols]
    pub fn new_text_grid(&mut self, char_width: f32, dim: [u32; 2]) -> Result<TextGrid> {
        let sheet = self.courier_sprite_sheet()?;
        Ok(TextGrid::new(sheet, char_width, dim))
    }

    /// Call this method to notify A2D that the window has been resized
    pub fn resized(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.sc_desc.width = new_size.width;
        self.sc_desc.height = new_size.height;
        self.swap_chain = self.device.create_swap_chain(&self.surface, &self.sc_desc);
    }

    /// By default, the screen coordinates are [0, 0] for the
    /// upper-left corner and [1, 1] for the lower-right corner.
    /// The coordinates of the lower-right corner may be customized
    /// with `set_scale`. The `scale` method returns the currently
    /// set [max_x, max_y] values for the lower-right corner.
    pub fn scale(&self) -> [f32; 2] {
        self.scale
    }

    /// Sets the the scale to set the coordinates of the
    /// lower-right corner (the upper-left is always [0, 0]).
    /// See the method `scale` for more info.
    pub fn set_scale(&mut self, new_scale: [f32; 2]) {
        self.scale = new_scale;
        self.scale_uniform_buffer = self.device.create_buffer_with_data(
            bytemuck::cast_slice(&self.scale),
            wgpu::BufferUsage::UNIFORM,
        );
    }

    pub fn render(&mut self, batches: &[&SpriteBatch]) {
        struct BatchInfo<'a> {
            batch: &'a SpriteBatch,
            instance_buffer: wgpu::Buffer,
            translation_bind_group: wgpu::BindGroup,
        }
        let batches_with_instance_buffers = {
            let mut vec = Vec::new();
            for batch in batches {
                // wgpu will error if you try to create a buffer of size 0,
                // so explicitly check for those cases and skip
                if batch.instances().is_empty() {
                    continue;
                }
                let instance_buffer = self.device.create_buffer_with_data(
                    bytemuck::cast_slice(batch.instances()),
                    wgpu::BufferUsage::VERTEX,
                );
                let translation_buffer = self.device.create_buffer_with_data(
                    bytemuck::cast_slice(&batch.translation()),
                    wgpu::BufferUsage::UNIFORM,
                );
                let translation_bind_group =
                    self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                        layout: &self.translation_uniform_bind_group_layout,
                        bindings: &[wgpu::Binding {
                            binding: 0,
                            resource: wgpu::BindingResource::Buffer {
                                buffer: &translation_buffer,
                                range: 0..std::mem::size_of::<Scaling>() as wgpu::BufferAddress,
                            },
                        }],
                        label: Some("per_batch_scale_uniform_bind_group"),
                    });
                vec.push(BatchInfo {
                    batch,
                    instance_buffer,
                    translation_bind_group,
                });
            }
            vec
        };
        let scale_uniform_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.scale_uniform_bind_group_layout,
            bindings: &[wgpu::Binding {
                binding: 0,
                resource: wgpu::BindingResource::Buffer {
                    buffer: &self.scale_uniform_buffer,
                    range: 0..std::mem::size_of::<Scaling>() as wgpu::BufferAddress,
                },
            }],
            label: Some("default_scale_uniform_bind_group"),
        });
        let frame = self
            .swap_chain
            .get_next_texture()
            .expect("Timeout getting next texture");
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                    attachment: &frame.view,
                    resolve_target: None,
                    load_op: wgpu::LoadOp::Clear,
                    store_op: wgpu::StoreOp::Store,
                    clear_color: wgpu::Color {
                        r: 0.0,
                        g: 0.0,
                        b: 0.0,
                        a: 0.0,
                    },
                }],
                depth_stencil_attachment: None,
            });
            render_pass.set_pipeline(&self.render_pipeline);
            for info in &batches_with_instance_buffers {
                let batch = info.batch;
                let instance_buffer = &info.instance_buffer;
                let translation_bind_group = &info.translation_bind_group;
                render_pass.set_bind_group(0, batch.sheet().bind_group(), &[]);
                render_pass.set_bind_group(1, &scale_uniform_bind_group, &[]);
                render_pass.set_bind_group(2, translation_bind_group, &[]);
                render_pass.set_vertex_buffer(0, instance_buffer, 0, 0);
                render_pass.draw(0..6, 0..batch.instances().len() as u32);
            }
        }

        self.queue.submit(&[encoder.finish()]);
    }

    pub(crate) fn device(&self) -> &wgpu::Device {
        &self.device
    }

    pub(crate) fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    pub(crate) fn texture_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.texture_bind_group_layout
    }
}
