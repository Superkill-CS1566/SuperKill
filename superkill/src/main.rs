use bevy::{prelude::*, window::PresentMode};

#[derive(Component, Deref, DerefMut)]
struct PopupTimer(Timer);

#[derive(Component)]
struct PopupImages {
    images: Vec<Handle<Image>>,
    current: usize,
}

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
        .add_systems(Startup, setup)
        .add_systems(Update, switch_image)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d);

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
        Sprite::from_image(asset_server.load("r")),
        Transform {
            translation: Vec3::new(0.,0.,0.),
            ..default()
        },
        PopupTimer(Timer::from_seconds(2., TimerMode::Repeating)),
        PopupImages {
            images,
            current: 0,
        }
    ));
}

fn switch_image(time: Res<Time>, mut popup: Query<(&mut PopupTimer, &mut Sprite, &mut PopupImages)>) {
    for(mut timer, mut sprite, mut images) in popup.iter_mut() {
        timer.tick(time.delta());
        if timer.just_finished() {
            // update our current position in images
            images.current = (images.current + 1) % images.images.len();
            // swap out image for new image
            sprite.image = images.images[images.current].clone();
        }
    }
}
