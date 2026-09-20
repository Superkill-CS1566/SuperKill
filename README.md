# SuperKill

by Team 1

A [Nidhogg](https://en.wikipedia.org/wiki/Nidhogg_(video_game))-inspired two-player side-scrolling swordfighting game. Two fighters start on opposite sides of a shared map and race to the other player's end of the world — by cutting them down, or by slipping past them.

Built in [Rust](https://www.rust-lang.org/) with [Bevy](https://bevyengine.org/) 0.19.

## Team Members
* Advanced Topic Subteam 1: NETWORKING
	* ericliu2006: Eric Liu
 	* leun-se: Brian Lee
  	* hmyld: Mengzi Chen

* Advanced Topic Subteam 2: PROCEDURAL GENERATION
	* raa193: Ryan Armendariz-Lopez
 	* abstractGuardian27: Astor Stave
  	* noahthebest112233: Noah

## Game Description

Niddhogg-inspired two-player sidescroller.

Both players start off on opposite ends of the screen, and their goal is to try to get to the opposite end by any means possible. This can be done by either defeating the opposing player or dodging and running past them. As long as either player is able to reach the end of the screen, both players are teleported to the next screen. For each player, the end goal is to reach the end screen, and both of their end screens are on opposite sides of the map.


## Advanced Topic Description

### Networking

Use a client-server setup. The server keeps the game state (player/weapon position, health, attacks, blocking). Players send actions to the server, and the server sends the updated state back to both players. When someone reaches the other end of the screen, the server moves both players to the next screen.

### Procedural Generation

Levels are built from preset assets (NPCs, weapons, obstacles). There are a few different in-between screen types. Random generation should still let players reach either end, and objects should not spawn in bad spots.

## Midterm Goals

* Successfully implement the player movement and basic mechanics to a playable level: 3 different attacks, weapon throwing/pickup, and horizontal/vertical movement.
* Have a basic map and screen system completed. Should be playable (pre-procedural gen). 1 static starting screen + end screens.
* Be able to have two players connect to a server and move around.
* Have some basic plan and direction for how online multiplayer and procedural generation is going to be implemented in the game. Start work on both. 


## Final Goals

* 40%: Online multiplayer works in a real match (move, fight, and go to the next screen together).
* 35%: Procedural generation for in-between screens.
* 25%: The game is playable from start to end with no major issues.

## Stretch Goals

* An auto-scrolling level with procedurally generated terrain.
* 2v2 multiplayer, where teams work together to reach the end of the map.
