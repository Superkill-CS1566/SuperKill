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

The game will use a client-server architecture over UDP, with the server acting as the authoritative source of truth for gameplay. 

* Tokio + UDP will be used to handle communication between the server and two Bevy clients. 
	* Clients will connect to the server, receive player IDs, and send player inputs such as movement, attacks, blocking, and jumping. 
	* Serde and Postcard will be used to serialize game messages into compact packets that can be sent over UDP. 
* The server will control the official game state. 
	* Player position, velocity, health, attacks, blocking, deaths, respawns, screen transitions, and win conditions will all be processed by the server. 
	* Clients will send inputs rather than directly telling the server their positions or combat results. 
* Once basic multiplayer is working, latency compensation techniques will be added. 
	* Client-side prediction will make the local player's movement feel responsive. 
	* Interpolation will smooth the movement of the remote player between server updates. 
	* Server-side rewind will use recent player and hitbox history to compensate for latency when determining attacks, blocks, and parries.

### Procedural Generation

Levels are built from preset assets (NPCs, weapons, obstacles). There are a few different in-between screen types. Random generation should still let players reach either end, and objects should not spawn in bad spots.

* A Recursive Divsion maze generation algorithm will be used to construct most standard bar/outside levels of the game. 
    * Outside levels will treat the maze like an implied path in which platforms/barriers will stand in as maze walls, while bar levels will simply treat the maze as the structure of the building. 
* The final auto-scrolling level will use procedural terrain generation for construction (via Perlin Noise).
    * Terrain generation must include pits and cliffs to provide extra challenges for players.
* After the level generation has completed, the game will then place any designated objects throughout the level.
	* Levels will have a list of objects which it must place in a given level (i.e. Level 2 has 2 weapons, 1 NPC, 1 Obstacle)
	* Randomized generation for world objects which will account for entity collision and NPC paths. In the case in which objects collide or NPC paths are impeded, generation will continue.


## Midterm Goals

* Successfully implement the core player mechanics to a playable level. 
	* Horizontal/vertical movement. 
	* At least 3 different attacks. 
	* Blocking and basic combat interactions. 
	* Weapon throwing and pickup. 

* Have the basic map and screen progression system working. 
	* One static starting screen. 
	* End screens for both players. 
	* Players can move between screens without procedural generation. 

* Complete the foundation of the multiplayer system. 
	* Run a Tokio UDP server. 
	* Allow two Bevy clients to connect and receive separate player IDs. 
	* Send structured messages using Serde and Postcard. 
	* Synchronize basic player movement so each client can see the other player moving. 

* Begin implementation of both advanced topics. 
	* Networking should have the client-server architecture established and basic synchronization working. 
	* Procedural generation should have the level-generation algorithms designed and initial generation tests underway.


## Final Goals

* 40%: Online multiplayer works in a real match (move, fight, and go to the next screen together).
	* Two players can connect and play through a full match. 
	* The server is authoritative over movement, combat, health, deaths, respawns, and screen progression. 
	* Attacks, blocking, weapon interactions, and player state remain synchronized between both clients. 
	* Implement client-side prediction and interpolation to improve multiplayer responsiveness. 
	* Implement server-side rewind for latency compensation during attacks and parries.

* 35%: Procedural generation for in-between screens.
	* Generate playable in-between screens using the DFS-based maze generation system. 
	* Procedurally place obstacles, weapons, NPCs, and other level objects without blocking required player paths. 
	* Ensure generated levels remain traversable in both directions.

* 25%: The game is playable from start to end with no major issues.
	* Players can start a match, fight through multiple screens, and reach their respective final end screens. 
	* Core mechanics, multiplayer, screen progression, and procedural generation work together without major gameplay-breaking issues.

## Stretch Goals

* An auto-scrolling level with procedurally generated terrain.
* 2v2 multiplayer, where teams work together to reach the end of the map.
