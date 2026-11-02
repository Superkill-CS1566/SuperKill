# Fixed Platform Test

This is the first platform-generation prototype. It has a flat ground surface
and three fixed, solid platforms, with heights of 50, 65, and 80 world units.
Positions and widths are fixed; random generation is a later step.

## Run

```powershell
cargo run --locked --bin platform_test
```

Player 1 uses A/D to move and W to jump. Player 2 uses Left/Right to move and Up
to jump. S/Down crouches. R resets both players, F1 toggles player collision
outlines, and Escape closes this test window.

Combat and networking are not started in this scene. The scene directly reuses
the existing PlayerPlugin, AnimationPlugin, Platform component, gravity, and
collision systems. Existing gameplay source files are unchanged.

## Verify Both Approaches

```powershell
cargo run --locked --bin platform_test -- --verify
```

The verification mode drives normal player input through the existing gameplay
systems at a fixed simulation timestep. It attempts each platform once from
the left and once from the right, using the left and right player respectively.
A pass requires the player to land with their full collision width on the
platform. A failed attempt times out and returns a nonzero exit code.

Successful output ends with:

```text
PASS: all three fixed platforms are reachable from both sides (6/6).
```

This verifies the six fixed jumps. It does not yet validate random layouts,
connections between elevated platforms, combat movement, or online play.
