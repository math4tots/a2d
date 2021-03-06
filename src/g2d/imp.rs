use super::*;

/// Call wgpu's device.poll(..) roughly 60 times per second
const POLL_SLEEP_DUR: Duration = Duration::from_micros((1000000.0 / 60.0) as u64);

/// Helper methods on Graphics2D (all listed here should be private to a2d)
impl Graphics2D {
    pub(super) async fn new0<W: HasRawWindowHandle>(
        physical_width: u32,
        physical_height: u32,
        window: &W,
    ) -> Result<Self> {
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
            width: physical_width,
            height: physical_height,
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
            device: Arc::new(device),
            queue,
            sc_desc,
            swap_chain,
            scale_uniform_bind_group_layout,
            translation_uniform_bind_group_layout,
            render_pipeline,
            texture_bind_group_layout,
            scale,
            scale_uniform_buffer,
            batches: Default::default(),
            text_grid_dim: None,
            dirty: true,
            poll_thread: None,
        })
    }

    pub(super) fn pixel_batch(&mut self) -> Result<&mut Batch> {
        if self.batches[BATCH_SLOT_PIXEL].is_none() {
            let [width, height] = self.scale();
            let width = width as usize;
            let height = height as usize;
            let mut descs = vec![];
            for y in 0..height {
                let y = y as f32;
                for x in 0..width {
                    let x = x as f32;
                    descs.push(SpriteDesc {
                        src: 0,
                        dst: [x, y, x + 1.0, y + 1.0].into(),
                        rotate: 0.0,
                        color: [0.0, 0.0, 0.0, 0.0].into(),
                    });
                }
            }
            let sheet = Sheet::from_color(self, [1.0, 1.0, 1.0])?;
            let batch = Batch::new(self, sheet, 1, 1, &descs);
            self.batches[BATCH_SLOT_PIXEL] = Some(batch);
        }
        Ok(self.batches[BATCH_SLOT_PIXEL].as_mut().unwrap())
    }

    pub(super) fn text_batch(&mut self) -> Result<&mut Batch> {
        Ok(self.batches[BATCH_SLOT_TEXT].as_mut().unwrap())
    }

    pub(super) fn ensure_polling(&mut self) -> Result<()> {
        if self.poll_thread.is_none() {
            let device = self.device.clone();
            let (sender, receiver) = std::sync::mpsc::channel();
            let thread = std::thread::Builder::new()
                .name("a2d-wgpu-poll".to_owned())
                .spawn(move || loop {
                    match receiver.try_recv() {
                        Ok(()) | Err(std::sync::mpsc::TryRecvError::Disconnected) => break,
                        Err(std::sync::mpsc::TryRecvError::Empty) => {}
                    }
                    device.poll(wgpu::Maintain::Wait);
                    std::thread::sleep(POLL_SLEEP_DUR);
                    std::thread::yield_now();
                })?;
            self.poll_thread = Some((thread, sender));
        }
        Ok(())
    }

    pub(super) async fn async_flush(&mut self) -> Result<()> {
        let futs: Vec<_> = self
            .batches
            .iter_mut()
            .flatten()
            .map(Batch::flush)
            .collect();
        let futs = futures::future::try_join_all(futs);
        self.device.poll(wgpu::Maintain::Wait);
        futs.await?;
        Ok(())
    }
}
