# Game Description For A Friend

Source docs: [[cortext_command_vault/spec/authoritative-game-spec-v0|Game Spec v0]], [[cortext_command_vault/spec/prototype-roadmap|Prototype Roadmap]], [[cortext_command_vault/spec/command-core-base-power|Command Core And Base Power]], [[cortext_command_vault/spec/chassis-armor-mechs-and-origins|Chassis, Armor, Mechs, And Origins]], [[cortext_command_vault/decisions/dr-027-combat-base-scope|DR-027 Combat-Base Scope]], [[cortext_command_vault/decisions/dr-036-systemic-material-simulation-direction|DR-036 Systemic Materials]], [[cortext_command_vault/comparables/noita-grade-material-simulation-research|Noita-Grade Material Simulation Research]]

## The Short Version

Imagine a 2D tactical physics game where you are not just one soldier, and not just a floating RTS cursor. You are the continuity commander of a small mercenary, rescue, and salvage outfit on a collapsing sci-fi frontier. You can command your squad like a strategy game, let your AI soldiers carry out orders on their own, or instantly take direct control of a body, android, power armor suit, or mech when you want to personally solve the problem.

The battlefield is a side-view pixel simulation full of destructible terrain, fire, smoke, water, acid, toxic gas, lava, electricity, rubble, broken machines, panicking soldiers, damaged armor, and mechs that can lose limbs, weapons, sensors, shields, or pilots. The goal is to capture the wild "anything can happen" energy of Cortex Command and Noita, then make it clearer, more tactical, more moddable, and much better at solo play.

The dream is simple: every mission should create a story worth replaying.

"The medic tried to drag our engineer out, but the corridor flooded."

"The enemy commander noticed we always breach low, so next mission they put mines under the bunker."

"I uprooted the command core from the base, shoved it into a damaged mech, and won the mission, but the base shields went down and half the hangar burned."

That kind of game.

## The Player Fantasy

You run a small outfit taking dangerous contracts across failing colonies, corporate war zones, derelict megastructures, alien biomes, disaster sites, and black-site jobs.

Sometimes you are doing something heroic, like rescuing trapped colonists from a burning base.

Sometimes you are doing mercenary work, like breaching a corporate bunker before a rival salvage crew gets there.

Sometimes the mission goes sideways because a rocket opens the wrong wall, toxic gas spills into the tunnel, your best android loses a leg, and the command core is suddenly the only thing keeping the base powered.

You are not locked into one mode of play. You can:

- Play like an RTS commander and issue orders while AI teammates move, breach, cover, rescue, repair, retreat, and explain what they are doing.
- Possess a soldier, robot, android, powered armor suit, or mech for twitchy direct control.
- Switch between bodies when the situation demands it.
- Let the AI continue running bodies when you are not directly piloting them.
- Build and defend a base around a vulnerable command core.
- Uproot that core and put it into a body or mech for a huge power spike at a huge strategic cost.

The command core is the heart of the game. When rooted in your base, it powers shields, doors, repair platforms, sensors, turrets, charging pads, logistics beacons, and AI command relays. When uprooted, the base starts losing those advantages. But if you embed the core into a chassis, that unit becomes a terrifying avatar with more armor, energy, equipment output, abilities, and command range.

So you get a real choice:

Do you keep the core safe and hold the base?

Do you move it to escape?

Do you gamble everything by planting it into a mech and personally leading the assault?

## What A Mission Feels Like

A mission begins with a contract card. It tells you the job, terrain, faction, hazards, expected equipment needs, possible salvage, and why this place is about to become a disaster.

Maybe the job is a compact breach contract:

You land outside a fortified bunker. The target is deep inside. There are two known breach paths, one salvage cache, one trapped engineer, and a defender commander who will react once the shooting starts. The material profile says the upper wall is reinforced concrete, the lower tunnel has soft dirt, the storage room contains fuel, and there is a flooded maintenance shaft that might be useful or deadly.

Before deployment, you build a squad:

- A shield unit in heavy armor.
- An engineer with a digger and repair tool.
- A medic with smoke and rescue gear.
- A light mech with a mounted cannon and a damaged left leg you repaired from the last mission.
- Maybe an android scout because toxic gas does not bother it as much.

The loadout screen is not just a shop. It is a planning table. It warns you if no one can dig, if your AI does not know how to use a weapon well, if your mech is too heavy for a tunnel, if your base power is low, or if your delivery craft might crash under fire.

Then the mission starts.

Your squad deploys. You can directly control the shield unit and push forward, or stay in commander mode and tell the engineer to start a low breach. The AI reports intent:

"Moving to lower wall."

"Covering the door."

"Low ammo, falling back."

"Toxic gas detected. Avoiding lower tunnel."

The terrain is not just backdrop. It is the mission. Bullets chew through dirt. Explosions open shortcuts. Fire spreads through wood and oil. Water can extinguish flames, become steam near heat, carry electricity, or neutralize dangerous chemicals. Acid can eat through material. Lava is terrifying even in tiny amounts. Smoke blocks sight. Toxic gas can make organics choke. Debris can knock someone down. A kicked pebble can actually hurt if it hits right.

Everything physical is supposed to matter. Limbs, weapons, armor plates, dropped guns, shields, doors, mech legs, projectiles, rubble, and terrain all collide unless there is a clear tested reason they do not. A rifle can be hit. A weapon can jam. A mech foot can crush infantry. A bullet can ricochet off armor, punch into terrain, deflect off another projectile, or shatter into fragments.

The end of the mission is not just "win" or "lose." It is a debrief:

- Who survived.
- Who was rescued.
- What gear was lost or recovered.
- Which armor cracked first.
- Why the AI changed plans.
- What material reaction caused the disaster.
- Which enemy tactic surprised you.
- What to try next on the same seed.

The replay should be useful, not just cinematic. It should tell you why the run became a story.

## The World And Tone

The tone is tactical pulp sci-fi disaster sandbox.

That means the game is serious enough that named actors, damaged mechs, and bad calls matter, but pulpy enough that ridiculous chain reactions, strange alien hazards, desperate avatar plays, and improvised chaos are part of the charm.

It is not pure military realism. It is not pure comedy. It is not just a sandbox toy. It is a battlefield where systems collide and missions turn into little sci-fi disaster movies.

The world is a frontier mess:

- Colonies are failing.
- Corporations are fighting over assets.
- Alien biomes are eating infrastructure.
- Derelict megastructures hide valuable salvage.
- Rescue jobs turn into firefights.
- Black-site missions go wrong.
- Rival salvage crews show up at the worst possible time.

The player's outfit is flexible enough to support different factions and mods. One faction might be a disposable alien swarm. Another might be a sleek corporate android cleanup squad. Another might be scavengers with ugly improvised gear. Another might worship the command core as something holy or dangerous.

The game should have room for weirdness without losing tactical clarity.

## Bodies, Armor, Mechs, And Origins

Characters are not just hit points with legs.

The plan treats humans, androids, robots, powered armor, mechs, and stranger future origins as different physical bodies with different strengths, weaknesses, repairs, and risks.

Armor is layered. A helmet, chest plate, arm plate, shield emitter, jet module, sensor pod, weapon mount, cockpit, reactor, mech leg, or repair drone can be damaged separately. Damage happens in readable stages:

- Nominal.
- Degraded.
- Module warning.
- Module failed.
- Weapon jammed.
- Armor cracked.
- Disabled.
- Pilot injured.
- Eject.
- Wreck.
- Exploded.

That matters because the AI, HUD, replay, and modding tools all need to understand what failed. If a mech stops moving, the game should not just say "mech bad." It should show that the left leg hydraulic module failed after a collision, the pilot is still alive, the shield emitter is intact, and a repair drone might save it if someone can get there.

Mechs are a major part of the fantasy. The roadmap supports a full ladder:

- Powered armor.
- Light mechs.
- Medium mechs.
- Heavy mechs.

Mechs can have different roles:

- Armored breakthrough units.
- Shielded units with force fields or electric protection.
- Fire-support platforms.
- Rescue and repair chassis.
- Sensor and command relay units.
- Heavy industrial breachers.
- Hazard-sealed units for gas, acid, lava, or underwater work.

The important part is that mechs are not just bigger health bars. They are physical machines with weight, power, modules, repair needs, cockpit risks, and salvage value.

## The Base Is Part Of The Game

The base is not meant to become a full colony sim, but it should be much deeper than a menu.

The base is a combat structure built around the command core. It can have:

- Shields.
- Powered turrets.
- Sensors.
- Smart doors and locks.
- Repair platforms.
- Charging pads.
- Hangars.
- Storage.
- Traps.
- Power reserves.
- Command relays.
- Breachable structure.

The command core powers and coordinates these systems. If the core is rooted, the base is strong. If the core is uprooted, the base starts relying on reserve power or losing systems.

This creates a delicious strategic tension. The thing that makes your base strong can also become your strongest battlefield avatar. But you cannot have both at full strength at the same time.

## AI Is A Main Feature, Not Filler

One of the biggest goals is to make solo play great enough that you do not need other players to have fun.

Friendly AI should not feel like disposable bots. They should feel like teammates with jobs, doctrine, memory, and readable mistakes.

The AI should:

- Communicate intent like a teammate.
- Understand terrain, hazards, material danger, equipment roles, and rescue opportunities.
- Make plausible mistakes and recover.
- Have different personalities and doctrines.
- Learn from repeated mission outcomes.
- Explain why it refused an order or changed plans.
- Give the player enough trust to delegate.

The game also plans a hybrid LLM "mind" layer, but not for twitch reactions. Fast local AI handles aiming, dodging, shooting, pathing, fleeing, and emergency actions. Optional background AI models can think more slowly about doctrine, memory, personality, squad plans, enemy adaptation, dialogue, and debriefs. The local AI never waits for a cloud model. If the LLM is off, the game still works.

That makes the AI practical:

- Reflexes stay fast.
- Strategy can become more humanlike over time.
- Replays can show what the AI thought.
- Tests can run without API keys.
- Modders can create AI profiles and doctrines.

## Materials And Physics Are The Secret Sauce

The material target is basically "Noita-style systemic danger inside a tactical Cortex-like war game," but bounded so it can actually be built and understood.

The launch material plan includes things like:

- Air.
- Dirt and sand.
- Rock and concrete.
- Metal.
- Wood and organic material.
- Water.
- Steam and mist.
- Smoke.
- Fire and heat.
- Oil and fuel.
- Acid.
- Toxic sludge.
- Toxic gas.
- Lava.
- Blood and vomit.
- Electricity charge.
- Pebbles and debris.

The point is not just to simulate materials for bragging rights. The point is that materials become verbs:

- Water extinguishes fire.
- Heat turns water into steam.
- Oil burns.
- Wood burns.
- Acid can be countered.
- Gas needs ventilation.
- Electricity flows through conductive liquids and metals.
- Lava is a terrifying environmental weapon.
- Debris can hurt.
- Blood, vomit, and other mess can become actual world materials if the design supports it.

For bases and sealed vehicles, the game borrows from Barotrauma-style thinking: rooms, hulls, leaks, flooding, pressure, oxygen, pumps, vents, fire, and life support. A breached base should not just lose HP. It should flood, vent atmosphere, lose sensors, short out doors, burn, smoke, or force an evacuation.

The key design rule is: if something can kill you, the game should eventually let you inspect why.

## Stationeers-Style Systems, But In A Combat Game

For a Stationeers fan, the most important thing to understand is that the game is not only "soldiers in destructible terrain." The real hook is that the battlefield, the base, and the machines are connected systems.

Stationeers is satisfying because pipes, gases, pressure, power, batteries, sensors, and damage all matter. This game wants that same engineering-brain satisfaction, but compressed into a tactical 2D action sandbox where somebody is actively shooting holes in your setup.

The base idea:

You are not building a calm perfect station. You are trying to keep a combat base, squad, mech bay, life-support network, and command core alive while the mission is falling apart.

### Materials Are Verbs

The material system is designed so each material has behavior, not just color. A material can define how it moves, burns, conducts, flows, evaporates, corrodes, poisons, stains, blocks line of sight, damages armor, affects AI pathing, and appears in replays.

| Material / State | What It Can Do | Why It Matters |
|---|---|---|
| Water | Pools, flows, extinguishes fire, becomes steam near heat, conducts electricity, neutralizes some hazards, floods rooms. | Water becomes a tool, hazard, countermeasure, and engineering problem. |
| Steam / mist | Rises, fills spaces, marks heat transfer, can obscure sight or reveal a hot zone. | Temperature changes become visible. |
| Smoke | Spreads through rooms and vents, blocks sight, signals fire, affects organic breathing and AI confidence. | Fire is not just damage over time; it changes command and visibility. |
| Fire / heat | Burns wood and oil, heats rooms, creates smoke, damages actors and equipment, can trigger chain reactions. | Fire becomes area denial, base disaster, and tactical weapon. |
| Oil / fuel | Flows like liquid, coats surfaces, ignites, burns, and can turn a safe floor into a trap. | A fuel leak can change the whole mission. |
| Acid | Corrodes terrain, armor, equipment, and base modules; can be countered by water or neutralizers. | Forces loadout and route decisions. |
| Toxic sludge / liquid | Contaminates areas, can be washed or neutralized, can create gas or afflictions. | Makes cleanup and rescue tools matter. |
| Toxic gas | Fills rooms, vents through gaps, asphyxiates organics, is safer for androids/robots/sealed suits. | Origin choice and ventilation become tactical. |
| Lava | Burns, melts, ignites, reacts with water, and can be lethal in tiny amounts. | Small material accidents can become huge stories. |
| Metal | Strong, conductive, ricochet-prone, good for structures, dangerous around electricity. | Helps terrain, armor, power, and bullet behavior connect. |
| Wood / organic | Burns, breaks, supports, creates smoke, can become battlefield fuel. | Structures are not inert. |
| Blood / vomit | Can become world material, stain actors, contaminate surfaces, or feed future weird recipes. | Gross, funny, systemic, and very readable. |
| Pebbles / debris | Has mass, can be kicked, can damage, can jam, can block, can become shrapnel. | Tiny physical objects can matter. |
| Electricity charge | Travels through conductive materials, liquids, wet actors, metal structures, and powered systems. | Water, metal, batteries, and armor become linked hazards. |

The important rule is not "simulate everything perfectly." It is "make every important reaction learnable, inspectable, and useful." Rare alchemy-style secrets can stay mysterious, but core combat hazards should have overlays, captions, AI reason labels, and replay events.

### Liquids, Density, And Layering

Liquids should not all behave like the same blue goop. Different liquids can have different mass, viscosity, flow speed, conductivity, toxicity, flammability, and density.

That means:

- Water can settle under oil.
- Oil can float, spread, and ignite.
- Acid can flow into cracks and eat through weak material.
- Sludge can move slowly and contaminate routes.
- Lava can be heavy, hot, and destructive.
- Coolant or foam can suppress heat or seal gaps.

This creates real tactical engineering. If a corridor floods, you do not only ask "is there water?" You ask: what liquid, how deep, is it electrified, is it hot, is it toxic, can my android cross it, can my mech's exposed joint survive it, and can I pump it somewhere useful?

### Gases, Oxygen, Pressure, And Rooms

For sealed bases, bunkers, ships, underwater structures, and larger mechs, the game uses a room/atmosphere layer inspired by Barotrauma and Stationeers.

Rooms can have:

- Oxygen level.
- Pressure.
- Toxic gas concentration.
- Smoke concentration.
- Temperature.
- Water level.
- Breach/gap state.
- Vent and pump links.
- Door and seal state.
- Power and device condition.

A room can fail in multiple ways. It can flood. It can depressurize. It can fill with smoke. It can lose oxygen. It can get too hot. It can become electrified. It can become toxic. It can become isolated because a powered door jammed shut.

That means base damage gets interesting:

| Event | System Consequence | Player Decision |
|---|---|---|
| Wall breach | Pressure changes, gas escapes, water enters, smoke moves, AI updates route safety. | Seal it, pump it, avoid it, exploit it, or send sealed units. |
| Vent destroyed | Smoke/toxic gas stops clearing; oxygen distribution changes. | Repair vent, open door, reroute squad, or accept the hazard. |
| Pump loses power | Flooding accelerates or cannot be cleared. | Restore power, carry portable pump, or evacuate. |
| Fire in oxygen-rich room | Fire grows faster, smoke rises, heat damages equipment. | Vent room, flood room, cut oxygen, use foam, or abandon module. |
| Door jammed | Pressure, smoke, enemies, and rescue routes change. | Force door, cut wall, power-cycle, or find alternate route. |
| Toxic leak | Organics need masks/suits; androids and robots gain tactical value. | Change squad composition or route. |

Pressure should be approximate and readable rather than real-unit simulation. The goal is the Stationeers feeling of connected systems without requiring every player to do spreadsheet engineering mid-fight.

### Temperature And Heat As Tactical Information

Temperature connects the material system to machines, armor, rooms, and base modules.

Heat can:

- Turn water into steam.
- Help fire spread.
- Overheat weapons.
- Damage batteries.
- Stress shields.
- Make armor uncomfortable or dangerous for organic pilots.
- Force ventilation.
- Cook off explosives or fuel.
- Reduce sensor reliability.
- Create thermal signatures for AI and sensors.

Cold or coolant can:

- Slow fire.
- Stabilize overheated modules.
- Change gas/liquid states.
- Create brittle material behavior.
- Protect a reactor or battery bay.

The game should expose heat through HUD cues, captions, overlays, audio, and replay. A player should be able to understand "this room killed me because the fire heated the pressure chamber, the battery overheated, the shield emitter failed, and then the door jammed."

### Power, Batteries, And Base Networks

The base is not meant to be a full colony simulator, but it should have a real tactical power network.

Base power can come from:

- The rooted command core.
- Auxiliary generators.
- Reserve batteries.
- Portable power cells.
- Salvaged enemy modules.
- Emergency capacitors.

Power can feed:

- Shields.
- Turrets.
- Sensors.
- Doors.
- Pumps.
- Vents.
- Oxygen generators.
- Repair platforms.
- Charging pads.
- Hangars.
- Alarm systems.
- Command relays.

When things are healthy, the base feels alive. Shields hum. Sensors scan. Turrets coordinate. Doors seal. Pumps move water. Vents clear smoke. Repair pads fix armor and androids. Charging stations refill batteries and mech energy.

When things go wrong, the base starts shedding capability.

| Power Problem | What The Player Sees |
|---|---|
| Main power loss | Lights dim, shields drop, turrets sleep, vents stop, repair pads slow down. |
| Battery drain | Systems keep running for a while, then fail in priority order. |
| Overload | A shield, turret, or repair platform pulls too much power and trips the local circuit. |
| Damaged cable / relay | One wing of the base loses power while the core room still works. |
| Uprooted command core | Base drops to reserve power; command boosts, shields, sensors, and automation weaken. |
| Emergency capacitor discharge | Short burst of shields, door seal, turret volley, or repair pulse. |

This is where the Stationeers comparison is strongest: the base is a machine. You are not just decorating it. You are designing a tactical system that can fail, recover, and be exploited.

### Command Core Buffs And The Rooted/Uprooted Tradeoff

The command core is the heart of the base. When rooted, it acts like a power/control super-node.

Rooted command core can buff:

- Shield strength and recharge.
- Turret target sharing.
- Sensor range and confidence.
- Door lock reliability.
- Repair platform speed.
- Charging pad throughput.
- AI command radius.
- Squad response time.
- Tactical map clarity.
- Delivery beacon accuracy.

If the player uproots it, those buffs weaken or disappear. The base may still run on batteries or auxiliary generators, but it is no longer operating at peak.

Then the player can embed the command core into a body, powered armor suit, android shell, or mech. That avatar can gain:

- More armor.
- More health or integrity.
- Faster mobility.
- Better jump/jet recovery.
- Larger battery.
- Faster energy recharge.
- Stronger shields.
- Higher equipment power output.
- Special command abilities.
- A local control aura that improves nearby AI.

That creates one of the central game decisions:

Do you keep the command core rooted so the base stays strong, or do you pull it out and turn one unit into a terrifying battlefield avatar?

It is a power fantasy with a cost. If the avatar wins, it feels legendary. If it dies, the replay should make it clear exactly how the gamble failed.

### Armor, Equipment, And Base Damage Reduces Performance

Damage should not only remove HP. It should reduce function.

For actors and mechs:

| Damaged Part | Reduced Performance |
|---|---|
| Leg armor / hydraulics | Slower movement, limp, poor jumps, bad recoil recovery, falling risk. |
| Arm / weapon mount | Worse aim, slower reload, weapon jam chance, dropped equipment. |
| Shield emitter | Lower shield capacity, slower recharge, flicker, overload risk. |
| Sensor pod | Worse visibility, lower AI confidence, weaker target lock, bad smoke/gas awareness. |
| Battery / reactor | Lower energy pool, heat spikes, shutdowns, explosion risk. |
| Jet pack / mobility module | Sputter, reduced thrust, fuel leak, unsafe landings. |
| Cockpit / pilot area | Pilot injury, panic, ejection warning, command delay. |
| Cooling system | Heat buildup, weapon lockout, engine damage, thermal signature. |

For equipment:

| Damaged Equipment | Reduced Performance |
|---|---|
| Rifle | Jams, misfires, worse accuracy, bent barrel, slower cycling. |
| Digger / drill | Slower cutting, overheating, bad material compatibility. |
| Repair tool | Lower repair rate, consumes more power, limited module types. |
| Shield tool | Smaller field, unstable edge, power drain. |
| Battery pack | Lower capacity, leakage, fire risk, shock hazard. |
| Sensor / scanner | Bad readings, short range, delayed updates. |

For base modules:

| Damaged Base Module | Reduced Performance |
|---|---|
| Turret | Slower tracking, reduced accuracy, jam, ammo feed failure, overheating. |
| Shield generator | Lower shield strength, slower recharge, flicker, overload. |
| Door | Slow open, jam, bad seal, lock failure, pressure leak. |
| Pump | Lower flow rate, reverse leak, power spikes. |
| Vent | Poor gas clearing, smoke backflow, oxygen loss. |
| Sensor mast | Lower range, blind spots, false positives. |
| Repair platform | Slower repair, cannot fix advanced modules, drains more power. |
| Battery | Lower capacity, heat, leakage, unstable discharge. |
| Command relay | Slower AI response, smaller command radius, weaker shared vision. |

This is important because it makes repair and salvage interesting. A damaged mech is not trash. It might still be worth fielding if the broken module does not matter for the next mission. Or it might become a liability because the left leg will fail under recoil.

### Base Building As Tactical Engineering

Base building should feel like designing a combat machine, not running a colony.

A good base layout should answer questions like:

- Where is the command core rooted?
- What is powered directly by the core?
- Which systems have battery backup?
- Which doors fail open or fail closed?
- Can smoke or toxic gas be vented?
- Can flooded rooms be pumped out?
- Are repair platforms protected but reachable?
- Do turrets have clean firing arcs?
- Are sensors protected from breach angles?
- Can androids/robots hold hazardous zones that humans cannot?
- Is there a safe path to extract the command core?
- Can the base survive if the core is embedded into a mech?

Different layouts should create different stories. A shield-heavy base might survive bombardment but fail when power is cut. A turret-heavy base might shred attackers but lose if smoke blocks sensors. A sealed base might handle toxic gas but become dangerous under pressure or flooding. A repair-focused base might keep veterans alive but become helpless if the repair bay is breached.

### The Engineering Disaster Example

Here is the kind of chain reaction the game should eventually support:

An enemy shell punches into the lower service room. The room begins flooding. Water reaches a damaged battery rack. The battery shorts and sends electricity through the floodwater. The pump should clear the water, but the pump is on the same local circuit and shuts down. Smoke from an oil fire enters the vent network. The AI medic refuses to enter because the route is electrified and toxic. An android can cross safely, but its leg actuator is damaged from the previous fight. You uproot the command core and embed it into a light mech to force the rescue. The mech gets stronger, but the base shield drops to reserve power. The enemy commander notices the shield dip and starts a breach push.

None of that needs to be scripted. It comes from systems:

- Material reaction.
- Liquid flow.
- Electricity conduction.
- Battery failure.
- Pump power dependency.
- Smoke ventilation.
- AI hazard perception.
- Damage-stage performance reduction.
- Command-core tradeoff.
- Enemy commander adaptation.

That is the whole pitch in one disaster.

### Why The Complexity Should Stay Fun

The danger with systems like this is that they can become unreadable. The roadmap tries to avoid that by making every important system observable.

The game should provide:

- Material overlays.
- Pressure overlays.
- Power overlays.
- Gas/smoke/toxic overlays.
- Heat overlays.
- AI reason labels.
- Damage-stage HUD icons.
- Base power panels.
- Replay cause chains.
- Captions for critical sounds.
- Debug/inspection tools for players, modders, and AI agents.

If a player dies because of pressure, heat, toxic gas, electricity, or a damaged component, the replay should be able to explain it.

That is the real promise for a Stationeers fan: not just complexity, but inspectable complexity under combat pressure.

## Why It Could Be Special

Lots of games have pieces of this idea:

- Cortex Command has fragile actors, physics chaos, modding, and destructible terrain.
- Noita has insane material causality.
- Barotrauma has disaster systems, pressure, flooding, and crew management.
- X-COM has named soldiers and loss.
- Soldat and Liero have fast 2D combat energy.
- The Powder Toy has material experimentation and community creation.

This game tries to combine the best parts without becoming a mess:

- Real-time 2D tactical combat.
- Strategy-style command.
- Optional direct possession.
- Destructible pixel terrain.
- Full physical collision.
- Layered body, armor, equipment, and mech damage.
- Base power and command-core risk.
- AI teammates that can explain themselves.
- Systemic materials and hazards.
- Replay and debrief tools that teach you.
- Scenario editor and modding from the start.
- Solo-first, but built toward LAN, online co-op, public PvP arenas, and persistent MMO shards through a dedicated server.

The big promise is not "more systems." It is "systems that create readable stories."

## The Visual And Audio Vibe

The battlefield is pixel-sim: chunky, readable, destructible, moddable, and physical.

The presentation is comic-noir: clean silhouettes, tactical panels, dramatic debrief cards, readable status banners, and a UI that makes the chaos understandable.

The audio is diegetic industrial synth-dread:

- Gunfire.
- Drills.
- Hydraulics.
- Servo grind.
- Shield buzz.
- Reactor hum.
- Radio chatter.
- Base alarms.
- Pilot eject warnings.
- Smoke vents.
- Repair tools.

Music is used carefully. The world itself should sound like the soundtrack, with synth dread rising when the command core is uprooted, a mech reactor is failing, the base power drops, or extraction is almost gone.

Every critical sound needs captions, because audio is not just flavor. It is tactical information.

## The Long-Term Dream

The first playable starts small: one actor, one breach, one enemy, one objective, one replay.

Then it grows:

1. Make one actor feel good.
2. Add a tiny breach mission that is actually fun.
3. Add destructible terrain and material affordances.
4. Add replay and event recording.
5. Add HUD and comic-noir UI.
6. Add equipment, armor, mechs, damage stages, and salvage.
7. Add full collision.
8. Add systemic materials and hazards.
9. Add AI trust harnesses and smarter teammates.
10. Add the command core, base power, mission director, and proof mission.
11. Add editor and mod tools.
12. Add dedicated server, LAN co-op, online co-op, PvP arenas, and persistent shards.

The roadmap is intentionally ambitious, but the implementation path is milestone-based. Each milestone has tests, replay evidence, and checklists so AI coding agents can build one slice at a time without guessing.

## The Pitch

This is a game about commanding fragile bodies and heavy machines through beautiful disasters.

It is about watching a perfect plan collapse because one rocket hit a pipe, then realizing you can still win if you flood the corridor, send an android through the toxic gas, recover the wounded pilot, and shove your command core into a smoking mech for one last push.

It is about AI teammates you trust enough to command, but can still possess when you want to personally make the shot.

It is about bases that matter, bodies that break, materials that react, mechs that limp, enemies that adapt, and replays that turn chaos into lessons.

The best version of this game should make you say:

"I need to try that mission again, because this time I know exactly what went wrong."

And then, ten minutes later:

"Wait. I made it worse. But it was amazing."
