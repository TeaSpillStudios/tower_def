use bevy::log::{debug, Level, LogSettings};
use bevy::{prelude::*, utils::FloatOrd};
use bevy::window::PresentMode;
use bevy_asset_loader::prelude::*;
use bevy_editor_pls::prelude::*;
use bevy_embedded_assets::EmbeddedAssetPlugin;

fn eul_to_rad(deg: f32) -> f32 {
    deg * std::f32::consts::PI / 180.0
}

#[derive(Reflect, Component, Default)]
#[reflect(Component)]
pub struct Bullet {
    direction: Vec3,
    speed: f32,
}

#[derive(Reflect, Component, Default)]
#[reflect(Component)]
pub struct Target {
    speed: f32,
}

#[derive(Reflect, Component, Default)]
#[reflect(Component)]
pub struct Health {
    value: i32,
}

#[derive(Reflect, Component, Default)]
pub struct Lifetime {
    timer: Timer,
}

#[derive(Reflect, Component, Default)]
#[reflect(Component)]
pub struct TowerBase {}

#[derive(Reflect, Component, Default)]
#[reflect(Component)]
pub struct Tower {
    shooting_timer: Timer,
    bullet_offset: Vec3,
}

fn main() {
    App::new()
        .insert_resource(LogSettings {
            level: Level::DEBUG,
            ..default()
        })
        .insert_resource(WindowDescriptor {
            width: 1920.0,
            height: 1080.0,
            title: "Tower Defense Game".to_string(),
            present_mode: PresentMode::AutoVsync,
            ..default()
        })
        .insert_resource(ClearColor(Color::rgb(0.25, 0.25, 0.25)))
        .insert_resource(Msaa { samples: 4 })
        .add_loading_state(
            LoadingState::new(GameState::AssetLoading)
                .continue_to_state(GameState::Next)
                .with_collection::<GameAssets>(),
        )
        .add_state(GameState::AssetLoading)
        .add_plugins_with(DefaultPlugins, |group| {
            group.add_before::<bevy::asset::AssetPlugin, _>(EmbeddedAssetPlugin)
        })
        .add_plugin(EditorPlugin)
        //.add_plugin(WorldInspectorPlugin::new())
        .add_system_set(SystemSet::on_enter(GameState::Next).with_system(spawn_basic_scene))
        .add_startup_system(spawn_camera)
        //.add_system_set(SystemSet::on_update(GameState::Next).with_system(add_barrel))
        .add_system_set(SystemSet::on_update(GameState::Next).with_system(tower_shooting))
        .add_system_set(SystemSet::on_update(GameState::Next).with_system(bullet_despawn))
        .add_system_set(SystemSet::on_update(GameState::Next).with_system(move_targets))
        .add_system_set(SystemSet::on_update(GameState::Next).with_system(move_bullets))
        .register_type::<Tower>()
        .register_type::<Lifetime>()
        .register_type::<Target>()
        .run();
}

#[derive(AssetCollection)]
struct GameAssets {
    #[asset(path = "TowerBase.glb#Scene0")]
    tower_base_scene: Handle<Scene>,
    #[asset(path = "TowerBarrel.glb#Scene0")]
    tower_barrel_scene: Handle<Scene>,
    #[asset(path = "Bullet.glb#Scene0")]
    bullet_scene: Handle<Scene>,
    #[asset(path = "Enemy.glb#Scene0")]
    target_scene: Handle<Scene>,
}

fn move_bullets(mut bullets: Query<(&Bullet, &mut Transform)>, time: Res<Time>) {
    for (bullet, mut transform) in &mut bullets {
        transform.translation += bullet.direction.normalize() * bullet.speed * time.delta_seconds();
    }
}

fn move_targets(mut targets: Query<(&Target, &mut Transform)>, time: Res<Time>) {
    for (target, mut transform) in &mut targets {
        transform.translation.x += target.speed * time.delta_seconds();
    }
}

fn tower_shooting(
    mut commands: Commands,
    mut towers: Query<(Entity, &mut Tower, &GlobalTransform)>,
    targets: Query<&GlobalTransform, With<Target>>,
    assets: Res<GameAssets>,
    time: Res<Time>,
) {
    for (tower_ent, mut tower, transform) in &mut towers {
        tower.shooting_timer.tick(time.delta());

        if tower.shooting_timer.just_finished() {
            let bullet_spawn = transform.translation() + tower.bullet_offset;

            let direction = targets
                .iter()
                .min_by_key(|target_transform| {
                    FloatOrd(Vec3::distance(target_transform.translation(), bullet_spawn))
                })
                .map(|closest_target| closest_target.translation() - bullet_spawn);

            if let Some(direction) = direction {
                commands.entity(tower_ent).with_children(|commands| {
                    commands
                        .spawn_bundle(SceneBundle {
                            scene: assets.bullet_scene.clone(),
                            transform: Transform::from_translation(tower.bullet_offset),
                            ..Default::default()
                        })
                        .insert(Lifetime {
                            timer: Timer::from_seconds(2.5, false),
                        })
                        .insert(Bullet {
                            direction,
                            speed: 2.5,
                        })
                        .insert(Name::new("Bullet"));
                });
                debug!(?direction.x, ?direction.y, ?direction.z);
            }
        }
    }
}

fn bullet_despawn(
    mut commands: Commands,
    mut bullets: Query<(Entity, &mut Lifetime)>,
    time: Res<Time>,
) {
    for (entity, mut lifetime) in &mut bullets {
        lifetime.timer.tick(time.delta());

        if lifetime.timer.just_finished() {
            commands.entity(entity).despawn_recursive();
        }
    }
}

fn spawn_camera(mut commands: Commands) {
    commands
        .spawn_bundle(Camera3dBundle {
            transform: Transform::from_xyz(5.0, 5.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        })
        .insert(Name::new("Camera"));
}

fn spawn_basic_scene(
    assets: Res<GameAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut commands: Commands,
) {
    commands
        .spawn_bundle(PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Plane { size: 50.0 })),
            material: materials.add(Color::rgb(0.3, 0.3, 0.3).into()),
            ..default()
        })
        .insert(Name::new("Ground"));

    commands
        .spawn_bundle(SceneBundle {
            scene: assets.target_scene.clone(),
            transform: Transform::from_xyz(-2.0, 0.5, 1.5)
                .with_rotation(Quat::from_rotation_y(eul_to_rad(90.0))),
            ..default()
        })
        .insert(Target { speed: 0.3 })
        .insert(Health { value: 3 })
        .insert(Name::new("Target"));

    commands
        .spawn_bundle(SceneBundle {
            scene: assets.tower_base_scene.clone(),
            transform: Transform::from_xyz(0.0, 0.75, 0.0),
            ..default()
        })
        .insert(TowerBase {})
        .insert(Name::new("Tower"));

    commands
        .spawn_bundle(PointLightBundle {
            point_light: PointLight {
                intensity: 1500.0,
                shadows_enabled: true,
                ..default()
            },

            transform: Transform::from_xyz(-4.0, 8.0, 4.0),
            ..default()
        })
        .insert(Name::new("Light"));
}

fn add_barrel(
    mut commands: Commands,
    tower_bases: Query<&Transform, With<TowerBase>>,
    assets: Res<GameAssets>,
) {
    debug!("I'm here smh");

    for transform in &tower_bases {
        debug!("Tower base found");

        commands.spawn_bundle(SceneBundle {
            scene: assets.tower_barrel_scene.clone(),
            transform: Transform::from_translation(transform.translation),
            ..default()
        })
        .insert(Tower {
            shooting_timer: Timer::from_seconds(1.0, true),
            bullet_offset: Vec3::new(0.0, 0.5, 0.0)
        })
        .insert(Name::new("TowerBarrel"));
    }
}

#[derive(Clone, Eq, PartialEq, Debug, Hash)]
enum GameState {
    AssetLoading,
    Next,
}
