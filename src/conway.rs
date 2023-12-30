// A plugin that implements Conway's Game of Life using a compute shader.

use std::borrow::Cow;

use bevy::{
    prelude::*,
    render::{
        render_resource::*,
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        renderer::{RenderDevice, RenderContext, RenderQueue},
        render_asset::RenderAssets,
        RenderApp, Render, RenderSet, 
        render_graph::{RenderGraph, Node as RenderNode, self},
        MainWorld,
        texture::ImageSampler,
    }, window::PrimaryWindow
    };

pub struct ConwayPlugin;

// How much to parallelize the compute shader.
const SCALE_FACTOR: u32 = 10;
const SIZE: (u32, u32) = (128 * SCALE_FACTOR, 72 * SCALE_FACTOR);
const WORKGROUP_SIZE: (u32, u32) = (8, 8);

/// The number of living cells -- this is computed by the compute shader
/// and shared to the MainWorld.
#[derive(Resource, Default)]
struct LivingCells(u64);


/// The texture that stores the Conway's game state.
#[derive(Resource, Clone, Deref, ExtractResource)]
struct ConwayWorld(Handle<Image>);


/// Cells to set in the compute shader.
#[derive(Resource, Clone, ExtractResource)]
struct SetCells(Vec<Vec2>);


impl Plugin for ConwayPlugin {
    fn build(&self, app: &mut App) {
        app
        .init_resource::<LivingCells>()
        .insert_resource(SetCells(vec![]))
        .add_plugins(ExtractResourcePlugin::<ConwayWorld>::default())
        .add_plugins(ExtractResourcePlugin::<SetCells>::default())
        .add_systems(First, clear_set_cells)
        .add_systems(Startup, setup)
        .add_systems(FixedUpdate, render_living_cells)
        .add_systems(Update, handle_mouse_click)
        ;

        // Add the compute shader to the render app.
        // The compute shader happens in the render pass, so we need to add it to the render graph.  
        let render_app = app.sub_app_mut(RenderApp);
        render_app.insert_resource(LivingCells(10));
        render_app.insert_resource(SetCells(vec![]));
        render_app.add_systems(Render, (
            view_mouse_click.in_set(RenderSet::PrepareBindGroups),
            prepare_bind_group.in_set(RenderSet::PrepareBindGroups),
            prepare_resources.in_set(RenderSet::PrepareResources),
            update_living_cells.in_set(RenderSet::Cleanup),
        ));
        // TODO(arun): this should move to after the rendering stage.
        render_app.add_systems(ExtractSchedule, copy_living_cells);
        let mut render_graph = render_app.world.resource_mut::<RenderGraph>();
        render_graph.add_node(
            "conway_state",
            ConwayRenderNode::default(),
        );
        render_graph.add_node_edge(
            "conway_state",
            bevy::render::main_graph::node::CAMERA_DRIVER
        );
    }

    fn finish(&self, app: &mut App) {
        app.sub_app_mut(RenderApp).init_resource::<ConwayPipeline>();
    }
}

fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    // Create a new image that will be used as a texture.
    let mut image = Image::new_fill(
                Extent3d {
                    width: SIZE.0,
                    height: SIZE.1,
                    depth_or_array_layers: 1,
                },
                TextureDimension::D2,
                &[0, 0, 0, 255],
                TextureFormat::Rgba8Unorm,
    );
    image.texture_descriptor.usage =
        TextureUsages::COPY_SRC | TextureUsages::RENDER_ATTACHMENT |
        TextureUsages::COPY_DST | TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING;
    image.sampler = ImageSampler::nearest();
    let image_handle = images.add(image);

    commands.spawn(
        SpriteBundle {
            texture: image_handle.clone(),
            transform: Transform {
                scale: Vec3::new(10.0 / (SCALE_FACTOR as f32), 10.0 / (SCALE_FACTOR as f32), 1.0),
                ..default()
            },
            ..default()
        }
    );
    commands.insert_resource(ConwayWorld(image_handle));

    commands.spawn(
        TextBundle::from_section(
            "Living cells: 0",
            TextStyle {
                font_size: 40.0,
                color: Color::WHITE,
                ..Default::default()
            }
        )
    );
}

fn render_living_cells(
    mut query: Query<&mut Text>,
    living_cells: Res<LivingCells>,
) {
    let mut text = query.single_mut();
    text.sections[0].value = format!("Living cells: {}", living_cells.0);
}

fn clear_set_cells(mut set_cells: ResMut<SetCells>) {
    set_cells.0.clear();
}

fn handle_mouse_click(
    mut set_cells: ResMut<SetCells>,
    mouse_button_input: Res<Input<MouseButton>>,
    window: Query<&Window, With<PrimaryWindow>>,
) {
    if mouse_button_input.just_pressed(MouseButton::Left) {
        let window = window.single();
        if let Some(cursor_pos) = window.cursor_position() {
            println!("user clicked {:?}", cursor_pos);
            // Transform to clip space
            let x = (cursor_pos.x / window.width()) * 2.0 - 1.0;
            let y = (cursor_pos.y / window.height()) * 2.0 - 1.0;
            set_cells.0.push(Vec2::new(x, -y));
        }
    }
}

// Render World stuff.
fn view_mouse_click(
    set_cells: Res<SetCells>,
) {
    if !set_cells.0.is_empty() {
        println!("set cells: {:?}", set_cells.0);
    }
}


// The compute pipeline.
#[derive(Resource)]
struct ConwayPipeline {
    // The bind group layout for resources used in the pipelines.
    texture_bind_group_layout: BindGroupLayout,
    // Pipeline for initializing the texture.
    init_pipeline: CachedComputePipelineId,
    // Pipeline for updating Conway State each step.
    update_pipeline: CachedComputePipelineId,
    // Pipeline for setting cells.
    set_cells_pipeline: CachedRenderPipelineId,
}

impl FromWorld for ConwayPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let shader = world.resource::<AssetServer>().load("shaders/conway.wgsl");
        let pipeline_cache = world.resource::<PipelineCache>();

        let texture_bind_group_layout = render_device.create_bind_group_layout(
            &BindGroupLayoutDescriptor {
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::StorageTexture {
                            access: StorageTextureAccess::ReadWrite,
                            format: TextureFormat::Rgba8Unorm,
                            view_dimension: TextureViewDimension::D2,
                        },
                        count: None,
                    },
                ],
                label: Some("conway_state_bind_group_layout"),
            },
        );

        let vertex_buffer_layout = VertexBufferLayout {
            array_stride: std::mem::size_of::<Vec2>() as u64,
            step_mode: VertexStepMode::Vertex,
            attributes: vec![
                VertexAttribute {
                    format: VertexFormat::Float32x2,
                    offset: 0,
                    shader_location: 0,
                },
            ]
        };

        let init_pipeline = pipeline_cache.queue_compute_pipeline(
            ComputePipelineDescriptor {
                label: Some(Cow::from("conway_init_pipeline")),
                layout: vec![texture_bind_group_layout.clone()],
                push_constant_ranges: vec![],
                shader: shader.clone(),
                shader_defs: vec![],
                entry_point: Cow::from("init"),
            }
        );
        let update_pipeline = pipeline_cache.queue_compute_pipeline(
            ComputePipelineDescriptor {
                label: Some(Cow::from("conway_update_pipeline")),
                layout: vec![texture_bind_group_layout.clone()],
                push_constant_ranges: vec![],
                shader: shader.clone(),
                shader_defs: vec![],
                entry_point: Cow::from("update"),
            },
        );
        let set_cells_pipeline = pipeline_cache.queue_render_pipeline(
            RenderPipelineDescriptor {
                label: Some(Cow::from("conway_set_cell_pipeline")),
                layout: vec![],
                primitive: PrimitiveState {
                    topology: PrimitiveTopology::PointList,
                    ..default()
                },
                vertex: VertexState {
                    entry_point: Cow::from("set_cells_vs"),
                    shader: shader.clone(),
                    shader_defs: vec![],
                    buffers: vec![vertex_buffer_layout],
                },
                fragment: Some(FragmentState {
                    entry_point: Cow::from("set_cells_fs"),
                    shader: shader.clone(),
                    shader_defs: vec![],
                    targets: vec![Some(ColorTargetState {
                        format: TextureFormat::Rgba8Unorm,
                        blend: Some(BlendState::ALPHA_BLENDING),
                        write_mask: ColorWrites::ALL,
                    })],
                }),
                depth_stencil: None,
                multisample: MultisampleState::default(),
                push_constant_ranges: vec![],
            },
        );
        Self {
            texture_bind_group_layout,
            init_pipeline,
            update_pipeline,
            set_cells_pipeline,
        }
    }
}

// Instantiate a bind group for the conway pipeline.
#[derive(Resource)]
struct ConwayStateBindGroup(BindGroup);
// Instantiate a bind group for the conway pipeline.
fn prepare_bind_group(
    mut commands: Commands,
    pipeline: Res<ConwayPipeline>,
    gpu_images: Res<RenderAssets<Image>>,
    conway_state: Res<ConwayWorld>,
    render_device: Res<RenderDevice>,
) {
    // Get the image for conway state from the GPU asset server.
    let image = gpu_images.get(&conway_state.0).unwrap();
    let bind_group = render_device.create_bind_group(
        Some("conway_state_bind_group"),
        &pipeline.texture_bind_group_layout,
        &[
            BindGroupEntry {
                binding: 0,
                resource: BindingResource::TextureView(&image.texture_view),
            },
        ]
    );
    commands.insert_resource(ConwayStateBindGroup(bind_group));
}

#[derive(Resource, Clone)]
struct OutputBuffer {
    buffer: Buffer,
}

fn prepare_resources(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
) {
    let buffer = render_device.create_buffer(&BufferDescriptor {
        label: Some("conway_output_buffer"),
        size: (SIZE.0 * SIZE.1 * 4) as u64,
        usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    commands.insert_resource(OutputBuffer { buffer });
}

// The RenderGraph for Conway's game.
#[derive(Default)]
enum ConwayState {
    #[default]
    Loading,
    Init,
    Update,
}

#[derive(Default)]
struct ConwayRenderNode(ConwayState);

impl RenderNode for ConwayRenderNode {
    fn update(&mut self, world: &mut World) {
        let pipeline = world.resource::<ConwayPipeline>();
        let pipeline_cache = world.resource::<PipelineCache>();

        match self {
            ConwayRenderNode(ConwayState::Loading) => {
                if let CachedPipelineState::Ok(_) = pipeline_cache.get_compute_pipeline_state(pipeline.init_pipeline) {
                    *self = ConwayRenderNode(ConwayState::Init);
                }
            }
            ConwayRenderNode(ConwayState::Init) => {
                if let CachedPipelineState::Ok(_) = pipeline_cache.get_compute_pipeline_state(pipeline.update_pipeline) {
                    *self = ConwayRenderNode(ConwayState::Update);
                }
            }
            ConwayRenderNode(ConwayState::Update) => {}
        }
    }

    fn run(
        &self,
        _graph: &mut render_graph::RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), render_graph::NodeRunError> {
        let pipeline = world.resource::<ConwayPipeline>();
        let pipeline_cache = world.resource::<PipelineCache>();
        let render_device = world.resource::<RenderDevice>();
        let texture_bind_group = &world.resource::<ConwayStateBindGroup>().0;
        let gpu_images = world.resource::<RenderAssets<Image>>();
        let conway_state = world.resource::<ConwayWorld>();
        let set_cells = world.resource::<SetCells>();

        let encoder = render_context.command_encoder();

        if !set_cells.0.is_empty() {
            let gpu_image = gpu_images.get(&conway_state.0).unwrap();
            let set_cell_data = bytemuck::cast_slice(set_cells.0.as_slice());
            let vertex_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
                usage: BufferUsages::VERTEX,
                label: Some("Mesh Vertex Buffer"),
                contents: set_cell_data,
            });

            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("set_cells_render"),
                color_attachments: &vec![Some(RenderPassColorAttachment {
                    view: &gpu_image.texture_view,
                    resolve_target: None,
                    ops: Operations { load: LoadOp::Load, store: true }
                })],
                depth_stencil_attachment: None,
            });
            // pass.set_bind_group(0, set_cells_bind_group, &[]);
            pass.set_pipeline(&pipeline_cache.get_render_pipeline(pipeline.set_cells_pipeline).unwrap());
            // Load the buffer with the cells to set.
            pass.set_vertex_buffer(0, *vertex_buffer.slice(..));
            pass.draw(0..set_cells.0.len() as u32, 0..1);
        }

        match self {
            ConwayRenderNode(ConwayState::Loading) => {
                return Ok(())
            }
            ConwayRenderNode(ConwayState::Init) => {
                let mut pass = encoder.begin_compute_pass(
                    &ComputePassDescriptor::default());
                pass.set_bind_group(0, texture_bind_group, &[]);
                pass.set_pipeline(&pipeline_cache.get_compute_pipeline(pipeline.init_pipeline).unwrap());
                pass.dispatch_workgroups(SIZE.0 / WORKGROUP_SIZE.0, SIZE.1 / WORKGROUP_SIZE.1, 1);
            } ConwayRenderNode(ConwayState::Update) => {
                let mut pass = encoder.begin_compute_pass(
                    &ComputePassDescriptor::default());
                pass.set_bind_group(0, texture_bind_group, &[]);
                pass.set_pipeline(&pipeline_cache.get_compute_pipeline(pipeline.update_pipeline).unwrap());
                pass.dispatch_workgroups(SIZE.0 / WORKGROUP_SIZE.0, SIZE.1 / WORKGROUP_SIZE.1, 1);
            }
        }
        Ok(())
    }
}


fn update_living_cells(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    output_buffer: Res<OutputBuffer>,
    conway_world: Res<ConwayWorld>,
    images: Res<RenderAssets<Image>>,
    mut living_cells: ResMut<LivingCells>
) {
    let gpu_image = images.get(&conway_world.0).unwrap();
    let mut encoder = render_device.create_command_encoder(&CommandEncoderDescriptor { label: Some("conway_encoder") });

    encoder.copy_texture_to_buffer(
        gpu_image.texture.as_image_copy(),
        ImageCopyBuffer {
            buffer: &output_buffer.buffer,
            layout: ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * SIZE.0),
                rows_per_image: Some(SIZE.1),
            },
        },
        Extent3d {
            width: gpu_image.size.x as u32,
            height: gpu_image.size.y as u32,
            depth_or_array_layers: 1,
        },
    );
    render_queue.submit(Some(encoder.finish()));

    let buffer_slice = output_buffer.buffer.slice(..);

    let (tx, rx) = async_channel::bounded(1);
    render_device.map_buffer(&buffer_slice, MapMode::Read, move |result| {
        let err = result.err();
        if err.is_some() {
            panic!("{}", err.unwrap().to_string());
        }
        tx.try_send(()).unwrap();
    });
    render_device.wgpu_device().poll(wgpu::Maintain::Wait);
    rx.try_recv().unwrap();
    let data = output_buffer.buffer.slice(..).get_mapped_range();
    let result = Vec::from(&*data).chunks(4).map(|x| x[0]).collect::<Vec<u8>>();
    let n_alive = result.iter().fold(0 as u64, |acc, x| acc + (*x == 255) as u64);
    living_cells.0 = n_alive;
}

fn copy_living_cells(
    render_living_cells: Res<LivingCells>,
    mut main_world: ResMut<MainWorld>,
) {
    main_world.resource_mut::<LivingCells>().0 = render_living_cells.0;
}
