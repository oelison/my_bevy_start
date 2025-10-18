// This example demonstrates how to load a glTF model with morph targets using Bevy and OpenXR.
// It uses the Bevy engine for rendering and OpenXR for VR support.
// The model is loaded from a file and displayed in a VR environment.
// The example includes a simple setup for a Bevy app with OpenXR integration.

mod asset_handler;
use asset_handler::{AssetElement, AssetElementList, ASSET_ELEMENTS, SIMPLE_HUMAN_RIG_INDEX, SIMPLE_WALL_INDEX};

use std::f32::consts::FRAC_PI_4;

use bevy_mod_openxr::session::OxrSession;

use bevy::{
    color::palettes::css, prelude::*, render::view::NoIndirectDrawing, scene::SceneInstanceReady
};
use bevy_mod_openxr::{
    add_xr_plugins,
    exts::OxrExtensions,
    init::OxrInitPlugin,
    resources::OxrSessionConfig,
};

use bevy_mod_xr::session::{XrSessionCreated, XrTrackingRoot};
use bevy_xr_utils::transform_utils::{self};
use openxr::EnvironmentBlendMode;
use bevy::prelude::MorphWeights;
use schminput::prelude::*;
use bevy::input::mouse::MouseMotion;

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
    center_camera: Entity,
    move_left: Entity,
    move_right: Entity,
    move_forward: Entity,
    move_backward: Entity,
    move_up: Entity,
    move_down: Entity,
}

// Zustand für Turn-steuerung
#[derive(Resource, Default)]
struct TurnState {
    ready: bool,
}

#[derive(Component)]
struct KeyboardCamera;

#[derive(Resource, Default)]
struct MouseState {
    pitch: f32,
    yaw: f32,
}

// the component that will be used to play the animation
#[derive(Component)]
struct AnimationToPlay {
    graph_handle: Handle<AnimationGraph>,
    index: AnimationNodeIndex,
}

#[bevy_main]
fn main() {
    App::new()
        .add_plugins(
            add_xr_plugins(DefaultPlugins).build()
            .set(
                OxrInitPlugin {
                    exts: {
                        let mut exts = OxrExtensions::default();
                        exts.ext_hp_mixed_reality_controller = true;
                        exts
                    },
                    ..OxrInitPlugin::default()
                }
            )
            .set(
                WindowPlugin {
                    primary_window: Some(Window {
                        title: "Bevy OpenXR Morph Target Example".to_string(),
                        canvas: Some("#bevy-canvas".to_string()),
                        ..default()
                    }),
                    ..default()
                }
            ),
        )
        .insert_resource(OxrSessionConfig {
            blend_modes: Some({vec![
                    EnvironmentBlendMode::ALPHA_BLEND,
                    EnvironmentBlendMode::OPAQUE,
                ]}),
            ..OxrSessionConfig::default()
        })
        .add_plugins(schminput::DefaultSchminputPlugins)
        .add_plugins(transform_utils::TransformUtilitiesPlugin)
        .add_systems(PreStartup, setup_assets)
        .add_systems(Startup, setup_mesh_and_animation)
        .add_systems(Startup, setup)
        .add_systems(Startup, setup2)
        .add_systems(XrSessionCreated, create_view_space)
        .add_systems(Update, disable_indirect)
        .add_systems(Update, modify_msaa)
        .add_systems(Update, update_morph_targets)
        .add_systems(Update, run)
        .add_systems(Update, snap_turn_system)
        .add_systems(Update, move_keyboard)
        .add_systems(Update, mouse_look_system)
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(TurnState::default())
        .insert_resource(MouseState::default())
        .run();
}

#[derive(Component)]
struct MsaaModified;

fn modify_msaa(cams: Query<Entity, (With<Camera>, Without<MsaaModified>)>, mut commands: Commands) {
    for cam in &cams {
        commands.entity(cam).insert(Msaa::Off).insert(MsaaModified);
    }
}

fn disable_indirect(
    mut commands: Commands,
    cameras: Query<Entity, (With<Camera>, Without<NoIndirectDrawing>)>,) {
    for entity in cameras {
        commands.entity(entity).insert(bevy::render::view::NoIndirectDrawing);
    }
}

#[derive(Component)]
struct HeadsetView;

fn create_view_space(
    session: Res<OxrSession>, 
    mut commands: Commands
) {
    let space = session.create_reference_space(openxr::ReferenceSpaceType::VIEW, Transform::IDENTITY).unwrap();
    // get the XrSpace out of the XrReferenceSpace
    commands.spawn((HeadsetView,space.0));
}

fn setup_assets(mut commands: Commands, asset_server: Res<AssetServer>) {
    let elements = vec![
        AssetElement {
            name: "simpleHumanRig",
            asset: asset_server.load(GltfAssetLabel::Scene(0).from_asset(ASSET_ELEMENTS[SIMPLE_HUMAN_RIG_INDEX].file_name)),
        },
        AssetElement {
            name: "simpleWall",
            asset: asset_server.load(GltfAssetLabel::Scene(0).from_asset(ASSET_ELEMENTS[SIMPLE_WALL_INDEX].file_name)),
        },
    ];

    commands.insert_resource(AssetElementList { elements });
    info!("Maze elements loaded!");
}

fn setup_mesh_and_animation(
    mut commands: Commands,
    asset_elements: Res<AssetElementList>,
    asset_server: Res<AssetServer>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
) {
    if let Some(handle) = asset_elements.get_by_index(SIMPLE_HUMAN_RIG_INDEX) {
        let (graph, index) = AnimationGraph::from_clip(
            asset_server.load(GltfAssetLabel::Animation(0).from_asset(ASSET_ELEMENTS[SIMPLE_HUMAN_RIG_INDEX].file_name)),
        );

        // Store the animation graph as an asset.
        let graph_handle = graphs.add(graph);
        let animation_to_play = AnimationToPlay {
            graph_handle,
            index,
        };
        let mesh_scene = SceneRoot(handle.clone());
        let _entity = commands.spawn((
            Transform::from_xyz(0.0, 0.0, -5.0).with_rotation(
                Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2),
            ),
            animation_to_play,
            mesh_scene,
        )).observe(play_animation_when_ready).id();
    }
}

fn setup2(mut cmds: Commands) {
    let player_set = cmds.spawn(ActionSet::new("player", "Player", 1)).id();
    let pose_set = cmds.spawn(ActionSet::new("pose", "Poses", 0)).id();
    let move_action = cmds
        .spawn((
            Action::new("move", "Move", player_set),
            //OxrBindings::new().bindngs("/interaction_profiles/hp/mixed_reality_controller", ["/user/hand/left/input/thumbstick"]),
            OxrBindings::new().bindngs(OCULUS_TOUCH_PROFILE, ["/user/hand/left/input/thumbstick"]),
            Vec2ActionValue::new(),
        ))
        .id();
    let turn_action = cmds
        .spawn((
            Action::new("turn", "Turn", player_set),
            //OxrBindings::new().bindngs("/interaction_profiles/hp/mixed_reality_controller", ["/user/hand/right/input/thumbstick"]),
            OxrBindings::new().bindngs(OCULUS_TOUCH_PROFILE, ["/user/hand/right/input/thumbstick"]),
            Vec2ActionValue::new(),
        ))
        .id();
    let look = cmds
        .spawn((
            Action::new("look", "Look", player_set),
            //OxrBindings::new().bindngs("/interaction_profiles/hp/mixed_reality_controller",["/user/hand/right/input/thumbstick/x"]),
            OxrBindings::new().bindngs(OCULUS_TOUCH_PROFILE,["/user/hand/right/input/thumbstick/x"]),
            F32ActionValue::new(),
        ))
        .id();
    let jump = cmds
        .spawn((
            Action::new("jump", "Jump", player_set),
            //OxrBindings::new().bindngs("/interaction_profiles/hp/mixed_reality_controller", ["/user/hand/right/input/a/click"]),
            OxrBindings::new().bindngs(OCULUS_TOUCH_PROFILE, ["/user/hand/right/input/a/click"]),
            KeyboardBindings::new().bind(KeyboardBinding::new(KeyCode::Space)),
            GamepadBindings::new()
                .bind(GamepadBinding::new(GamepadBindingSource::South).button_just_pressed()),
            BoolActionValue::new(),
        ))
        .id();
    let move_left = cmds
        .spawn((
            Action::new("move_left", "Move Left", player_set),
            KeyboardBindings::new().bind(KeyboardBinding::new(KeyCode::KeyA)),
            BoolActionValue::new(),
        ))
        .id();
    let move_right = cmds
        .spawn((
            Action::new("move_right", "Move Right", player_set),
            KeyboardBindings::new().bind(KeyboardBinding::new(KeyCode::KeyD)),
            BoolActionValue::new(),
        ))
        .id();
    let move_forward = cmds
        .spawn((
            Action::new("move_forward", "Move Forward", player_set),
            KeyboardBindings::new().bind(KeyboardBinding::new(KeyCode::KeyW)),
            BoolActionValue::new(),
        ))
        .id();
    let move_backward = cmds
        .spawn((
            Action::new("move_backward", "Move Backward", player_set),
            KeyboardBindings::new().bind(KeyboardBinding::new(KeyCode::KeyS)),
            BoolActionValue::new(),
        ))
        .id();
    let move_up = cmds
        .spawn((
            Action::new("move_up", "Move Up", player_set),
            KeyboardBindings::new().bind(KeyboardBinding::new(KeyCode::KeyE)),
            BoolActionValue::new(),
        ))
        .id();
    let move_down = cmds
        .spawn((
            Action::new("move_down", "Move Down", player_set),
            KeyboardBindings::new().bind(KeyboardBinding::new(KeyCode::KeyQ)),
            BoolActionValue::new(),
        ))
        .id();
    let center_camera = cmds
        .spawn((
            Action::new("center_camera", "Center Camera", player_set),
            //OxrBindings::new().bindngs("/interaction_profiles/hp/mixed_reality_controller", ["/user/hand/left/input/y/click"]),
            OxrBindings::new().bindngs(OCULUS_TOUCH_PROFILE, ["/user/hand/left/input/y/click"]),
            GamepadBindings::new()
                .bind(GamepadBinding::new(GamepadBindingSource::East).button_just_pressed()),
            BoolActionValue::new(),
        ))
        .id();
    let left_hand = cmds.spawn(HandLeft).id();
    let right_hand = cmds.spawn(HandRight).id();
    let left_pose = cmds
        .spawn((
            Action::new("hand_left_pose", "Left Hand Pose", pose_set),
            //OxrBindings::new().bindngs("/interaction_profiles/hp/mixed_reality_controller", ["/user/hand/left/input/grip/pose"]),
            OxrBindings::new().bindngs(OCULUS_TOUCH_PROFILE, ["/user/hand/left/input/grip/pose"]),
            AttachSpaceToEntity(left_hand),
            SpaceActionValue::new(),
        ))
        .id();
    let right_pose = cmds
        .spawn((
            Action::new("hand_right_pose", "Right Hand Pose", pose_set),
            //OxrBindings::new().bindngs("/interaction_profiles/hp/mixed_reality_controller", ["/user/hand/right/input/aim/pose"]),
            OxrBindings::new().bindngs(OCULUS_TOUCH_PROFILE, ["/user/hand/right/input/aim/pose"]),
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
        center_camera,
        move_left,
        move_right,
        move_forward,
        move_backward,
        move_up,
        move_down,
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

fn move_keyboard(
    move_actions: Res<MoveActions>,
    bool_value: Query<&BoolActionValue>,
    mut camera_query: Query<&mut Transform, With<KeyboardCamera>>,
    time: Res<Time>,
) {
    let mut moved = false;
    let mut direction = Vec3::ZERO;
    let mut direction_up_down = Vec3::ZERO;
    let speed = 1.0;

    if bool_value.get(move_actions.move_left).unwrap().any {
        direction.x -= 1.0;
        moved = true;
    }
    if bool_value.get(move_actions.move_right).unwrap().any {
        direction.x += 1.0;
        moved = true;
    }
    if bool_value.get(move_actions.move_forward).unwrap().any {
        direction.z -= 1.0;
        moved = true;
    }
    if bool_value.get(move_actions.move_backward).unwrap().any {
        direction.z += 1.0;
        moved = true;
    }
    if bool_value.get(move_actions.move_up).unwrap().any {
        direction_up_down.y += 1.0;
        moved = true;
    }
    if bool_value.get(move_actions.move_down).unwrap().any {
        direction_up_down.y -= 1.0;
        moved = true;
    }

    if moved {
        let delta = time.delta_secs();
        for mut transform in camera_query.iter_mut() {
            // Bewegung in Blickrichtung (lokaler Raum)
            let mut local_direction = transform.rotation * direction.normalize_or_zero();
            local_direction.y = 0.0; // Keine vertikale Bewegung durch Blickrichtung
            transform.translation += (local_direction * speed + direction_up_down * speed) * delta;
        }
    }
}

fn mouse_look_system(
    mut mouse_state: ResMut<MouseState>,
    mut camera_query: Query<&mut Transform, With<KeyboardCamera>>,
    mut mouse_motion_events: EventReader<MouseMotion>,
) {
    
    let mut delta = Vec2::ZERO;
    for event in mouse_motion_events.read() {
        delta += event.delta * 4.0; // scaling for higher sensitivity
    }
    if delta == Vec2::ZERO {
        return;
    }

    // Empfindlichkeit anpassen
    let sensitivity = 0.005;
    mouse_state.yaw -= delta.x * sensitivity;
    mouse_state.pitch -= delta.y * sensitivity;
    mouse_state.pitch = mouse_state.pitch.clamp(-1.54, 1.54); // ca. +/- 88 Grad

    for mut transform in camera_query.iter_mut() {
        transform.rotation = Quat::from_axis_angle(Vec3::Y, mouse_state.yaw)
            * Quat::from_axis_angle(Vec3::X, mouse_state.pitch);
    }
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
    left_hand: Query<&GlobalTransform, With<HandLeft>>,
    right_hand: Query<&GlobalTransform, With<HandRight>>,
    mut gizmos: Gizmos,
    mut root_query: Query<&mut Transform, With<XrTrackingRoot>>,
) {
    let movevals = vec2_value.get(move_actions.move_action).unwrap().any;
    let mut delta = Vec3::ZERO;
    if movevals.length_squared() > 0.05
        && let Ok(mut root_transform) = root_query.single_mut() 
        && let Some(hand) = right_hand.iter().next() {
        let pose = hand.to_isometry();
        
        let forward = pose.rotation.mul_vec3(-Vec3::Z).normalize();
        let right = pose.rotation.mul_vec3(Vec3::X).normalize();
        info!("forward: {:?}", forward);
        info!("right: {:?}", right);
        delta = forward * movevals.y * 0.05 + right * movevals.x * 0.05;
        root_transform.translation += delta;
        
    }
    for hand in left_hand.into_iter() {
        let mut pose = hand.to_isometry();
        pose.translation += Vec3A::from(delta);
        gizmos.arrow(Vec3 { x: 0.0, y: 0.0, z: 0.0 }, pose.rotation.mul_vec3(-Vec3::Z).normalize(), css::BLUE);
        gizmos.sphere(pose, 0.1, css::BLUE);
    }
    for hand in right_hand.into_iter() {
        let mut pose = hand.to_isometry();
        pose.translation += Vec3A::from(delta);
        gizmos.arrow(Vec3 { x: 0.0, y: 0.0, z: 0.0 }, pose.rotation.mul_vec3(-Vec3::Z).normalize(), css::RED);
        gizmos.sphere(pose, 0.1, css::RED);
    }
}

fn snap_turn_system(
    turn_actions: Res<MoveActions>,
    mut root_query: Query<&mut Transform, With<XrTrackingRoot>>,
    vec2_value: Query<&Vec2ActionValue>,
    mut turn_state: ResMut<TurnState>,
) {
    let movevals = vec2_value.get(turn_actions.turn_action).unwrap().any;
    
    let turn_value = movevals.x;

    // activate Snap-Turn only if the thumbstick is clearly moved
    if turn_value.abs() > 0.8 && turn_state.ready
        && let Ok(mut transform) = root_query.single_mut() {
        let angle = if turn_value > 0.0 { -FRAC_PI_4 } else { FRAC_PI_4 }; // Rechts = negative Rotation
        transform.rotate(Quat::from_rotation_y(angle));
        turn_state.ready = false;
        
    }

    // only one turn per thumbstick movement
    if turn_value.abs() < 0.2 {
        turn_state.ready = true;
    }
}