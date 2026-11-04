//! Melee: one fighter's attack meeting the other fighter.

use super::fighter::{AttackType, FighterState, PlayerSide, HITSTUN_TICKS};

/// A fighter going down this tick. Kills from melee, swords, and the crush edge
/// are all collected first and applied together, so two fighters killing each
/// other on the same tick is seen as a trade rather than whoever was checked
/// first winning.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Kill {
    pub victim: PlayerSide,
    pub killer: PlayerSide,
    /// Where the killer — or their sword — was, for the respawn distance check.
    pub killer_x: f32,
}

#[derive(Clone, Copy)]
enum Outcome {
    Miss,
    Blocked,
    Hit(u8),
}

/// Resolve each fighter's attack against the other.
pub fn resolve_melee(fighters: &mut [FighterState; 2], kills: &mut Vec<Kill>) {
    // Both directions are judged on the state from before either hit lands —
    // that's what makes a simultaneous exchange damage both fighters.
    let outcomes = [
        outcome(&fighters[0], &fighters[1]),
        outcome(&fighters[1], &fighters[0]),
    ];

    for attacker_index in 0..2 {
        let defender_index = 1 - attacker_index;

        match outcomes[attacker_index] {
            Outcome::Miss => {}

            // connected with a block, so it can't hit anything else this swing
            Outcome::Blocked => {
                fighters[attacker_index].attack_has_hit = true;
            }

            Outcome::Hit(damage) => {
                fighters[attacker_index].attack_has_hit = true;
                let killer = fighters[attacker_index].side;
                let killer_x = fighters[attacker_index].x;

                let defender = &mut fighters[defender_index];
                defender.health = defender.health.saturating_sub(damage);
                defender.hitstun = HITSTUN_TICKS;
                defender.vx = 0.0;

                if defender.health == 0 {
                    kills.push(Kill {
                        victim: defender.side,
                        killer,
                        killer_x,
                    });
                }
            }
        }
    }
}

fn outcome(attacker: &FighterState, defender: &FighterState) -> Outcome {
    if attacker.is_dead() || defender.is_dead() || attacker.attack_has_hit {
        return Outcome::Miss;
    }
    let (Some(attack), Some(hitbox)) = (attacker.attack, attacker.attack_hitbox()) else {
        return Outcome::Miss;
    };

    if !hitbox.overlaps(&defender.body()) {
        return Outcome::Miss;
    }

    // standing swing passes over a crouching player
    if attack == AttackType::Standing && defender.crouching {
        return Outcome::Miss;
    }

    // A block stops anything coming at the side the defender is facing, and
    // nothing from behind.
    let from_front = (attacker.x - defender.x) * defender.facing > 0.0;
    if defender.blocking && from_front {
        return Outcome::Blocked;
    }

    Outcome::Hit(attack.spec().damage)
}