use crate::Graphics2D;
use std::rc::Rc;

pub struct SpriteSheet {
    bind_group: wgpu::BindGroup,
}

impl SpriteSheet {
    pub fn from_bytes(state: &mut Graphics2D, diffuse_bytes: &[u8]) -> Rc<Self> {
        let device = state.device();
        let texture_bind_group_layout = state.texture_bind_group_layout();
        let queue = state.queue();

        let diffuse_image = image::load_from_memory(diffuse_bytes).unwrap();
        let diffuse_rgba = diffuse_image.as_rgba8().unwrap();

        use image::GenericImageView;
        let dimensions = diffuse_image.dimensions();
        let size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth: 1,
        };
        let buffer = device.create_buffer_with_data(&diffuse_rgba, wgpu::BufferUsage::COPY_SRC);
        let diffuse_texture = device.create_texture(&wgpu::TextureDescriptor {
            // All textures are stored as 3d, we represent our 2d texture
            // by setting depth to 1.
            size: wgpu::Extent3d {
                width: dimensions.0,
                height: dimensions.1,
                depth: 1,
            },
            // You can store multiple textures of the same size in one
            // SpriteSheet object
            array_layer_count: 1,
            mip_level_count: 1, // We'll talk about this a little later
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            // SAMPLED tells wgpu that we want to use this texture in shaders
            // COPY_DST means that we want to copy data to this texture
            usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::COPY_DST,
            label: Some("diffuse_texture"),
        });
        {
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("texture_buffer_copy_encoder"),
            });

            encoder.copy_buffer_to_texture(
                wgpu::BufferCopyView {
                    buffer: &buffer,
                    offset: 0,
                    bytes_per_row: 4 * dimensions.0,
                    rows_per_image: dimensions.1,
                },
                wgpu::TextureCopyView {
                    texture: &diffuse_texture,
                    mip_level: 0,
                    array_layer: 0,
                    origin: wgpu::Origin3d::ZERO,
                },
                size,
            );

            queue.submit(&[encoder.finish()]);
        }
        let diffuse_texture_view = diffuse_texture.create_default_view();

        let diffuse_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            lod_min_clamp: -100.0,
            lod_max_clamp: 100.0,
            compare: wgpu::CompareFunction::Always,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &texture_bind_group_layout,
            bindings: &[
                wgpu::Binding {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&diffuse_texture_view),
                },
                wgpu::Binding {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&diffuse_sampler),
                },
            ],
            label: Some("diffuse_bind_group"),
        });
        Rc::new(Self { bind_group })
    }

    pub(crate) fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}