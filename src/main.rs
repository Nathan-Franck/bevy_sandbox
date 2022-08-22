use bevy::prelude::*;
use bevy_inspector_egui::WorldInspectorPlugin;
use bevy_rapier3d::{prelude::*, rapier::prelude::ColliderShape};
use rand::prelude::random;
use std::{collections::HashMap, slice::Iter};

fn main() {
    App::new()
        .insert_resource(WindowDescriptor {
            title: "Combine".to_string(),
            width: 805.,
            height: 484.,
            position: WindowPosition::At(Vec2 { x: 1106., y: 516. }),
            present_mode: bevy::window::PresentMode::AutoVsync,
            ..default()
        })
        .add_plugins(DefaultPlugins)
        .add_plugin(RapierPhysicsPlugin::<NoUserData>::default())
        // .add_plugin(RapierDebugRenderPlugin::default())
        .add_system(connect_from_scene)
        .add_system(resize_notificator)
        .add_system(play_on_load)
        .add_system(gamepad_system)
        .add_startup_system(setup)
        .add_startup_system(setup_physics)
        .add_plugin(WorldInspectorPlugin::new())
        .run();
}

fn setup_physics(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // /* Create the ground. */
    // commands
    //     .spawn()
    //     .insert(Collider::cuboid(100.0, 0.1, 100.0))
    //     .insert_bundle(TransformBundle::from(Transform::from_xyz(0.0, -2.0, 0.0)));
    // add entities to the world
    for y in -2..=2 {
        for x in -5..=5 {
            // sphere
            commands
                .spawn_bundle(PbrBundle {
                    mesh: meshes.add(Mesh::from(shape::Icosphere {
                        radius: 0.45,
                        subdivisions: 32,
                    })),
                    material: materials.add(StandardMaterial {
                        base_color: Color::hsl(random::<f32>() * 256., 1., 0.5),
                        metallic: 0.,
                        perceptual_roughness: 0.,
                        ..default()
                    }),
                    transform: Transform::from_xyz(x as f32, y as f32 + 0.5, 0.0),
                    ..default()
                })
                .insert(RigidBody::Dynamic)
                .insert(Collider::ball(0.5))
                .insert(Restitution::coefficient(0.7));
        }
    }
}

const CHARACTER_SPEED: f32 = 10.0;

type Animations = HashMap<AnimationID, Handle<AnimationClip>>;

#[derive(Component)]
struct Player;

#[derive(Clone, Eq, PartialEq, Hash)]
enum AnimationID {
    Idle,
    Walk,
}

fn connect_from_scene(
    named_entities: Query<(Entity, &Name), (With<Transform>, Added<Name>)>,
    named_entities_with_children: Query<(Entity, &Name, &Children), Added<Name>>,
    meshes: Query<&Handle<Mesh>>,
    mesh_assets: Res<Assets<Mesh>>,
    mut commands: Commands,
) {
    for (_, name) in named_entities.iter() {
        println!("{}", name);
    }
    let bunnies = named_entities
        .iter()
        .filter(|&(_, name)| name.eq(&Name::new("Puppy")));
    for (entity, name) in bunnies {
        println!("{}", name);
        commands
            .entity(entity)
            .insert(Player)
            .insert(RigidBody::Dynamic)
            .insert(Collider::ball(2.0))
            .insert(Restitution::coefficient(0.7))
            .insert(LockedAxes::ROTATION_LOCKED);
    }
    let level = named_entities_with_children
        .iter()
        .filter(|&(_, name, _)| name.eq(&Name::new("Level")));
    for (entity, name, children) in level {
        let (child_mesh_entities, _): (Vec<_>, Vec<_>) = children
            .iter()
            .map(|&child| meshes.get(child))
            .partition(Result::is_ok);
        let child_mesh_entities: Vec<_> = child_mesh_entities.into_iter().map(Result::unwrap).collect();
        for mesh in child_mesh_entities {
            commands.entity(entity).insert(
                Collider::from_bevy_mesh(
                    mesh_assets.get(mesh).unwrap(),
                    &ComputedColliderShape::TriMesh,
                )
                .unwrap(),
            );
        }
        println!("Level Geometry Found: {}", name);
    }
}

fn gamepad_system(
    time: Res<Time>,
    gamepads: Res<Gamepads>,
    axes: Res<Axis<GamepadAxis>>,
    camera: Query<&Parent, With<Camera3d>>,
    global_transforms: Query<&GlobalTransform>,
    mut player: Query<&mut Transform, With<Player>>,
) {
    for gamepad in gamepads.iter().cloned() {
        let left_stick = Vec3 {
            x: axes
                .get(GamepadAxis::new(gamepad, GamepadAxisType::LeftStickX))
                .unwrap(),
            z: -axes
                .get(GamepadAxis::new(gamepad, GamepadAxisType::LeftStickY))
                .unwrap(),
            ..default()
        };

        let camera_relative_input = if let Ok(camera_root) = camera.get_single() {
            let camera_transform = global_transforms.get(camera_root.get()).unwrap();
            let (_, camera_rotation, _) = camera_transform.to_scale_rotation_translation();
            let flat_camera_rotation = Quat::from_axis_angle(
                Vec3::Y,
                ((camera_rotation * Vec3::Z)
                    * Vec3 {
                        x: 1.,
                        y: 0.,
                        z: 1.,
                    })
                .angle_between(Vec3::Z),
            );
            flat_camera_rotation * left_stick
        } else {
            left_stick
        };

        for mut transform in player.iter_mut() {
            let mut translation = transform.translation;
            translation += camera_relative_input * time.delta_seconds() * CHARACTER_SPEED;

            // TEMP limit on position to keep player in-frame
            if translation.length() > 10. {
                translation *= 10. / translation.length();
            }
            transform.translation = translation;
        }
    }
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    asset_server.watch_for_changes().unwrap();

    // load scene
    commands.spawn_bundle(SceneBundle {
        scene: asset_server.load("animals.gltf#Scene0"),
        ..default()
    });

    // load animations
    commands.insert_resource(Animations::from_iter([
        (
            AnimationID::Idle,
            asset_server.load("animals.gltf#Animation0") as Handle<AnimationClip>,
        ),
        (
            AnimationID::Walk,
            asset_server.load("animals.gltf#Animation1") as Handle<AnimationClip>,
        ),
    ]));

    commands.spawn_bundle(DirectionalLightBundle {
        transform: Transform {
            rotation: Quat::from_euler(EulerRot::XYZ, -45., 0., 0.),
            ..Default::default()
        },
        directional_light: DirectionalLight {
            ..Default::default()
        },
        ..Default::default()
    });

    commands.spawn_bundle(PointLightBundle {
        transform: Transform {
            translation: Vec3 {
                x: 0.,
                y: -10.,
                z: 0.,
            },
            rotation: Quat::from_euler(EulerRot::XYZ, 45., 0., 0.),
            ..Default::default()
        },
        point_light: PointLight {
            color: Color::Rgba {
                red: 0.,
                green: 1.,
                blue: 0.,
                alpha: 1.,
            },
            range: 50.,
            radius: 10.,
            intensity: 3000.,
            ..Default::default()
        },
        ..Default::default()
    });
}

fn resize_notificator(
    resize_event: Res<Events<bevy::window::WindowResized>>,
    move_event: Res<Events<bevy::window::WindowMoved>>,
) {
    for e in resize_event.get_reader().iter(&resize_event) {
        println!("width: {} height: {}", e.width, e.height);
    }
    for e in move_event.get_reader().iter(&move_event) {
        println!("x: {} y: {}", e.position.x, e.position.y);
    }
}

fn play_on_load(
    mut animations: ResMut<Animations>,
    mut animation_clips: ResMut<Assets<AnimationClip>>,
    mut players: Query<&mut AnimationPlayer, Added<AnimationPlayer>>,
) {
    // HACK - offset animations to start at 0, requires animations to have a keyframe at the 0th frame of their "action" (Blender term)
    for (animation_id, animation_handle) in animations.clone().iter() {
        if let Some(animation_clip) = animation_clips.get_mut(&animation_handle) {
            let existing_animation_clip = animation_clip.clone();
            let curves_map = existing_animation_clip.curves();
            let mut new_animation_clip = AnimationClip::default();
            for (path, curves) in curves_map.iter() {
                for curve in curves.iter() {
                    if let Some(first_time) = curve.keyframe_timestamps.first() {
                        new_animation_clip.add_curve_to_path(
                            path.clone(),
                            VariableCurve {
                                keyframe_timestamps: curve
                                    .keyframe_timestamps
                                    .iter()
                                    .map(|timestamp| timestamp - first_time)
                                    .collect(),
                                keyframes: curve.keyframes.clone(),
                            },
                        );
                    }
                }
            }
            animations.insert(
                animation_id.clone(),
                animation_clips.set(animation_handle, new_animation_clip),
            );
        }
    }

    // start playing animation
    for mut player in players.iter_mut() {
        player.play(animations[&AnimationID::Idle].clone()).repeat();
    }
}
