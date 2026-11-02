// Reuse the gameplay modules without starting combat, networking, or terrain generation.
#![allow(dead_code)]

#[path = "../animation.rs"]
mod animation;
#[path = "../camera.rs"]
mod camera;
#[path = "../combat.rs"]
mod combat;
#[path = "../common.rs"]
mod common;
#[path = "../game.rs"]
mod game;
#[path = "../loading.rs"]
mod loading;
#[path = "../perlin_noise.rs"]
mod perlin_noise;
#[path = "../player.rs"]
mod player;
#[path = "../simplerng.rs"]
mod simplerng;
#[path = "../weapon.rs"]
mod weapon;

use bevy::{input::InputSystems, prelude::*, time::TimeUpdateStrategy};
use std::time::Duration;

use animation::AnimationPlugin;
use common::{AppState, GROUND_THICKNESS, GROUND_Y, LEVEL_LEN, Platform};
use game::GameState;
use loading::LoadingAssets;
use player::{Controls, PLAYER_SPAWN_Y, Player, PlayerPlugin, PlayerSide};
use weapon::ShowHitboxes;

const GROUND_TOP: f32 = GROUND_Y + GROUND_THICKNESS / 2.0;
const FIXED_PLATFORMS: [(f32, f32, f32); 3] = [
    (-360.0, 160.0, 50.0),
    (0.0, 200.0, 65.0),
    (360.0, 180.0, 80.0),
];

#[derive(Resource, Default)]
struct JumpVerification {
    enabled: bool,
    case: usize,
    started: bool,
    elapsed: f32,
}

fn main() -> AppExit {
    let verify = std::env::args().any(|arg| arg == "--verify");
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "SuperKill - Fixed Platform Test".into(),
            resolution: (1280, 720).into(),
            resizable: false,
            ..default()
        }),
        ..default()
    }))
    .insert_resource(ClearColor(Color::srgb(0.08, 0.09, 0.11)))
    .insert_resource(LoadingAssets(Vec::new()))
    .insert_resource(ShowHitboxes(true))
    .insert_resource(JumpVerification {
        enabled: verify,
        ..default()
    })
    .init_state::<AppState>()
    .init_state::<GameState>()
    .add_plugins((PlayerPlugin, AnimationPlugin))
    .add_systems(Startup, begin_platform_test)
    .add_systems(OnEnter(AppState::InGame), setup_platform_test)
    .add_systems(Update, test_controls)
    .add_systems(PreUpdate, drive_verification.after(InputSystems))
    .add_systems(PostUpdate, check_landing);

    if verify {
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
            1.0 / 120.0,
        )));
    }

    app.run()
}

fn begin_platform_test(mut state: ResMut<NextState<AppState>>) {
    state.set(AppState::InGame);
}

fn setup_platform_test(mut commands: Commands) {
    commands.spawn(Camera2d);
    commands.spawn((
        Text::new("Fixed Platform Test"),
        TextFont {
            font_size: FontSize::Px(28.0),
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(20.0),
            top: Val::Px(16.0),
            ..default()
        },
    ));

    spawn_surface(
        &mut commands,
        0.0,
        GROUND_Y,
        LEVEL_LEN,
        GROUND_THICKNESS,
        Color::srgb(0.27, 0.29, 0.31),
    );

    let colors = [
        Color::srgb(0.2, 0.65, 0.55),
        Color::srgb(0.8, 0.55, 0.25),
        Color::srgb(0.28, 0.5, 0.8),
    ];
    for (index, &(x, width, height)) in FIXED_PLATFORMS.iter().enumerate() {
        spawn_surface(
            &mut commands,
            x,
            GROUND_TOP + height / 2.0,
            width,
            height,
            colors[index],
        );
        commands.spawn((
            Text2d::new(format!("{} / {}", index + 1, height as u32)),
            TextFont {
                font_size: FontSize::Px(18.0),
                ..default()
            },
            TextColor(Color::WHITE),
            Transform::from_xyz(x, GROUND_TOP + height + 130.0, 2.0),
        ));
    }
}

fn spawn_surface(commands: &mut Commands, x: f32, y: f32, width: f32, height: f32, color: Color) {
    commands.spawn((
        Sprite::from_color(color, Vec2::new(width, height)),
        Transform::from_xyz(x, y, 0.0),
        Platform { width, height },
    ));
}

fn test_controls(
    keys: Res<ButtonInput<KeyCode>>,
    verification: Res<JumpVerification>,
    mut hitboxes: ResMut<ShowHitboxes>,
    mut players: Query<(&mut Player, &mut Transform, &PlayerSide)>,
    mut exit: MessageWriter<AppExit>,
) {
    if verification.enabled {
        return;
    }
    if keys.just_pressed(KeyCode::Escape) {
        exit.write(AppExit::Success);
    }
    if keys.just_pressed(KeyCode::F1) {
        hitboxes.0 = !hitboxes.0;
    }
    if keys.just_pressed(KeyCode::KeyR) {
        for (mut player, mut transform, side) in &mut players {
            transform.translation.x = side.spawn_x();
            transform.translation.y = PLAYER_SPAWN_Y;
            player.velocity = Vec2::ZERO;
            player.is_grounded = false;
            player.is_crouching = false;
            player.prev_crouching = false;
        }
    }
}

// Inject normal player controls, then let PlayerPlugin run its unchanged physics.
fn drive_verification(
    mut verification: ResMut<JumpVerification>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut players: Query<(&mut Player, &mut Transform, &PlayerSide, &Controls)>,
    time: Res<Time>,
    mut exit: MessageWriter<AppExit>,
) {
    if !verification.enabled || players.iter().count() != 2 {
        return;
    }
    keys.reset_all();
    if verification.case == FIXED_PLATFORMS.len() * 2 {
        println!("PASS: all three fixed platforms are reachable from both sides (6/6).");
        exit.write(AppExit::Success);
        return;
    }

    let (x, width, _) = FIXED_PLATFORMS[verification.case / 2];
    let from_left = verification.case % 2 == 0;
    let tested_side = if from_left {
        PlayerSide::Left
    } else {
        PlayerSide::Right
    };
    let direction = if from_left { 1.0 } else { -1.0 };

    for (mut player, mut transform, side, controls) in &mut players {
        if !verification.started {
            transform.translation.x = if *side == tested_side {
                x - direction * (width / 2.0 + 100.0)
            } else {
                side.spawn_x().signum() * 850.0
            };
            transform.translation.y = PLAYER_SPAWN_Y;
            player.velocity = Vec2::ZERO;
            player.is_grounded = true;
            player.is_crouching = false;
            player.prev_crouching = false;
        }
        if *side == tested_side {
            keys.press(if from_left {
                controls.right
            } else {
                controls.left
            });
            if !verification.started {
                keys.press(controls.jump);
            }
        }
    }
    verification.started = true;
    verification.elapsed += time.delta_secs();
    if verification.elapsed > 2.0 {
        eprintln!(
            "FAIL: platform {} from {} did not produce a stable landing.",
            verification.case / 2 + 1,
            if from_left { "left" } else { "right" },
        );
        exit.write(AppExit::error());
    }
}

fn check_landing(
    mut verification: ResMut<JumpVerification>,
    players: Query<(&Player, &Transform, &PlayerSide)>,
) {
    if !verification.enabled
        || !verification.started
        || verification.case >= FIXED_PLATFORMS.len() * 2
    {
        return;
    }
    let (x, width, height) = FIXED_PLATFORMS[verification.case / 2];
    let from_left = verification.case % 2 == 0;
    let tested_side = if from_left {
        PlayerSide::Left
    } else {
        PlayerSide::Right
    };
    for (player, transform, side) in &players {
        if *side != tested_side {
            continue;
        }
        let feet = transform.translation.y - player.height / 2.0;
        let fully_on_platform =
            (transform.translation.x - x).abs() + player.width / 2.0 < width / 2.0 - 1.0;
        if player.is_grounded && fully_on_platform && (feet - (GROUND_TOP + height)).abs() < 0.5 {
            println!(
                "PASS: platform {} (height {}) from {}.",
                verification.case / 2 + 1,
                height,
                if from_left { "left" } else { "right" },
            );
            verification.case += 1;
            verification.started = false;
            verification.elapsed = 0.0;
        }
    }
}
