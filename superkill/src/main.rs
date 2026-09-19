use bevy::{prelude::*, window::PresentMode};

#[derive(States, Clone, PartialEq, Eq, Debug, Hash, Default)]
enum AppState {
    #[default]
    MainMenu,  //主菜单
    Credits,  //鸣谢
}

// camera
#[derive(Component)]
struct UiCamera;

#[derive(Component)]
struct MainMenuRoot;

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

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "End Credits for SuperKill".into(),
                resolution: (1280, 720).into(),
                present_mode: PresentMode::AutoVsync,
                ..default()
            }),
            ..default()
        }))
        .init_state::<AppState>()
        // main menu sys
        .add_systems(OnEnter(AppState::MainMenu), spawn_main_menu)
        .add_systems(Update, button_interaction.run_if(in_state(AppState::MainMenu)))
        .add_systems(OnExit(AppState::MainMenu), despawn_main_menu)

        // credits sys
        .add_systems(OnEnter(AppState::Credits), setup_credits)
        .add_systems(Update, switch_image.run_if(in_state(AppState::Credits)))
        .add_systems(OnExit(AppState::Credits), despawn_credits)
        .run();
}

fn spawn_main_menu(mut commands: Commands) {
    // if print 1 it is ok
    println!("1");

    // 2D camera summon to activate UI，UiCamera
    commands.spawn((
        Camera2d,
        Transform::default(),
        GlobalTransform::default(),
        UiCamera,
    ));

    commands.spawn((
        // root Node
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        BackgroundColor(Color::srgb(0.15, 0.15, 0.2)),
        MainMenuRoot,
    ))
    .with_children(|parent| {
        parent.spawn((
            // button position
            Button,
            Node {
                width: Val::Px(300.0),
                height: Val::Px(90.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.3, 0.5, 0.95)),
            BorderColor::all(Color::srgb(0.5, 0.7, 1.0)),
        ))
        .with_children(|parent| {
            // button word
            parent.spawn((
                Text::new("Credits"),
                TextFont {
                    font_size: FontSize::Px(36.0),
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
    });
}

// handle interact with bottons
fn button_interaction(
    mut interaction_query: Query<&Interaction, (Changed<Interaction>, With<Button>)>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for interaction in &mut interaction_query {
        if let Interaction::Pressed = *interaction {
            next_state.set(AppState::Credits);
        }
    }
}

// exit main menu and delete camera
fn despawn_main_menu(
    mut commands: Commands,
    query_root: Query<Entity, With<MainMenuRoot>>,
    query_camera: Query<Entity, With<UiCamera>>,
) {
    for entity in &query_root {
        commands.entity(entity).despawn();
    }
    // delete camera
    for entity in &query_camera {
        commands.entity(entity).despawn();
    }
}

// setup credits
fn setup_credits(mut commands: Commands, asset_server: Res<AssetServer>) {

    // camera in credit，UiCamera
    commands.spawn((
        Camera2d,
        Transform::default(),
        GlobalTransform::default(),
        UiCamera,
    ));

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
fn despawn_credits(
    mut commands: Commands,
    query_pic: Query<Entity, With<PopupImages>>,
    query_camera: Query<Entity, With<UiCamera>>,
) {
    for entity in &query_pic {
        commands.entity(entity).despawn();
    }
    for entity in &query_camera {
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