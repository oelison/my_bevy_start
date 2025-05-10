// This example demonstrates how to load a glTF model with morph targets using Bevy and OpenXR.
// It uses the Bevy engine for rendering and OpenXR for VR support.
// The model is loaded from a file and displayed in a VR environment.
// The example includes a simple setup for a Bevy app with OpenXR integration.

use std::f32::consts::FRAC_PI_4;

use bevy::{
    prelude::*, 
    render::pipelined_rendering::PipelinedRenderingPlugin, 
    scene::SceneInstanceReady,
    color::palettes::css,
};
use bevy_mod_openxr::{
    add_xr_plugins,
    exts::OxrExtensions,
    init::OxrInitPlugin,
    resources::OxrSessionConfig,
};

use bevy_mod_xr::session::XrTrackingRoot;
use openxr::EnvironmentBlendMode;
use bevy::prelude::MorphWeights;
use schminput::prelude::*;

#[derive(Component, Clone, Copy)]
struct HandLeft;
#[derive(Component, Clone, Copy)]
struct HandRight;

#[allow(dead_code)]
#[derive(Resource, Clone, Copy)]
struct CoreActions {
    set: Entity,
    left_pose: Entity,
    right_pose: Entity,
}

#[allow(dead_code)]
#[derive(Resource, Clone, Copy)]
struct MoveActions {
    set: Entity,
    move_action: Entity,
    turn_action: Entity,
    look: Entity,
    jump: Entity,
}

// Zustand für Turn-steuerung
#[derive(Resource, Default)]
struct TurnState {
    ready: bool,
}

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
            add_xr_plugins(DefaultPlugins).build().set(
                OxrInitPlugin {
            exts: {
                let mut exts = OxrExtensions::default();
                exts.ext_hp_mixed_reality_controller = true;
                exts
            },
            ..OxrInitPlugin::default()
        }))
        .insert_resource(OxrSessionConfig {
            blend_modes: Some({vec![
                    EnvironmentBlendMode::ALPHA_BLEND,
                    EnvironmentBlendMode::OPAQUE,
                ]}),
            ..OxrSessionConfig::default()
        })
        .add_plugins(schminput::DefaultSchminputPlugins)
        .add_systems(Startup, setup_mesh_and_animation)
        .add_systems(Startup, setup)
        .add_systems(Startup, setup2)
        .add_systems(Update, update_morph_targets)
        .add_systems(Update, run)
        .add_systems(Update, snap_turn_system)
        .insert_resource(ClearColor(Color::NONE))
        .insert_resource(TurnState::default()) 
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

fn setup2(mut cmds: Commands) {
    let player_set = cmds.spawn(ActionSet::new("player", "Player", 1)).id();
    let pose_set = cmds.spawn(ActionSet::new("pose", "Poses", 0)).id();
    let move_action = cmds
        .spawn((
            Action::new("move", "Move", player_set),
            OxrBindings::new().bindngs("/interaction_profiles/hp/mixed_reality_controller", ["/user/hand/left/input/thumbstick"]),
            Vec2ActionValue::new(),
        ))
        .id();
    let turn_action = cmds
        .spawn((
            Action::new("turn", "Turn", player_set),
            OxrBindings::new().bindngs("/interaction_profiles/hp/mixed_reality_controller", ["/user/hand/right/input/thumbstick"]),
            Vec2ActionValue::new(),
        ))
        .id();
    let look = cmds
        .spawn((
            Action::new("look", "Look", player_set),
            OxrBindings::new().bindngs(
                "/interaction_profiles/hp/mixed_reality_controller",
                ["/user/hand/right/input/thumbstick/x"],
            ),
            F32ActionValue::new(),
        ))
        .id();
    let jump = cmds
        .spawn((
            Action::new("jump", "Jump", player_set),
            OxrBindings::new().bindngs("/interaction_profiles/hp/mixed_reality_controller", ["/user/hand/right/input/a/click"]),
            KeyboardBindings::new().bind(KeyboardBinding::new(KeyCode::Space)),
            GamepadBindings::new()
                .bind(GamepadBinding::new(GamepadBindingSource::South).button_just_pressed()),
            BoolActionValue::new(),
        ))
        .id();
    let left_hand = cmds.spawn(HandLeft).id();

    let right_hand = cmds.spawn(HandRight).id();
    let left_pose = cmds
        .spawn((
            Action::new("hand_left_pose", "Left Hand Pose", pose_set),
            OxrBindings::new().bindngs("/interaction_profiles/hp/mixed_reality_controller", ["/user/hand/left/input/grip/pose"]),
            AttachSpaceToEntity(left_hand),
            SpaceActionValue::new(),
        ))
        .id();
    let right_pose = cmds
        .spawn((
            Action::new("hand_right_pose", "Right Hand Pose", pose_set),
            OxrBindings::new().bindngs("/interaction_profiles/hp/mixed_reality_controller", ["/user/hand/right/input/aim/pose"]),
            AttachSpaceToEntity(right_hand),
            SpaceActionValue::new(),
        ))
        .id();
    cmds.insert_resource(MoveActions {
        set: player_set,
        move_action,
        turn_action,
        look,
        jump,
    });
    cmds.insert_resource(CoreActions {
        set: pose_set,
        left_pose,
        right_pose,
    });
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
    if let Ok(animation_to_play) = animations_to_play.get(trigger.target()) {
        // The SceneRoot component will have spawned the scene as a hierarchy
        // of entities parented to our entity. Since the asset contained a skinned
        // mesh and animations, it will also have spawned an animation player
        // component. Search our entity's descendants to find the animation player.
        for child in children.iter_descendants(trigger.target()) {
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

fn run(
    move_actions: Res<MoveActions>,
    vec2_value: Query<&Vec2ActionValue>,
    //f32_value: Query<&F32ActionValue>,
    //bool_value: Query<&BoolActionValue>,
    left_hand: Query<&GlobalTransform, With<HandLeft>>,
    right_hand: Query<&GlobalTransform, With<HandRight>>,
    mut gizmos: Gizmos,
    mut root_query: Query<&mut Transform, With<XrTrackingRoot>>,
) {
    info!(
        "move: {}",
        vec2_value.get(move_actions.move_action).unwrap().any
    );
    //info!("look: {}", f32_value.get(move_actions.look).unwrap().any);
    //info!("jump: {}", bool_value.get(move_actions.jump).unwrap().any);
    for hand in left_hand.into_iter() {
        let pose = hand.to_isometry();
        gizmos.arrow(Vec3 { x: 0.0, y: 0.0, z: 0.0 }, pose.rotation.mul_vec3(-Vec3::Z).normalize(), css::BLUE);
        gizmos.sphere(pose, 0.1, css::BLUE);
    }
    for hand in right_hand.into_iter() {
        let pose = hand.to_isometry();
        gizmos.arrow(Vec3 { x: 0.0, y: 0.0, z: 0.0 }, pose.rotation.mul_vec3(-Vec3::Z).normalize(), css::RED);
        gizmos.sphere(pose, 0.1, css::RED);
    }
    let movevals = vec2_value.get(move_actions.move_action).unwrap().any;
    if movevals.length_squared() < 0.05 {
        return;
    }
    if let Ok(mut root_transform) = root_query.single_mut() {
        if let Some(hand) = right_hand.iter().next() {
            let pose = hand.to_isometry();
            
            let forward = pose.rotation.mul_vec3(-Vec3::Z).normalize();
            let right = pose.rotation.mul_vec3(Vec3::X).normalize();
            info!("forward: {:?}", forward);
            info!("right: {:?}", right);
            let delta = forward * movevals.y * 0.05 + right * movevals.x * 0.05;
            root_transform.translation += delta;
        }
    }
}

fn snap_turn_system(
    turn_actions: Res<MoveActions>,
    //time: Res<Time>,
    mut root_query: Query<&mut Transform, With<XrTrackingRoot>>,
    vec2_value: Query<&Vec2ActionValue>,
    mut turn_state: ResMut<TurnState>,
) {
    let movevals = vec2_value.get(turn_actions.turn_action).unwrap().any;
    
    let turn_value = movevals.x;

    // Snap-Turn nur auslösen, wenn Stick deutlich nach links oder rechts zeigt
    if turn_value.abs() > 0.8 && turn_state.ready {
        if let Ok(mut transform) = root_query.single_mut() {
            let angle = if turn_value > 0.0 { -FRAC_PI_4 } else { FRAC_PI_4 }; // Rechts = negative Rotation
            transform.rotate(Quat::from_rotation_y(angle));
            turn_state.ready = false;
        }
    }

    // Stick muss erst losgelassen werden, bevor nächste Drehung möglich ist
    if turn_value.abs() < 0.2 {
        turn_state.ready = true;
    }
}