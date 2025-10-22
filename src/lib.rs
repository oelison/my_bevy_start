// This example demonstrates how to load a glTF model with morph targets using Bevy and OpenXR.
// It uses the Bevy engine for rendering and OpenXR for VR support.
// The model is loaded from a file and displayed in a VR environment.
// The example includes a simple setup for a Bevy app with OpenXR integration.

mod asset_handler;
use asset_handler::{AssetElement, AssetElementList, ASSET_ELEMENTS, MAX_ASSET_ELEMENTS};

use core::f32;
use std::{f32::consts::FRAC_PI_4, ops::DerefMut};

use bevy_mod_openxr::session::OxrSession;

use bevy::{
    animation::AnimationTarget, camera::primitives::Aabb, color::palettes::css::{self, WHITE}, gltf::{GltfMaterialName, GltfMeshExtras, GltfMeshName}, light::{AmbientLight, CascadeShadowConfigBuilder, DirectionalLight}, log::LogPlugin, mesh::{morph::MeshMorphWeights, skinning::SkinnedMesh}, prelude::*, render::view::NoIndirectDrawing, scene::SceneInstanceReady
};
use bevy_mod_openxr::{
    add_xr_plugins,
    exts::OxrExtensions,
    init::OxrInitPlugin,
    resources::OxrSessionConfig,
};

use bevy_mod_xr::session::{XrSessionCreated, XrTrackingRoot};
use bevy_mod_xr::camera::XrProjection;
use bevy_xr_utils::transform_utils::{self};
use bevy::prelude::MorphWeights;
use schminput::prelude::*;
use bevy::input::mouse::MouseMotion;

use bevy::asset::AssetMetaCheck;

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
    new_scene: Entity,
    center_camera: Entity,
    move_left: Entity,
    move_right: Entity,
    move_forward: Entity,
    move_backward: Entity,
    move_up: Entity,
    move_down: Entity,
    shown_scene: usize,
    new_scene_released: bool,
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
    unsafe {
        std::env::set_var("RUST_BACKTRACE", "1");
        std::env::set_var("RUST_LOG", "wgpu=trace,bevy_render=debug,bevy_gltf=debug,bevy_asset=debug");
        std::env::set_var("WGPU_BACKEND", "vulkan");
        std::env::set_var("WGPU_TRACE", "/sdcard/wgpu_trace");
    }

    App::new()
        .add_plugins(
            add_xr_plugins(DefaultPlugins).build()
            .set(AssetPlugin {
                meta_check: AssetMetaCheck::Never,
                ..default()
            })
            .set(
                LogPlugin {
                    filter: "wgpu=info,bevy_render=info,bevy_asset=debug,bevy_gltf=debug".into(),
                    ..default()
                }
            )
            .set(
                OxrInitPlugin {
                    exts: {
                        let mut exts = OxrExtensions::default();
                        exts.enable_fb_passthrough();
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
            ..OxrSessionConfig::default()
        })
        .add_plugins(schminput::DefaultSchminputPlugins)
        .add_plugins(transform_utils::TransformUtilitiesPlugin)
        .add_systems(PreStartup, setup_assets)
        .add_systems(Startup, setup_mesh_and_animation)
        .add_systems(Startup, setup)
        .add_systems(Startup, setup2)
        .add_systems(XrSessionCreated, create_view_space)
        .add_systems(Update, modify_cams)
        .add_systems(Update, adjust_near_plane)
        .add_systems(Update, update_morph_targets)
        .add_systems(Update, run)
        .add_systems(Update, snap_turn_system)
        .add_systems(Update, move_keyboard)
        .add_systems(Update, mouse_look_system)
        .add_systems(Update, animate_light_direction)
        .add_systems(Update, spawn_new_scene)
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(TurnState::default())
        .register_type::<Transform>()
        .register_type::<GlobalTransform>()
        .register_type::<TransformTreeChanged>()
        .register_type::<ChildOf>()
        .register_type::<Children>()
        .register_type::<Visibility>()
        .register_type::<InheritedVisibility>()
        .register_type::<ViewVisibility>()
        .register_type::<Name>()
        .register_type::<AnimationTarget>()
        .register_type::<MorphWeights>()
        .register_type::<AnimationPlayer>()
        .register_type::<Mesh3d>()
        .register_type::<MeshMorphWeights>()
        .register_type::<Aabb>()
        .register_type::<GltfExtras>()
        .register_type::<GltfMeshExtras>()
        .register_type::<GltfMeshName>()
        .register_type::<GltfMaterialName>()
        .register_type::<SkinnedMesh>()
        .insert_resource(MouseState::default())
        .run();
}

#[derive(Component)]
struct CamModified;

fn modify_cams(cams: Query<Entity, (With<Camera>, Without<CamModified>)>, mut commands: Commands) {
    for cam in &cams {
        commands.entity(cam)
        .insert(Msaa::Off)
        .insert(NoIndirectDrawing)
        .insert(CamModified);
    }
}

fn adjust_near_plane(query: Query<&mut Projection, With<Camera3d>>) {
    // Safely get the single camera projection if present and adjust its near plane when it's perspective.
    for mut projection in query {
        match projection.deref_mut() {
            Projection::Perspective(perspective_projection) => perspective_projection.near = 0.003,
            Projection::Orthographic(orthographic_projection) => orthographic_projection.near = 0.003,
            Projection::Custom(custom_projection) => {
                if let Some(xr) = custom_projection.get_mut::<XrProjection>() {
                    xr.near = 0.1;
                } else {
                    error_once!("unknown custom camera projection");
                }
            }
        }
    }
    
}

#[derive(Component)]
struct HeadsetView;

fn create_view_space(
    session: Res<OxrSession>, 
    mut commands: Commands
) {
    let space = session.create_reference_space(openxr::ReferenceSpaceType::VIEW, Isometry3d::IDENTITY).unwrap();
    // get the XrSpace out of the XrReferenceSpace
    commands.spawn((HeadsetView,space.0));
}

fn setup_assets(mut commands: Commands, asset_server: Res<AssetServer>) {
    let mut elements = vec![];
    for asset in ASSET_ELEMENTS {
        info!("Loading asset: {}", asset.file_name);
        elements.push(AssetElement { asset: asset_server.load(GltfAssetLabel::Scene(0).from_asset(asset.file_name)) });
    }
    commands.insert_resource(AssetElementList { elements });
    info!("gltf elements loaded!");
}

fn setup_mesh_and_animation(
    mut commands: Commands,
    asset_elements: Res<AssetElementList>,
    asset_server: Res<AssetServer>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
) {
    if let Some(handle) = asset_elements.get_by_index(0) {
        let (graph, index) = AnimationGraph::from_clip(
            asset_server.load(GltfAssetLabel::Animation(0).from_asset(ASSET_ELEMENTS[0].file_name)),
        );

        // Store the animation graph as an asset.
        let graph_handle = graphs.add(graph);
        let animation_to_play = AnimationToPlay {
            graph_handle,
            index,
        };
        let mesh_scene = SceneRoot(handle.clone());
        let _entity = commands.spawn((
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
            OxrBindings::new().bindings(OCULUS_TOUCH_PROFILE, ["/user/hand/left/input/thumbstick"]),
            Vec2ActionValue::new(),
        ))
        .id();
    let turn_action = cmds
        .spawn((
            Action::new("turn", "Turn", player_set),
            //OxrBindings::new().bindngs("/interaction_profiles/hp/mixed_reality_controller", ["/user/hand/right/input/thumbstick"]),
            OxrBindings::new().bindings(OCULUS_TOUCH_PROFILE, ["/user/hand/right/input/thumbstick"]),
            Vec2ActionValue::new(),
        ))
        .id();
    let look = cmds
        .spawn((
            Action::new("look", "Look", player_set),
            //OxrBindings::new().bindngs("/interaction_profiles/hp/mixed_reality_controller",["/user/hand/right/input/thumbstick/x"]),
            OxrBindings::new().bindings(OCULUS_TOUCH_PROFILE,["/user/hand/right/input/thumbstick/x"]),
            F32ActionValue::new(),
        ))
        .id();
    let new_scene = cmds
        .spawn((
            Action::new("new_scene", "New scene", player_set),
            //OxrBindings::new().bindngs("/interaction_profiles/hp/mixed_reality_controller", ["/user/hand/right/input/a/click"]),
            OxrBindings::new().bindings(OCULUS_TOUCH_PROFILE, ["/user/hand/right/input/a/click"]),
            KeyboardBindings::new().bind(KeyboardBinding::new(KeyCode::KeyI)),
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
            OxrBindings::new().bindings(OCULUS_TOUCH_PROFILE, ["/user/hand/left/input/y/click"]),
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
            OxrBindings::new().bindings(OCULUS_TOUCH_PROFILE, ["/user/hand/left/input/grip/pose"]),
            AttachSpaceToEntity(left_hand),
            SpaceActionValue::new(),
        ))
        .id();
    let right_pose = cmds
        .spawn((
            Action::new("hand_right_pose", "Right Hand Pose", pose_set),
            //OxrBindings::new().bindngs("/interaction_profiles/hp/mixed_reality_controller", ["/user/hand/right/input/aim/pose"]),
            OxrBindings::new().bindings(OCULUS_TOUCH_PROFILE, ["/user/hand/right/input/aim/pose"]),
            AttachSpaceToEntity(right_hand),
            SpaceActionValue::new(),
        ))
        .id();
    cmds.insert_resource(MoveActions {
        set: player_set,
        move_action,
        turn_action,
        look,
        new_scene,
        center_camera,
        move_left,
        move_right,
        move_forward,
        move_backward,
        move_up,
        move_down,
        shown_scene: 0,
        new_scene_released: true,
    });
    cmds.insert_resource(CoreActions {
        set: pose_set,
        left_pose,
        right_pose,
    });
}

fn setup(
    mut commands: Commands,
) {
    commands.spawn((
        DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
        // This is a relatively small scene, so use tighter shadow
        // cascade bounds than the default for better quality.
        // We also adjusted the shadow map to be larger since we're
        // only using a single cascade.
        CascadeShadowConfigBuilder {
            num_cascades: 1,
            maximum_distance: 4.0,
            ..default()
        }
        .build(),
    ));
    commands.insert_resource(
        AmbientLight {
            color: WHITE.into(),
            brightness: 400.0,
            ..default()
        }
    );
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
    mut mouse_motion_events: MessageReader<MouseMotion>,
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

fn spawn_new_scene(
    mut commands: Commands,
    query: Query<Entity, With<SceneRoot>>,
    assets: Res<AssetElementList>,
    mut move_actions: ResMut<MoveActions>,
    bool_value: Query<&BoolActionValue>,
) {
    if !bool_value.get(move_actions.new_scene).unwrap().any {
        if !move_actions.new_scene_released {
            info!("Button released");
        }
        move_actions.new_scene_released = true;
        return;
    }
    if !move_actions.new_scene_released {
        return;
    }
    info!("Spawning new scene index {}", move_actions.shown_scene);
    move_actions.new_scene_released = false;
    move_actions.shown_scene += 1;
    if move_actions.shown_scene >= MAX_ASSET_ELEMENTS {
        move_actions.shown_scene = 0;
    }
    for entity in query.iter() {
        info!("despawn: {}", entity.index().to_string());
        commands.entity(entity).despawn();
    }
    // Function to spawn a new scene if needed
    if let Some(handle) = assets.get_by_index(move_actions.shown_scene) {
        let _entity = commands.spawn((
            Transform::from_xyz(0.0, 0.0, 0.0),
            SceneRoot(handle.clone(),
        )
        )).id();
    } else {
        info!("No asset found for index {}", move_actions.shown_scene);
    }
}

fn animate_light_direction(
    time: Res<Time>,
    mut query: Query<&mut Transform, With<DirectionalLight>>,
) {
    for mut transform in &mut query {
        transform.rotation = Quat::from_euler(
            EulerRot::ZYX,
            0.0,
            time.elapsed_secs() * f32::consts::PI / 5.0,
            -FRAC_PI_4,
        );
    }
}
// is called when the scene is loaded
// this is where we play the animation (head nodding)
fn play_animation_when_ready(
    trigger: On<SceneInstanceReady>,
    mut commands: Commands,
    children: Query<&Children>,
    animations_to_play: Query<&AnimationToPlay>,
    mut players: Query<&mut AnimationPlayer>,
) {
    // The entity we spawned in `setup_mesh_and_animation` is the trigger's target.
    // Start by finding the AnimationToPlay component we added to that entity.
    if let Ok(animation_to_play) = animations_to_play.get(trigger.entity) {
        // The SceneRoot component will have spawned the scene as a hierarchy
        // of entities parented to our entity. Since the asset contained a skinned
        // mesh and animations, it will also have spawned an animation player
        // component. Search our entity's descendants to find the animation player.
        for child in children.iter_descendants(trigger.entity) {
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
    mut gizmos: bevy_gizmos::gizmos::Gizmos,
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
        delta = forward * movevals.y * 0.01 + right * movevals.x * 0.01;
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
    headset_view_query: Query<&Transform, (With<HeadsetView>, Without<XrTrackingRoot>)>
) {
    let movevals = vec2_value.get(turn_actions.turn_action).unwrap().any;
    
    let turn_value = movevals.x;

    // activate Snap-Turn only if the thumbstick is clearly moved
    if turn_value.abs() > 0.8 && turn_state.ready {
        if let Ok(mut root_transform) = root_query.single_mut() {
            if let Ok(headset_transform) = headset_view_query.single() {
                let root_translation = root_transform.translation;
                let root_rotation = root_transform.rotation;
                let local_headset = headset_transform.translation;
                let world_headset = root_translation + root_rotation * local_headset;
                let angle = if turn_value > 0.0 { -FRAC_PI_4 } else { FRAC_PI_4 }; // right = negative Rotation
                root_transform.rotate_around(world_headset, Quat::from_rotation_y(angle));
                turn_state.ready = false;
            } else {
                info!("No headset view found, cannot rotate.");
            }
        } else {
            info!("No root transform found, cannot rotate.");
        }
    }

    // only one turn per thumbstick movement
    if turn_value.abs() < 0.2 {
        turn_state.ready = true;
    }
}