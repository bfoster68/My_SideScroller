//! GPU-driven parallax mountain background using a compute shader.
//!
//! Replaces the old sprite-based parallax with a single fullscreen texture
//! written by `assets/shaders/mountain_bg.wgsl` every frame. The shader
//! receives the game camera's X position so the hills scroll at different
//! parallax rates, plus elapsed time for the day/night cycle.

use std::borrow::Cow;

use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::{
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        render_asset::RenderAssets,
        render_graph::{self, RenderGraph, RenderLabel},
        render_resource::{
            binding_types::{texture_storage_2d, uniform_buffer},
            *,
        },
        renderer::{RenderContext, RenderDevice, RenderQueue},
        texture::GpuImage,
        Render, RenderApp, RenderStartup, RenderSystems,
    },
};

use crate::player::Player;
use crate::state::GameState;

/// Marker for the background sprite so we can move it with the camera.
#[derive(Component)]
struct MountainBgSprite;

const BG_WIDTH: u32 = 1280;
const BG_HEIGHT: u32 = 720;
const WORKGROUP_SIZE: u32 = 16;

/// Z depth for the background sprite — behind everything else.
const BG_Z: f32 = -50.0;

// ---------------------------------------------------------------------------
// Main-world resources
// ---------------------------------------------------------------------------

/// Uniform data sent to the GPU each frame. Layout must match the shader's
/// `Params` struct exactly (std140 alignment).
#[derive(Resource, Clone, ExtractResource, ShaderType, Default)]
struct MountainParams {
    camera_x: f32,
    camera_y: f32,
    time: f32,
    resolution_x: f32,
    resolution_y: f32,
    _padding: Vec3,
}

/// Handle to the storage texture the compute shader writes to.
#[derive(Resource, Clone, ExtractResource)]
struct MountainImage(Handle<Image>);

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct MountainBgPlugin;

impl Plugin for MountainBgPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MountainParams>()
            .add_plugins(ExtractResourcePlugin::<MountainParams>::default())
            .add_plugins(ExtractResourcePlugin::<MountainImage>::default())
            .add_plugins(MountainComputePlugin)
            .add_systems(Startup, setup_mountain_bg)
            .add_systems(
                Update,
                (update_mountain_params, follow_camera)
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

// ---------------------------------------------------------------------------
// Setup
// ---------------------------------------------------------------------------

fn setup_mountain_bg(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let mut image = Image::new_fill(
        Extent3d {
            width: BG_WIDTH,
            height: BG_HEIGHT,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.texture_descriptor.usage =
        TextureUsages::COPY_DST | TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING;

    let image_handle = images.add(image);

    // Fullscreen sprite behind all gameplay.
    commands.spawn((
        Sprite {
            image: image_handle.clone(),
            custom_size: Some(Vec2::new(BG_WIDTH as f32, BG_HEIGHT as f32)),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, BG_Z),
        MountainBgSprite,
    ));

    commands.insert_resource(MountainImage(image_handle));
}

// ---------------------------------------------------------------------------
// Per-frame param update
// ---------------------------------------------------------------------------

/// Feed the game camera position and elapsed time into MountainParams so the
/// shader knows where to render.
fn update_mountain_params(
    mut params: ResMut<MountainParams>,
    camera_query: Query<&Transform, (With<Camera2d>, Without<Player>)>,
    time: Res<Time>,
) {
    let Ok(cam_tf) = camera_query.single() else {
        return;
    };

    params.camera_x = cam_tf.translation.x;
    params.camera_y = cam_tf.translation.y;
    params.time = time.elapsed_secs();
    params.resolution_x = BG_WIDTH as f32;
    params.resolution_y = BG_HEIGHT as f32;
}

/// Keep the background sprite centered on the camera so it always fills the
/// viewport regardless of where the player is.
fn follow_camera(
    camera_query: Query<&Transform, (With<Camera2d>, Without<MountainBgSprite>)>,
    mut bg_query: Query<&mut Transform, (With<MountainBgSprite>, Without<Camera2d>)>,
) {
    let Ok(cam_tf) = camera_query.single() else { return };
    let Ok(mut bg_tf) = bg_query.single_mut() else { return };
    bg_tf.translation.x = cam_tf.translation.x;
    bg_tf.translation.y = cam_tf.translation.y;
}

// ---------------------------------------------------------------------------
// Render-world plugin (compute pipeline)
// ---------------------------------------------------------------------------

struct MountainComputePlugin;

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct MountainComputeLabel;

impl Plugin for MountainComputePlugin {
    fn build(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .add_systems(RenderStartup, init_mountain_pipeline)
            .add_systems(
                Render,
                prepare_bind_group.in_set(RenderSystems::PrepareBindGroups),
            );

        let mut render_graph = render_app.world_mut().resource_mut::<RenderGraph>();
        render_graph.add_node(MountainComputeLabel, MountainComputeNode::default());
        render_graph.add_node_edge(
            MountainComputeLabel,
            bevy::render::graph::CameraDriverLabel,
        );
    }
}

// --- GPU pipeline ---

#[derive(Resource)]
struct MountainPipeline {
    bind_group_layout: BindGroupLayoutDescriptor,
    pipeline_id: CachedComputePipelineId,
}

fn init_mountain_pipeline(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    pipeline_cache: Res<PipelineCache>,
) {
    let bind_group_layout = BindGroupLayoutDescriptor::new(
        "mountain_bg_bind_group_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::COMPUTE,
            (
                texture_storage_2d(TextureFormat::Rgba8Unorm, StorageTextureAccess::WriteOnly),
                uniform_buffer::<MountainParams>(false),
            ),
        ),
    );

    let shader = asset_server.load("shaders/mountain_bg.wgsl");

    let pipeline_id = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
        layout: vec![bind_group_layout.clone()],
        shader,
        entry_point: Some(Cow::from("main")),
        ..default()
    });

    commands.insert_resource(MountainPipeline {
        bind_group_layout,
        pipeline_id,
    });
}

#[derive(Resource)]
struct MountainBindGroup(BindGroup);

fn prepare_bind_group(
    mut commands: Commands,
    pipeline: Res<MountainPipeline>,
    mountain_image: Res<MountainImage>,
    render_device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    params: Res<MountainParams>,
    gpu_images: Res<RenderAssets<GpuImage>>,
    pipeline_cache: Res<PipelineCache>,
) {
    let Some(gpu_image) = gpu_images.get(&mountain_image.0) else {
        return;
    };

    let mut uniform_buffer = UniformBuffer::from(params.clone());
    uniform_buffer.write_buffer(&render_device, &queue);

    let bind_group = render_device.create_bind_group(
        None,
        &pipeline_cache.get_bind_group_layout(&pipeline.bind_group_layout),
        &BindGroupEntries::sequential((&gpu_image.texture_view, &uniform_buffer)),
    );

    commands.insert_resource(MountainBindGroup(bind_group));
}

// --- Render graph node ---

#[derive(Default)]
struct MountainComputeNode;

impl render_graph::Node for MountainComputeNode {
    fn run(
        &self,
        _graph: &mut render_graph::RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), render_graph::NodeRunError> {
        let Some(bind_group) = world.get_resource::<MountainBindGroup>() else {
            return Ok(());
        };
        let Some(pipeline_resource) = world.get_resource::<MountainPipeline>() else {
            return Ok(());
        };
        let pipeline_cache = world.resource::<PipelineCache>();

        let Some(pipeline) =
            pipeline_cache.get_compute_pipeline(pipeline_resource.pipeline_id)
        else {
            return Ok(());
        };

        let mut pass = render_context
            .command_encoder()
            .begin_compute_pass(&ComputePassDescriptor::default());

        pass.set_bind_group(0, &bind_group.0, &[]);
        pass.set_pipeline(pipeline);
        pass.dispatch_workgroups(
            (BG_WIDTH + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE,
            (BG_HEIGHT + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE,
            1,
        );

        Ok(())
    }
}
