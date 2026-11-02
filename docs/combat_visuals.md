# Health visuals and camera smoothing

These changes use the existing Bevy dependency and run in Play Game. They do
not change server code, damage, health values, or combat rules.

## Health visuals

- A small overhead bar follows each living fighter, including while crouching.
- At half health or below, the bar turns red and the fighter has a gentle red
  pulse. With the current two-hit health system, this means one health remains.
- Death hides the bar. Respawning restores the full green bar and normal tint.
- Bars are removed when their player disappears or the match ends.

## Camera

- Camera following uses frame-rate-independent exponential smoothing.
- Death, respawn, and priority changes ease toward the new framing instead of
  instantly jumping there.
- Starting a match and changing screens snap to the correct position. The
  camera stays inside the existing level bounds and does not ease during pause.

## Manual checks

1. Run `cargo run --locked --bin superkill`, then select Play Game.
2. Land one standing attack: Player 1 uses G, Player 2 uses H. The injured
   fighter should have a half-full red bar and a subtle red pulse.
3. In a debug build, 3 toggles Player 1 between full and low health; 4 does the
   same for Player 2. These preview keys are not included in release builds.
4. Use the existing 1 / 2 debug death keys to kill Player 1 / Player 2. Watch
   the camera ease toward the survivor, then ease back when the opponent returns.
5. Move through a screen boundary. The new screen should appear immediately,
   without panning across the old screen. Pause and resume with Escape.
6. Quit to the menu and start again. Bars should not duplicate, and tint should
   reset. Test Game > Platform Test remains a separate physics-only scene.

## Automated checks

`cargo test --locked --bin superkill` covers health thresholds, damage/death/
respawn visuals, orphan cleanup, and smoothing at different frame rates.
