// This example demonstrates how to load a glTF model with morph targets using Bevy and OpenXR.
// It uses the Bevy engine for rendering and OpenXR for VR support.
// The model is loaded from a file and displayed in a VR environment.
// The example includes a simple setup for a Bevy app with OpenXR integration.

use bevy::{
    prelude::*, render::pipelined_rendering::PipelinedRenderingPlugin, scene::SceneInstanceReady, 
};
use bevy_mod_openxr::{add_xr_plugins, init::OxrInitPlugin};

use openxr::EnvironmentBlendMode;
use bevy::prelude::MorphWeights;

const GLTF_PATH: &str = "simpleHumanRig.glb";

// the component that will be used to play the animation
#[derive(Component)]
struct AnimationToPlay {
    graph_handle: Handle<AnimationGraph>,
    index: AnimationNodeIndex,
}

// A common main function to start the Bevy app
fn main() {
    App::new()
        .add_plugins(
            add_xr_plugins(DefaultPlugins.build().disable::<PipelinedRenderingPlugin>()).set(
                OxrInitPlugin {
                    blend_modes: Some(vec![
                        EnvironmentBlendMode::ALPHA_BLEND,
                        EnvironmentBlendMode::ADDITIVE,
                        EnvironmentBlendMode::OPAQUE,
                    ]),
                    ..Default::default()
                },
            ),
        )
        .add_systems(Startup, setup_mesh_and_animation)
        .add_systems(Startup, setup)
        .add_systems(Update, update_morph_targets)
        .insert_resource(ClearColor(Color::NONE))
        .run();
}

fn setup_mesh_and_animation(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
) {
    // Create an animation graph containing a single animation. We want the "run"
    // animation from our example asset, which has an index of two.
    let (graph, index) = AnimationGraph::from_clip(
        asset_server.load(GltfAssetLabel::Animation(0).from_asset(GLTF_PATH)),
    );

    // Store the animation graph as an asset.
    let graph_handle = graphs.add(graph);

    // Create a component that stores a reference to our animation.
    let animation_to_play = AnimationToPlay {
        graph_handle,
        index,
    };

    // Start loading the asset as a scene and store a reference to it in a
    // SceneRoot component. This component will automatically spawn a scene
    // containing our mesh once it has loaded.
    let mesh_scene = SceneRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset(GLTF_PATH)));

    // Spawn an entity with our components, and connect it to an observer that
    // will trigger when the scene is loaded and spawned.
    commands
        .spawn((animation_to_play, mesh_scene))
        .observe(play_animation_when_ready);
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // circular base
    commands.spawn((
        Mesh3d(meshes.add(Circle::new(4.0))),
        MeshMaterial3d(materials.add(Color::srgb_u8(255, 0, 0))),
        Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
    ));
    commands.spawn((
        PointLight {
            shadows_enabled: false,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
    ));
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-2.5, 2.5, 9.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

// is called when the app is running
// this is making the left arm move up and down
fn update_morph_targets(
    time: Res<Time>,
    mut query: Query<&mut MorphWeights>,
) {
    for mut weights in &mut query {
        let t = time.elapsed_secs();
        let value = t.sin() * 0.5 + 0.5;
        // Set the first morph target weight to a value between 0 and 1
        weights.weights_mut()[0] = value;
        
    }
}

// is called when the scene is loaded
// this is where we play the animation (head nodding)
fn play_animation_when_ready(
    trigger: Trigger<SceneInstanceReady>,
    mut commands: Commands,
    children: Query<&Children>,
    animations_to_play: Query<&AnimationToPlay>,
    mut players: Query<&mut AnimationPlayer>,
) {
    // The entity we spawned in `setup_mesh_and_animation` is the trigger's target.
    // Start by finding the AnimationToPlay component we added to that entity.
    if let Ok(animation_to_play) = animations_to_play.get(trigger.entity()) {
        // The SceneRoot component will have spawned the scene as a hierarchy
        // of entities parented to our entity. Since the asset contained a skinned
        // mesh and animations, it will also have spawned an animation player
        // component. Search our entity's descendants to find the animation player.
        for child in children.iter_descendants(trigger.entity()) {
            if let Ok(mut player) = players.get_mut(child) {
                // Tell the animation player to start the animation and keep
                // repeating it.
                //
                // If you want to try stopping and switching animations, see the
                // `animated_mesh_control.rs` example.
                player.play(animation_to_play.index).repeat();

                // Add the animation graph. This only needs to be done once to
                // connect the animation player to the mesh.
                commands
                    .entity(child)
                    .insert(AnimationGraphHandle(animation_to_play.graph_handle.clone()));
            }
        }
    }
}
