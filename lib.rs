// The library both programs share: the game (src/main.rs) and the server
// (src/bin/server.rs). Every module lives here so either one can use it,
// which is what lets the game and the server run the same rules.

pub mod animation;
pub mod camera;
pub mod combat;
pub mod common;
pub mod credits;
pub mod game;
pub mod game_over;
pub mod health_visuals;
pub mod level_select;
pub mod loading;
pub mod main_menu;
pub mod network;
pub mod perlin_noise;
pub mod perlin_noise_test;
pub mod player;
pub mod procedural_gen_menu;
pub mod procedural_generation;
pub mod procedural_test;
pub mod protocol;
pub mod recursive_generation;
pub mod recursive_test;
pub mod settings;
pub mod simplerng;
pub mod test;
pub mod weapon;