use bevy::prelude::*;

use crate::sheep;
use crate::utils::{
    bounds_check, AttackRange, AttackValue, Bounds, Health, PursuitType, Speed, SpottingRange,
};
use rand::{thread_rng, Rng};

use crate::GameState;

// Every WarMachine is defined by:
// - `SpottingRange`: if a sheep is found within this radius, it will be pursued
// - `AttackRange`: if a sheep is within this radius, it will be attacked by `AttackValue`
// - `AttackValue`: attack damage value
// - `Health`:  if health value falls below 0, it dies
// - `Speed`: how fast it moves
// - `PursuitType`: how it selects the next sheep to hunt
// - any other traits that may alter behaviour
#[derive(Component, Default)]
pub struct WarMachine;

pub struct Level(pub usize);

const BATTLEFIELD_BOUNDS_X: Vec2 = Vec2::new(-6.2, 6.2);
const BATTLEFIELD_BOUNDS_Y: Vec2 = Vec2::new(-6.4, 7.0);

pub struct BattlePlugin;

impl Plugin for BattlePlugin {
    fn build(&self, app: &mut App) {
        app.add_system_set(SystemSet::on_enter(GameState::Battle).with_system(init_level))
            .add_system_set(
                SystemSet::on_update(GameState::Battle)
                    .with_system(move_and_attack)
                    .with_system(remove_dead_sheep)
                    .with_system(sheep::update_sheep)
                    .with_system(sheep::wander)
                    .with_system(sheep::wobble_sheep)
                    .with_system(sheep::update_sheep_ordering)
                    .with_system(check_end_battle),
            )
            .add_system_set(
                SystemSet::on_update(GameState::Battle)
                    .after("update")
                    .with_system(bounds_check),
            )
            .add_system_set(
                SystemSet::on_exit(GameState::Battle).with_system(despawn_war_machines),
            );
    }
}

fn init_level(mut commands: Commands, level: Res<Level>, asset_server: Res<AssetServer>) {
    match level.0 {
        1 => setup_level1(&mut commands, &asset_server),
        2 => setup_level2(&mut commands, &asset_server),
        _ => panic!("This level does not exists!"),
    }
}

fn move_and_attack(
    mut sheep_q: Query<(&mut Health, &mut Transform), (With<sheep::Sheep>, Without<WarMachine>)>,
    mut war_machines_q: Query<
        (
            &Speed,
            &mut Transform,
            &SpottingRange,
            &AttackRange,
            &AttackValue,
            &PursuitType,
        ),
        (With<WarMachine>, Without<sheep::Sheep>),
    >,
    time: Res<Time>,
) {
    for (speed, mut wm_transform, spotting_range, attack_range, attack_value, pursuit_type) in
        war_machines_q.iter_mut()
    {
        // Calculate the distance between the sheep and the current war machine
        let mut sheep = sheep_q
            .iter_mut()
            .filter(|(_, sheep_transform)| {
                wm_transform
                    .translation
                    .truncate()
                    .distance(sheep_transform.translation.truncate())
                    <= spotting_range.0
            })
            .collect::<Vec<_>>();

        sheep.sort_by(|(_, transform1), (_, transform2)| {
            wm_transform
                .translation
                .truncate()
                .distance(transform1.translation.truncate())
                .partial_cmp(
                    &wm_transform
                        .translation
                        .truncate()
                        .distance(transform2.translation.truncate()),
                )
                .unwrap()
        });

        // Find the closest sheep
        if let Some((ref mut sheep_health, sheep_transform)) = sheep.get_mut(0) {
            let difference =
                sheep_transform.translation.truncate() - wm_transform.translation.truncate();

            // If the sheep is close enough, attack it
            if difference.length() <= attack_range.0 {
                sheep_health.0 -= attack_value.0;
            }

            // Move towards the sheep depending on the `pursuit_type`
            match pursuit_type {
                PursuitType::ChasingClosest => {
                    let direction = difference.normalize_or_zero();
                    wm_transform.translation +=
                        direction.extend(0.0) * speed.0 * time.delta_seconds();
                }
            }
        }
    }
}

fn remove_dead_sheep(
    mut commands: Commands,
    sheep_q: Query<(Entity, &mut Health), (With<sheep::Sheep>, Changed<Health>)>,
) {
    for (sheep, health) in sheep_q.iter() {
        if health.0 <= 0.0 {
            commands.entity(sheep).despawn_recursive();
        }
    }
}

fn check_end_battle(
    sheep_q: Query<Entity, (With<sheep::Sheep>, Without<WarMachine>)>,
    war_machines_q: Query<Entity, (Without<sheep::Sheep>, With<WarMachine>)>,
    mut game_state: ResMut<State<GameState>>,
    mut level: ResMut<Level>
) {
    if sheep_q.is_empty() || war_machines_q.is_empty() {
        // TODO: should show battle report, before going straight to Herding
        // Should also add a timer to avoid long drawn battles
        game_state.set(GameState::Herding).unwrap();

        // Increase level if all war machines are dead
        if war_machines_q.is_empty() {
            level.0 += 1;
        }
    }
}

fn despawn_war_machines(mut commands: Commands, war_machines_q: Query<Entity, With<WarMachine>>) {
    for e in war_machines_q.iter() {
        commands.entity(e).despawn_recursive();
    }
}

fn setup_level1(commands: &mut Commands, asset_server: &Res<AssetServer>) {
    // Spawn red battlefield to distinguish from the pen
    // TODO: should be replaced with a proper asset
    commands
        .spawn_bundle(SpriteBundle {
            texture: asset_server.load("SheepFarmBehind.png"),
            sprite: Sprite {
                color: Color::ORANGE_RED,
                custom_size: Some(Vec2::splat(260.0 / 16.0)),
                ..default()
            },
            transform: Transform {
                translation: Vec3::splat(0.0),
                ..default()
            },
            ..default()
        })
        .insert(Name::from("Battlefield"));

    // Spawn a single war machine
    let mut rng = thread_rng();
    let transform = Transform::from_translation(Vec3::new(
        rng.gen_range(BATTLEFIELD_BOUNDS_X.x..=BATTLEFIELD_BOUNDS_X.y),
        rng.gen_range(BATTLEFIELD_BOUNDS_Y.x..=BATTLEFIELD_BOUNDS_Y.y),
        10.0,
    ));

    let war_machine = new_war_machine(commands, &asset_server, transform);
    commands
        .entity(war_machine)
        .insert(Speed(6.0))
        .insert(Health(10.0))
        .insert(AttackValue(1.0))
        .insert(AttackRange(1.0))
        .insert(SpottingRange(1000.0))
        .insert(PursuitType::ChasingClosest);
}

fn setup_level2(commands: &mut Commands, asset_server: &Res<AssetServer>) {}

fn new_war_machine(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    transform: Transform,
) -> Entity {
    let mut transform = transform;
    transform.rotation = Quat::IDENTITY;
    commands
        .spawn_bundle(SpriteBundle {
            transform,
            texture: asset_server.load("BaseSheep.png"),
            sprite: Sprite {
                color: Color::BLACK,
                custom_size: Some(Vec2::splat(1.0)),
                ..default()
            },
            ..default()
        })
        .insert(WarMachine)
        .insert(Bounds {
            x: (BATTLEFIELD_BOUNDS_X.x, BATTLEFIELD_BOUNDS_X.y),
            y: (BATTLEFIELD_BOUNDS_Y.x, BATTLEFIELD_BOUNDS_Y.y),
        })
        .id()
}
