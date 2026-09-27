use bevy::{prelude::*};

use crate::common::{AppState, spawn_ui_camera, despawn_ui_camera};

#[derive(Component)]
struct LoadingProgressFrame;

#[derive(Component)]
struct LoadingProgress;

#[derive(Resource, Deref, DerefMut)]
pub struct LoadingAssets(pub Vec<UntypedHandle>);

#[derive(Resource, Deref, DerefMut)]
pub struct TimedLoad(Timer);

const PROGRESS_LENGTH: f32 = 400.;   // bar width in pixels
const PROGRESS_HEIGHT: f32 = 20.;    // bar height
const PROGRESS_FRAME: f32 = 10.;     // border thickness around the bar
const MIN_LOAD_TIME: f32 = 5.;

pub struct LoadingPlugin;
impl Plugin for LoadingPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(LoadingAssets(Vec::new()))
            .add_systems(OnEnter(AppState::Loading), (setup_loading, spawn_ui_camera))
            .add_systems(Update, update_loading.run_if(in_state(AppState::Loading)))
            .add_systems(Update, load_timer.run_if(resource_exists::<TimedLoad>))
            .add_systems(
                OnExit(AppState::Loading),
                (
                    despawn_with::<LoadingProgressFrame>,
                    despawn_with::<LoadingProgress>,
                    free_loading_handles,
                    despawn_ui_camera,
                ),
            );
    }
}

fn setup_loading(mut commands: Commands) {
    commands.spawn((
        Sprite::from_color(Color::BLACK, Vec2::ONE),
        Transform {
            scale: Vec3::new(
                PROGRESS_LENGTH + PROGRESS_FRAME,
                PROGRESS_HEIGHT + PROGRESS_FRAME,
                0.,
            ),
            ..default()
        },
        LoadingProgressFrame,
    ));

    commands.spawn((
        Sprite::from_color(Color::WHITE, Vec2::ONE),
        Transform {
            scale: Vec3::new(0., PROGRESS_HEIGHT, 0.),
            ..default()
        },
        LoadingProgress,
    ));

    commands.insert_resource(TimedLoad(Timer::from_seconds(
        MIN_LOAD_TIME,
        TimerMode::Once,
    )));
    info!("Loading: Fake timed asset");
}

fn update_loading(
    asset_server: Res<AssetServer>,
    loading_assets: ResMut<LoadingAssets>,
    mut progress_transform: Single<&mut Transform, With<LoadingProgress>>,
    mut next_state: ResMut<NextState<AppState>>,
    timed_load: Res<TimedLoad>,
) {
    let loaded: usize = loading_assets
        .iter()
        .map(|a| {
            if asset_server.load_state(a).is_loaded() {
                1
            } else {
                0
            }
        })
        .sum::<usize>()
        + if timed_load.is_finished() { 1 } else { 0 };
    // account for fake TimedLoad "Asset"
    let total = loading_assets.len() + 1;
    let percent = (loaded as f32) / (total as f32);

    progress_transform.scale.x = PROGRESS_LENGTH * percent;

    // Check if all assets are loaded
    if loaded == total {
        next_state.set(AppState::Play);
    }
}

fn load_timer(time: Res<Time>, mut timed_load: ResMut<TimedLoad>) {
    timed_load.tick(time.delta());
    if timed_load.just_finished() {
        info!("Loaded: Fake timed asset");
    }
}

fn free_loading_handles(mut loading_assets: ResMut<LoadingAssets>) {
    loading_assets.clear();
}

pub fn despawn_with<T: Component>(mut commands: Commands, q: Query<Entity, With<T>>) {
    for e in q.iter() {
        commands.entity(e).despawn();
    }
}