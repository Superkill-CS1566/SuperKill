use bevy::prelude::*;

use crate::common::{despawn_ui_camera, spawn_ui_camera, AppState};

pub struct CreditsPlugin;

impl Plugin for CreditsPlugin{
    fn build(&self, app: &mut App) {
        // credits sys
        app.add_systems(OnEnter(AppState::Credits), (spawn_ui_camera,setup_credits))
        .add_systems(Update, switch_image.run_if(in_state(AppState::Credits)))
        .add_systems(OnExit(AppState::Credits), (despawn_ui_camera, despawn_credits));
    }
}

#[derive(Component, Deref, DerefMut)]
struct PopupTimer(Timer);

#[derive(Component)]
struct PopupImages {
    images: Vec<Handle<Image>>,
    current: usize,
}

// Fade in and Fade out For pics in Credits
#[derive(Component)]
enum FadeState {
    FadeOut,
    Switch,
    FadeIn,
    Stay,
}

#[derive(Component)]
struct FadeProgress(f32);

// setup credits
// removed camera spawning from fn
fn setup_credits(mut commands: Commands, asset_server: Res<AssetServer>) {
    // handy dandy list of images
    let images = vec![
        asset_server.load("AstorStave.png"),
        asset_server.load("BrianLee.png"),
        asset_server.load("EricLiu.png"),
        asset_server.load("MengziChen.png"),
        asset_server.load("NuoyaLiu.png"),
        asset_server.load("RyanArmendarizLopez.png"),
        asset_server.load("Team1.png")
    ];
    
    commands.spawn((
        Sprite {
            image: images[0].clone(),
            color: Color::srgba(1.0,1.0,1.0,0.0),
            ..default()
        },
        Transform {
            translation: Vec3::new(0.,0.,0.),
            ..default()
        },
        PopupTimer(Timer::from_seconds(2., TimerMode::Once)),
        PopupImages {
            images,
            current: 0,
        },
        FadeState::FadeIn,
        FadeProgress(0.0)
    ));
}

// exit credit and delete camera
// removed camera despawning from fn 
fn despawn_credits(
    mut commands: Commands,
    query_pic: Query<Entity, With<PopupImages>>,
) {
    for entity in &query_pic {
        commands.entity(entity).despawn();
    }
}

fn switch_image(
    time: Res<Time>,
    mut next_state: ResMut<NextState<AppState>>,
    mut popup: Query<(&mut PopupTimer, &mut Sprite, &mut PopupImages, &mut FadeState, &mut FadeProgress)>,
) {
    let fade_duration = 0.5; // Time of pictures out or in

    for (mut timer, mut sprite, mut images, mut fade_state, mut progress) in &mut popup {
        match *fade_state {
            FadeState::FadeIn => {
                progress.0 += time.delta().as_secs_f32() / fade_duration;
                let alpha = progress.0.clamp(0.0,1.0);
                sprite.color = Color::srgba(1.0,1.0,1.0, alpha);

                if progress.0 >= 1.0 {
                    *fade_state = FadeState::Stay;
                    timer.reset();
                }
            }

            FadeState::Stay => {
                timer.tick(time.delta());
                if timer.just_finished() {
                    *fade_state = FadeState::FadeOut;
                }
            }

            FadeState::FadeOut => {
                progress.0 -= time.delta().as_secs_f32() / fade_duration;
                let alpha = progress.0.clamp(0.0,1.0);
                sprite.color = Color::srgba(1.0,1.0,1.0, alpha);

                if progress.0 <= 0.0 {
                    *fade_state = FadeState::Switch;
                }
            }

            FadeState::Switch => {
                if images.current < images.images.len() - 1 {
                    images.current += 1;
                    sprite.image = images.images[images.current].clone();
                    *fade_state = FadeState::FadeIn;
                    progress.0 = 0.0;
                }else{
                    next_state.set(AppState::MainMenu);
                }
            }
        }
    }
}