# BP4→BP12 Future Features Inventory

Generated 2026-05-11 by Droid audit. **Scope: BP4 (M5.5 + M5.5.5 + M5.6 + M5.7 + M5.8) + BP5 (M5.9 + M5.9.5 + M5.10) + BP6 (M6 + M6.5 + M6.6) + BP7 (M7 + M7.5 + M7.7 + M4B) + BP8 (M8 + M8.5 + M8.6) + BP9 (M9 + M10) + BP10 (M11 + M9.5) + BP11 (M12) + BP12 (production T-tracks)**. Every line is a gap between what the codebase ships today and what BP4..BP12 require.

This file is a sibling of `MISSING_FEATURES.md` (which scopes BP0..BP3 closure gaps only). Anything that belongs to a milestone M5.5 or later — including the OPEN side-track / DR work that ships at those milestones — lives here.

## Top-10 highest-impact BP4→BP12 groups (read these first)

1. **§A.3 M5.6 Material Kernel (#38..#71)** — `cf-material` is a 1-line stub; Noita-grade chunked CA + 17 launch materials + reaction table + density layering + phase change all missing. Underpins everything else.
2. **§B.1 M5.9 Atmospherics-Grade Kernel (#98..#132)** — `cf-atmos` is a 1-line stub; PV=nRT atmospheres + pipe networks + combustion + suit life-support missing. The Stationeers-grade promise.
3. **§A.1 M5.5 Full Collision Gauntlet (#1..#32)** — Full collision matrix + projectile-projectile + CCD tiers + impulse-to-damage routing all missing.
4. **§C.1 M6 AI Core (#147..#164)** — DR-022 8-criteria humanlike bar; current cf-ai is single ReactiveGuard FSM.
5. **§F.1 M9 Dedicated Server (#242..#257)** — `cf-server`+`cf-server-ops`+`cf-server-persistence`+`cf-server-anti-cheat`+`cf-server-admin` are all 1-line stubs.
6. **§D.1 M7 Mission Director + Breach Contract + Bunker Defence (#182..#192)** — DR-017 typed mission manifest schema + director + Bunker Defence Proof Mission.
7. **§D.4 M4B Comic-Noir Polish (#207..#213)** — Mission cards, slowdown overlay, tactical map; DR-009 closure.
8. **§I.1-I.4 BP12 production T-tracks (#301..#322)** — Launch roster (70+ weapons, 44+ actors, etc.) + narrative bible + Tier-A 11 langs + LiveOps signing certs.
9. **§J.4-J.6 DR-005/DR-011/DR-013 architecture-from-day-one (#323..#363)** — Multiplayer/server/account/anti-cheat scaffolds missing.
10. **§A.5 M5.8 Origin Resource & Overclock Pass (#78..#97)** — Per-origin resource accumulators + overclock + downclock + G-Force blackout + ORIGIN-A acceptance suite.

Format: `[STATE] milestone-or-area — gap`
- `[GAP]` = feature/criterion listed but no code anywhere
- `[PART]` = partially implemented, contract not closed
- `[FAKE]` = code present but bypasses the production contract
- `[BP-GATE]` = BP-level closure gate item

---

## A. BP4 — Physics Sandbox Alpha (M5.5 + M5.5.5 + M5.6 + M5.7 + M5.8)

### A.1 M5.5 — Full Collision Gauntlet (DR-033 / T-PHYS)
1. [GAP] No `cf-physics` collision pipeline (broadphase + narrowphase + contact manifold + stable pair ids).
2. [GAP] No collision matrix loader (collision class × collision class → behavior).
3. [GAP] No deterministic pair ordering for replay determinism.
4. [GAP] No 16 collision classes bound to physics paths.
5. [GAP] No collision proxies for actor core / limbs / armor zones / held weapons / loose items.
6. [GAP] No collision proxies for kinetic / explosive projectiles / terrain proxies / debris chunks.
7. [GAP] No collision proxies for mech parts / base objects / force fields / sensor triggers.
8. [GAP] No controlled-animation vs physical-limb blend (self-collision filters for normal locomotion).
9. [GAP] No `collision_filter_reason` field on every filter.
10. [GAP] No CCD tier 1: discrete collision.
11. [GAP] No CCD tier 2: speculative contacts.
12. [GAP] No CCD tier 3: sweep ray.
13. [GAP] No CCD tier 4: sweep capsule.
14. [GAP] No CCD tier 5: sweep shape.
15. [GAP] No CCD tier 6: TOI substep.
16. [GAP] No projectile-projectile contact (bullet-bullet deflects/fragments/tumbles/loses energy).
17. [GAP] No explosive-projectile contact (detonate / fuze-fail / deflect by authored profile).
18. [GAP] No impulse-to-damage routing (collision impulse + contact area + sharpness + material pair + armor layer + origin/chassis → damage).
19. [GAP] No terrain chunk collision proxies updated from M2 dirty regions.
20. [GAP] No chunk-seam / tiny-hole / edge-case test fixtures.
21. [GAP] COLL-001 collision matrix generator (build fails on any physical pair with no rule).
22. [GAP] COLL-002 player/ally/enemy/AI unit-unit body collisions block/shove/knock-down/recover with events.
23. [GAP] COLL-003 limb-to-limb, limb-to-body, limb-to-terrain, limb-to-door contacts; detached limbs collide normally.
24. [GAP] COLL-004 held weapons collide with limbs/terrain/doors/other held weapons; owner self-filter reason-labeled.
25. [GAP] COLL-005 bullets hit bodies/armor/weapons/dropped items/terrain/shields/mech modules.
26. [GAP] COLL-006 bullet-bullet/projectile-projectile contacts → deflection/fragment/fuze/detonation per profile.
27. [GAP] COLL-007 high-speed projectiles + falling bodies do NOT tunnel through tiny holes / chunk boundaries / shields / thin limbs.
28. [GAP] COLL-008 physics impacts damage limbs/armor/equipment/chassis modules/debris/terrain/base objects/mechs at thresholds.
29. [GAP] COLL-009 Full Collision Gauntlet replays headlessly with identical contact ids/checksums.
30. [GAP] COLL-010 `cfctl observe --collisions` exposes live contacts/filters/last 30 collision events.
31. [GAP] COLL-011 perf report records 1080p/60 pass plus 4K/120 and Steam Deck status.
32. [GAP] COLL-012 AI pathing/behavior reacts to body blocking/debris/doors/shields/contact damage with reason labels.

### A.2 M5.5.5 — Micro Sabotage Fun Slice
33. [GAP] No `micro_sabotage` scenario (60-90 s; collapse catwalk / push crate / physics-driven kill).
34. [GAP] No `mission.physics_kill_count` field for the AT-LEAST-ONE-NON-RIFLE-KILL acceptance.
35. [GAP] No catwalk supports with impulse-threshold collapse behavior.
36. [GAP] No pushed-crate-vs-guard collision_damage_applied with reason `crush` / `debris_impact`.
37. [GAP] Two micro_sabotage cfctl scripts (win + loss) not authored.

### A.3 M5.6 — Material Kernel (DR-036 / T-MAT)
38. [GAP] `cf-material` is a 1-line stub.
39. [GAP] No `Material` struct (id / display_name / category / movement_class / density / viscosity / mass_per_pixel).
40. [GAP] No material schema (hardness / heat_capacity / thermal_conductivity / temperature / ignition_temperature / burn_rate).
41. [GAP] No material schema (oxygen_requirement / burn_products / phase_changes / conductivity / wetting / reaction_tags).
42. [GAP] No material schema (ai_affordances / ui_overlay_color / caption_priority / performance_tier / network_replay_mode).
43. [GAP] No `MaterialRegistry` BTreeMap-backed storage beyond the cf-terrain ZST.
44. [GAP] No chunked CA grid (64×64 Noita default).
45. [GAP] No deterministic update order for replay determinism.
46. [GAP] No dirty-rect tracker for downstream consumers.
47. [GAP] No sleeping-chunk policy.
48. [GAP] No per-pixel material id field.
49. [GAP] No per-pixel temperature field.
50. [GAP] No per-pixel state field (solid/liquid/gas/plasma).
51. [GAP] No CPU-deterministic kernel (DR-036 mandates).
52. [GAP] Launch material set 17 missing: water, steam/mist, smoke, fire/heat, oil/fuel, acid, toxic sludge, toxic gas, lava, blood/vomit, electricity charge, pebble/debris (9 of 17 — the 8 terrain ids exist; cf-terrain only carries solid/empty).
53. [GAP] No reaction table — no pair/triple reactions with priority/temperature/catalysts/byproducts.
54. [GAP] No `water + fire → steam` reaction.
55. [GAP] No `oil + ignition → fire on oil surface` reaction.
56. [GAP] No `lava + water → rock + steam` reaction (with heat dump latent-heat).
57. [GAP] No density layering (oil-on-water, sludge-below-water, gas-above-air).
58. [GAP] No phase change kernel (water ↔ steam at temperature thresholds).
59. [GAP] No liquid/gas movement rules (falling sand / falling liquid / rising gas).
60. [GAP] No viscosity-driven slow-flow.
61. [GAP] No `material.*` event category in cf-replay.
62. [GAP] No `reaction.*` event category in cf-replay.
63. [GAP] No per-chunk material checksum for replay determinism.
64. [GAP] No chunk-budget governor (32 active 64×64 chunks at 60 Hz).
65. [GAP] No `cfctl observe --materials` command.
66. [GAP] No `cfctl inspect material <chunk-id>` command.
67. [GAP] No MAT-01 acceptance (256×256 sandbox sand/water/oil/steam/fire for 5 min at ≥60 FPS).
68. [GAP] No MAT-02 acceptance (reaction-table; water+fire→steam; oil+spark→fire-on-oil; lava+water→rock+steam).
69. [GAP] No MAT-03 fire package (oil trail burns; sealed room consumes oxygen; water extinguishes fire).
70. [GAP] No MAT-06 density/layering (60 s stable oil/water/sludge/gas).
71. [GAP] No MAT-13 minimal replay determinism (same seed + inputs → identical material checksum after 10,000 ticks).

### A.4 M5.7 — Hazard Package
72. [GAP] No MAT-04 acid neutralization (water + acid → reaction + byproducts + damage reduction).
73. [GAP] No MAT-05 electricity through wet metal (energize puddle touching metal door + actor; conduction).
74. [GAP] No MAT-07 debris impact (kicked pebble damages enemy at speed; bounces at low speed).
75. [GAP] No MAT-08 ingestion (actor ingests toxic sludge → poisoned affliction + vomit material spawn).
76. [GAP] No `affliction.*` event family.
77. [GAP] No affliction kinds in actor state beyond the AfflictionKind enum scaffold (no per-tick effects).

### A.5 M5.8 — Origin Resource & Overclock Pass
78. [GAP] No `origin_id` enum field on actor records (`human`, `android_*`, `robot`, modder origins) beyond a scaffold.
79. [GAP] Per-origin impulse-to-damage branches in M5.5-008 not implemented.
80. [GAP] `g_load_dose` + `concussion_dose` accumulator on humans — not consumed per tick.
81. [GAP] `internal_shock` damage to robot per-module — not consumed per tick.
82. [GAP] `caloric_energy` resource on humans + android organic side — not consumed per tick.
83. [GAP] `battery_charge` resource on android battery variants — not consumed per tick.
84. [GAP] `power` resource on robots — no `resource.power_action_rejected` event.
85. [GAP] `heat` resource on robot global / android per-module — not consumed.
86. [GAP] `oxygen_supply` resource on humans + androids with helmet+tank — not consumed.
87. [GAP] Afflictions: `internal_shock` / `coolant_leaking` / `oil_leaking` / `overheating` / `low_battery` / `power_starved` / `weak` / `exhausted` / `hypoxia` / `downclocked` / `heat_exhaustion` — enum kinds exist but no actor-tick consumption.
88. [GAP] Voluntary overclock state machine on robot/android — not implemented.
89. [GAP] Involuntary downclock state machine — not implemented.
90. [GAP] `chassis_thermal_throttle_started` event — not emitted.
91. [GAP] Coolant + oil leak channels for robots — not implemented.
92. [GAP] G-Force vision blackout HUD effect (vignette darkens proportional to g_load_dose) — not implemented.
93. [GAP] `--reduced-g-force-blackout` accessibility flag — not implemented.
94. [GAP] Origin-gated equipment validation (`origin_compatibility` field on items rejects with `wrong_origin_for_equipment`) — not implemented.
95. [GAP] AI doctrine origin awareness (`wrong_origin_for_treatment`, `power_below_threshold`, `low_battery_module_lockout`) — not implemented.
96. [GAP] `cfctl observe --origin-state <actor>` command — not implemented.
97. [GAP] ORIGIN-A-01..ORIGIN-A-15 acceptance suite — not authored.

## B. BP5 — Atmospherics & Worlds Alpha (M5.9 + M5.9.5 + M5.10)

### B.1 M5.9 — Atmospherics-Grade Kernel (DR-037)
98. [GAP] `cf-atmos` is a 1-line stub.
99. [GAP] No `Atmosphere` unit struct (per-room/pipe/suit/canister/lung atmosphere).
100. [GAP] No per-gas mole-quantity tracking.
101. [GAP] No `P = nRT/V` pressure calculation with `R = 8314.46`.
102. [GAP] No 10 launch gases (O2 / N2 / CO2 / H2O vapor / H2 / N2O / O3 / Volatiles / Pollutant / Noxious).
103. [GAP] No 6 launch liquid mixtures.
104. [GAP] No active-region scheduling for atmospheres.
105. [GAP] No `cf-atmos::combustion` 6 locked launch reactions (Volatiles+O2/N2O/O3, H2+O2/N2O/O3).
106. [GAP] No `cf-atmos::phase_change` per-gas vapor pressure curve.
107. [GAP] No gradual condensation / evaporation / freezing with latent heat.
108. [GAP] No pipe-rupture detection (frozen content / liquid stress / ΔP thresholds).
109. [GAP] No `cf-atmos::pipe_network` connected-segment graph.
110. [GAP] No pipe pumps / valves / regulators / volume + turbo pumps / filtration / one-way / purge / pressurant.
111. [GAP] No condensation / expansion / evaporation chambers.
112. [GAP] No `cf-atmos::room_detection` per-tick connected-volume detection.
113. [GAP] No sealed-cell collapse for kernel performance.
114. [GAP] No per-cell partial-pressure HUD queries.
115. [GAP] No `cf-atmos::door_state_machine` (closed_sealed / closed_unsealed / cycling_open / open / cycling_close / breached).
116. [GAP] No airlock controller (canonical 2-door + 2-active-vent + console assembly).
117. [GAP] No `cf-atmos::apertures` (door openings / vents / windows / bullet holes / shaped-charge cuts / blast breaches / pipe ruptures / suit punctures / terrain cracks).
118. [GAP] No aperture-flow calculation `Flow = ΔP × aperture area` with choked-flow caps.
119. [GAP] No `cf-atmos::liquid_flow` (pressure/head/gravity-driven jets / sprays / flooding / siphons).
120. [GAP] No `cf-atmos::thermal_transfer` conduction / convection / advection / phase-change latent heat.
121. [GAP] No combustion/electrical/collision heat sources.
122. [GAP] No bounded ambient/radiative thermal exchange.
123. [GAP] No `cf-atmos::suit_life_support` (lung + helmet + suit nested atmospheres).
124. [GAP] No canister + filter + waste-tank slots.
125. [GAP] No breathing math `0.0048 mol/tick · BreathingRate · BreathingEfficiency`.
126. [GAP] No helmet flush function; no filter max waste-tank pressure 4052 kPa enforcement.
127. [GAP] No `cf-atmos::planetary_ambient` (Earth/Mars/Moon/Mimas/Europa/Vulcan/Venus locked).
128. [GAP] No modder schema for new ambients via `content/worlds/`.
129. [GAP] No `cf-atmos::wind` ΔP-driven impulse force on actors / items / debris / gibs.
130. [GAP] No `cf-atmos::stratification` (per-tick partial-pressure adjust by local g × molar mass spread).
131. [GAP] No `cfctl observe --atmospheres` / `--pipe-networks` / `--rooms` / `--suits` / `--gravity` / `--ballistics` commands.
132. [GAP] No ATMOS-A-01..ATMOS-A-19 acceptance (PV=nRT correctness / mixing / pressure spike / combustion / pipe network / filtration / planetary ambient / suit life-support / filter mismatch / helmet flush / phase change / wind force / photosynthesis / furnace combustion / determinism replay / bullet-hole depressurization / liquid jet/flooding / material heat transfer / player thermal techniques).

### B.2 M5.9.5 — Micro Pressure Hold Fun Slice
133. [GAP] No `micro_pressure_hold` scenario (60-90 s; vent room via breach / ignite atmosphere / freeze via coolant).
134. [GAP] No `mission.atmospheric_kill` field for the AT-LEAST-ONE-ATMOSPHERIC-KILL acceptance.
135. [GAP] No room pressure / oxygen % / temperature / suit oxygen-remaining HUD lines.
136. [GAP] Two micro_pressure_hold cfctl scripts (win + loss) not authored.

### B.3 M5.10 — Environmental Conditions Aggregation (DR-040)
137. [GAP] `cf-environment` aggregator crate scaffolded but no per-tick computation.
138. [GAP] `EnvironmentSignal` struct exists but no SoA actors / SIMD friendly path.
139. [GAP] 15-class hazard taxonomy declared but no consumer (no AI / HUD / replay / audio).
140. [GAP] Tick schedule for aggregator (after kernels, before consumers) not enforced.
141. [GAP] `cfctl observe --environment <actor>` command not implemented.
142. [GAP] `environment` run-bundle event category not emitted.
143. [GAP] CI grep gate (no consumer reads atmospheric / gravitational data outside `cf-environment::for_actor(...)`) — not active.
144. [GAP] ENV-A-01..ENV-A-10 acceptance suite — not authored.
145. [GAP] 12-world catalog (Earth / Mars / Phobos / Deimos / Moon / Mimas / Europa / Vulcan / Venus / Sol-zone habitats / belt asteroids / orbital stations) — `content/worlds/` directory missing.
146. [GAP] DR-039 per-planet astrography (rotation_period_seconds + axial_tilt_deg + semi_major_axis_au + parent.solar_distance_au) schema not declared.

## C. BP6 — AI Combat Alpha (M6 + M6.5 + M6.6)

### C.1 M6 — AI Core And Trust Harness (DR-008 / DR-022 closure)
147. [GAP] No `cf-ai` perception model (sight cone + hearing range + memory grid for last-known positions).
148. [GAP] No utility scoring + doctrine slots (cautious / aggressive / support / scout / sniper).
149. [GAP] No `tactic_chosen` reason-label vocabulary beyond initial M1.5 set.
150. [GAP] No mistake/recovery model (panic / miss / stuck recovery).
151. [GAP] No AI-H scenario runner (AI-H-01..AI-H-06).
152. [GAP] No reason-label HUD overlay.
153. [GAP] No cross-mission state stub (faction commander persists across same campaign session, file-based).
154. [GAP] DR-022 criterion 1 "Intent" — bots announce actions; not implemented.
155. [GAP] DR-022 criterion 2 "Perception" — bots act from sight/hearing/memory; not implemented.
156. [GAP] DR-022 criterion 3 "Doctrine/personality" — cautious medic / aggressive breacher / etc.; not implemented.
157. [GAP] DR-022 criterion 4 "Plausible mistakes" — beyond static miss_chance; not implemented.
158. [GAP] DR-022 criterion 5 "Recovery" — replan after terrain destruction / pick up dropped gear / call for help; not implemented.
159. [GAP] DR-022 criterion 6 "Strategic adaptation" — enemy commander remembers tactics across missions; not implemented.
160. [GAP] DR-022 criterion 7 "Replay proof" — every AI decision in replay viewer shows perception / options / score / chosen / result.
161. [GAP] DR-022 criterion 8 "Fairness" — no hidden vision/range bonuses; not tested.
162. [GAP] `perception_updated` event family — not in event taxonomy.
163. [GAP] `recovery_action` event family — not emitted.
164. [GAP] `commander_adaptation` event family — not emitted.

### C.2 M6.5 — LLM Mind Lab (DR-032)
165. [GAP] `cf-ai::mind::schema` (MindObservationFrame / MindTask / AiMindProposal / MindValidationResult / MindMemoryRecord / MindProviderConfig) — not declared.
166. [GAP] JSON schemas under `game/crates/cf-ai/schemas/mind/v1/` — not present.
167. [GAP] `cf-ai::mind::provider` shared trait + mock/openai/anthropic/ollama/openai-compatible adapters — not present.
168. [GAP] `cf-ai::mind::compressor` (derives MindObservationFrame from observe stream with fog-of-war) — not present.
169. [GAP] `cf-ai::mind::validator` rejects stale/invalid/impossible/unfair/over-budget/hidden-info/capability-violating proposals.
170. [GAP] `cf-ai::mind::policy` applies accepted proposals as utility-weight patches / commander-blackboard goals / doctrine tags / dialogue / memory writes.
171. [GAP] `mind` event category (mind.task_created / prompt_recorded / response_received / proposal_validated / patch_applied / patch_rejected / memory_written) — not emitted.
172. [GAP] `cfctl observe --mind-frame <scope>` — not implemented.
173. [GAP] `content/scenarios/micro_breach_mind_lab.ron` (mind_off / mind_mock / mind_live_optional modes) — not authored.
174. [GAP] `cf-tools-editor` mind dashboard — not implemented.
175. [GAP] MIND-001..MIND-010 acceptance suite — not authored.

### C.3 M6.6 — AI Environmental Competence (DR-040)
176. [GAP] AI hazard perception map (per-actor view of nearby material/temperature/electricity/gas fields with fog-of-war) — not implemented.
177. [GAP] AI affordance tags consumed (avoid / seek / use-as-weapon / extinguish-with / neutralize-with / vent / pump) — not implemented.
178. [GAP] Utility scorer hazard-cost extension — not implemented.
179. [GAP] `tactic_chosen` reasons with `material_*` suffix (e.g., `material_acid_neutralize_with_water`) — not implemented.
180. [GAP] AI-MAT-01..AI-MAT-08 acceptance suite — not authored.
181. [GAP] AI-ENV per-world doctrine (Mars dust storm visibility / Vulcan combustible-atmosphere awareness / Mimas microgravity grenade arcs / vacuum radio-only comms / Bunker Defence per-team doctrine) — not authored.

## D. BP7 — Vertical Slice Alpha (M7 + M7.5 + M7.7 + M4B)

### D.1 M7 — Mission Director + Breach Contract + Bunker Defence (DR-042)
182. [GAP] DR-017 typed mission manifest schema (objectives / teams / terrain_rules / command-core/base state / capability_requirements / director_phases / save_fields / replay_events) — not implemented.
183. [GAP] Mission director (pacing / reinforcement / LZ risk / objective escalation with reason labels) — not implemented.
184. [GAP] DR-015 command-core mechanic minimum (rooted core powers ≥ 2 base systems: shield + 1 turret) — not implemented.
185. [GAP] DR-015 uproot core → embeds into player avatar with stat boost — not implemented.
186. [GAP] DR-015 losing core = mission failure if `command_core_endgame` policy — not implemented.
187. [GAP] Base system slice (command core + power grid + 1 shield + 1 turret + 1 door + 1 repair pad) — not implemented.
188. [GAP] Breach Contract scenario (compound entry → wall breach → 2-3 enemies → extract before timer) — not authored.
189. [GAP] Bunker Defence Proof Mission (DR-042; rooted defender + dropship attacker + Coop-Defence variant) — not authored.
190. [GAP] Comic-noir pre-/post-mission cards (M4B scope; pre/post UI) — not implemented.
191. [GAP] Death recap from replay (auto-rendered debrief markdown) — not implemented.
192. [GAP] MISSION-A acceptance tests — not authored.

### D.2 M7.5 — Base Atmospherics (DR-027 + DR-037)
193. [GAP] DR-027 base shields module — no shield game-object.
194. [GAP] DR-027 base turrets module — no turret game-object.
195. [GAP] DR-027 base sensors module — no sensor game-object.
196. [GAP] DR-027 base doors module — no door game-object.
197. [GAP] DR-027 base repair pads — no repair-pad game-object.
198. [GAP] DR-027 base power grid — no power graph.
199. [GAP] MAT-09 hull/gap network test (blast a hull breach; verify flooding + pressure equalization + actor pull force).
200. [GAP] MAT-10 base-equipment-loop test (damage a pump; oxygen + water levels respond; AI repair task fires).
201. [GAP] Heaters / coolers / heat exchangers / radiators / coolant loops / insulated panels — not implemented.
202. [GAP] Atmospherics event categories (atmospherics.hull_breached / flooded / depressurized / oxygen_depleted / fire_started / fire_extinguished / pump_repaired / vent_opened / alarm_triggered / aperture_created / thermal_transfer) — not emitted.

### D.3 M7.7 — Day/Night, Weather & Dynamic Events (DR-039 + DR-040)
203. [GAP] `cf-environment::day_night` kernel (per-tick local solar time + sun elevation + solar phase per World data) — not implemented.
204. [GAP] `cf-environment::weather` kernel (per-world weather variation table; deterministic event firing per scenario seed).
205. [GAP] Weather event roster (mars_dust_storm / mars_local_dust_devil / mars_thermal_inversion / vulcan_thermal_storm / vulcan_oxidizer_pocket / europa_cryo_storm / mimas_meteor_shower / solar_flare_minor/major / magnetic_storm / earth_thunderstorm).
206. [GAP] WEATHER-A acceptance suite — not authored.

### D.4 M4B — Comic-Noir Polish (DR-019 + DR-009 closure)
207. [GAP] Comic-noir mission cards (pre + post) — not implemented.
208. [GAP] Stylized event banners — not implemented.
209. [GAP] Slowdown overlay (DR-009; 25% time dilation) — not implemented.
210. [GAP] Tactical map polish — not implemented.
211. [GAP] DR-009 hold-or-toggle slowdown key — not bound.
212. [GAP] DR-009 commander focus charge resource — not declared.
213. [GAP] DR-009 ORDER-01 acceptance — not authored.

## E. BP8 — Creator Alpha (M8 + M8.5 + M8.6)

### E.1 M8 — Scenario Editor And Mod Tools (DR-006 + DR-030)
214. [GAP] DR-030 `cf-tools-editor` in-engine workbench mode — scaffolded only.
215. [GAP] DR-030 `.cfpkg` export — not implemented.
216. [GAP] DR-006 `.cfpkg` package format — not defined.
217. [GAP] DR-006 trust tier schema — not defined.
218. [GAP] DR-006 capability declarations — not enforced.
219. [GAP] DR-006 scripted hook surface (mlua vs Rhai) — decision OPEN.
220. [GAP] DR-006 mod hash sync for multiplayer — not implemented.
221. [GAP] Sample mod (new chassis archetype using same grammar) — not authored.
222. [GAP] Lua/Rhai scripting host — not chosen.
223. [GAP] Scenario validator (catches missing fields / broken refs / AI policy violations / accessibility issues) — not implemented.
224. [GAP] PACK-A + MOD-A acceptance — not authored.

### E.2 M8.5 — Material Lab
225. [GAP] `cf-tools-editor` material lab mode — not implemented.
226. [GAP] Material inspect tool (click pixel → tooltip with id/temperature/state/density/last reaction).
227. [GAP] Recipe journal (reactions discovered by player; UI shows reagents → byproducts).
228. [GAP] Stamps (`.cfstamp` files; community-shareable) — not implemented.
229. [GAP] Material packs (`.cfpkg` declares new materials/reactions/ingestion effects/pipe devices/AI affordances).
230. [GAP] AI puppet test (`cfctl --bot puppet --scenario <author-pkg>`) — not implemented.
231. [GAP] MAT-11 inspect tool acceptance + MAT-14 designer-authored-puzzle acceptance — not authored.

### E.3 M8.6 — Mining And Extraction (DR-041)
232. [GAP] `cf-equipment` mining tool roles + content rows (Sampler / LightDigger / HeavyDrill / CoreDrill / RefiningStation / SmelterFurnace / EnrichmentReactor / OreCargoBay / ConveyorBelt) — not authored.
233. [GAP] `cf-material` ore-as-material entries — not authored.
234. [GAP] `cf-mission` mining objective schema + dynamic-event hooks — not implemented.
235. [GAP] `cf-server-persistence` resource ledger — not implemented.
236. [GAP] Per-world ore deposit generator (deterministic seed) — not implemented.
237. [GAP] AI miner doctrine (prospect / drill / refine / haul / retreat with reason labels) — not implemented.
238. [GAP] `cfctl observe --mining` — not implemented.
239. [GAP] `mining` run-bundle event category — not emitted.
240. [GAP] AI-MINE-A-01..AI-MINE-A-08 acceptance suite — not authored.
241. [GAP] 12 launch ores — not declared.

## F. BP9 — Server/LAN Alpha (M9 + M10)

### F.1 M9 — Dedicated Server App (DR-034)
242. [GAP] `cf-server` is a 36-line scaffold.
243. [GAP] `cf-server-ops` is a 1-line stub (config loader / health / readiness / Prometheus metrics / structured logs / drain / restart).
244. [GAP] `cf-server-persistence` is a 1-line stub (snapshot writer + event journal + restore loop).
245. [GAP] `cf-server-anti-cheat` is a 1-line stub (input validation / rate-limit hooks / capability gates / audit log).
246. [GAP] `cf-server-admin` is a 1-line stub (capability-gated cfctl-shape admin endpoints).
247. [GAP] Anti-cheat profiles (`casual`, `competitive`, `tournament_strict`) — not declared as enums.
248. [GAP] Determinism island contracts (which subsystems are bit-deterministic; which stochastic-but-replayable; which cosmetic) — not documented.
249. [GAP] Reference Docker image for community deployments — not present.
250. [GAP] Networking transport library (lightyear / renet / quinn) — not chosen.
251. [GAP] `cf-server --mode coop_room` boots + accepts 2-4 clients + runs Breach Contract — not implemented.
252. [GAP] `cf-server --mode pvp_arena` boots + accepts 4-player session — not implemented.
253. [GAP] `cf-server --mode lan_room` auto-discovered on LAN — not implemented.
254. [GAP] `cf-server --mode mmo_shard` boots + persistence snapshot every 10 min + restart restore < 30 s — not implemented.
255. [GAP] `cf-server --mode lobby_directory` returns shard list — not implemented.
256. [GAP] DET-A acceptance suite — not authored.
257. [GAP] SERVER-001..SERVER-016 acceptance suite — not authored.

### F.2 M10 — LAN Co-op
258. [GAP] `cf-net` authority model (server-authoritative for sim; clients send inputs via cf-control envelope) — not implemented.
259. [GAP] LAN discovery (mDNS / UDP broadcast) — not implemented.
260. [GAP] Lobby + ready-up flow in client — not implemented.
261. [GAP] Replicated state (actors / terrain / inventory / objective state / base modules) — not implemented.
262. [GAP] Co-op friendly fire policy — not implemented.
263. [GAP] Per-client replay bundles align tick-for-tick (`cf-headless replay-compare`) — not implemented.
264. [GAP] Anti-cheat profile `casual` enabled by default — not implemented.
265. [GAP] Mod hash sync on join — not implemented.

## G. BP10 — Online Beta (M11 + M9.5)

### G.1 M11 — Online Co-op (Self-Hosted)
266. [GAP] NAT punch-through or relay using chosen transport — not implemented.
267. [GAP] `lobby_directory` integration (register / heartbeat / deregister) — not implemented.
268. [GAP] Code-based join + browse-list lobby UI — not implemented.
269. [GAP] Package hash sync with soft-fail dev / hard-fail shipping — not implemented.
270. [GAP] Latency compensation (client-side prediction + server reconciliation for player actor) — not implemented.
271. [GAP] Steam Datagram Relay / EOS adapter behind cargo features — not implemented.
272. [GAP] Reference systemd / launchd / Docker configs — not present.
273. [GAP] Anti-cheat profile `competitive` enabled by default — not implemented.
274. [GAP] Account adapter (local file / lobby_directory token / Steam/EOS/PlayFab stubs) — not implemented.

### G.2 M9.5 — Voice + Radio Sim (DR-043)
275. [GAP] `cf-comms` crate (acoustic propagation kernel + radio propagation kernel) — not present.
276. [GAP] Audio middleware `bevy_kira_audio` primary + `bevy_fmod` optional flag — not present.
277. [GAP] Voice codec Opus — not present.
278. [GAP] Frequency band registry (HF / VHF / UHF / Microwave) — not declared.
279. [GAP] Radio hardware roster (PRR-Lite / Squad-Mk1/Mk2 / LongHaul-AT / Dish-Beacon / HAM-Field / Ionopulse / Robot-Internal / Android-Module) — not authored.
280. [GAP] Antenna roster (whip / long-whip / dipole-wire / yagi / microwave-dish / helical / ground-spike) — not authored.
281. [GAP] Audio reconstruction (band-limit 300-3000 Hz / compander / static-gating / distortion / squelch tail) — not implemented.
282. [GAP] Origin gating (humans equip / robots built-in / androids built-in OR modular) — not implemented.
283. [GAP] AI subscriptions (radio.transmission_received triggers doctrine reasoning) — not implemented.
284. [GAP] `voice` + `radio` run-bundle event categories — not emitted.
285. [GAP] COMMS-A-01..COMMS-A-15 acceptance suite — not authored.

## H. BP11 — Public Systems Beta (M12)

### H.1 M12 — Public PvP Arenas + Persistent MMO Shards (DR-035 + DR-042 + DR-043)
286. [GAP] `cf-server --mode pvp_arena` 2-8 player server-authoritative match server — not implemented.
287. [GAP] PvP-specific scenarios under `content/scenarios/pvp/` — not authored.
288. [GAP] Persistent world manifest (region map / materials / hazards / faction territories) — not declared.
289. [GAP] Persistent state store (snapshot every 10 min + append-only event journal) — not implemented.
290. [GAP] Persistent terrain (carved/repaired regions survive reboot) — not implemented.
291. [GAP] Persistent bases (base layouts + module HP/ammo/power state) — not implemented.
292. [GAP] Persistent named-actor veterans across sessions — not implemented.
293. [GAP] Persistent faction state + commander memory + LLM mind memory writes — not implemented.
294. [GAP] Account adapter for public shards — not implemented.
295. [GAP] Interest management (clients only receive in-range events/snapshots) — not implemented.
296. [GAP] Anti-cheat profile `competitive` default — not implemented.
297. [GAP] Lobby/portal model (player log-out on Shard A, log-in on Shard B) — not implemented.
298. [GAP] MMO-001..MMO-012 acceptance suite — not authored.
299. [GAP] Reference Docker image + hosting guide for MMO operators — not present.
300. [GAP] 50-100 concurrent player target (community + regional tiers) — not measured.

## I. BP12 — Release Candidate (production T-tracks)

### I.1 T-CONTENT-ART (DR-044 + DR-045)
301. [GAP] M-ART-0 Tier 1 SVG pipeline bootstrap (`tools/asset_gen/build_placeholders.py`) — not implemented.
302. [GAP] Per-faction palette JSON (`content/palettes/<faction>.json`) — not authored.
303. [GAP] Per-category generators (actors / weapons / vehicles / base objects / materials / UI icons) — not implemented.
304. [GAP] License-clean fonts (JetBrains Mono + Press Start 2P + Noto) — not present.
305. [GAP] Placeholder audio (sine/square synth blips) — not present.
306. [GAP] Tier 2 hand-tuned art pass — not started.
307. [GAP] Tier 3 polish + animation pass — not started.
308. [GAP] Launch roster (70+ weapons / 44+ actors / 18+ vehicles / 60+ base objects / 8 factions / 30+ missions / 12 worlds × 3-5 biomes / 17 materials / 12 ores / 30+ music tracks / 400+ SFX) — not declared.

### I.2 T-CONTENT-NARRATIVE
309. [GAP] ~80,000 words narrative bible — not authored.
310. [GAP] 8 faction archives — not authored.
311. [GAP] 24+ named NPCs — not authored.
312. [GAP] ~600 codex entries — not authored.

### I.3 T-LOCALIZATION (DR-046)
313. [GAP] Tier-A 11 languages (Project Fluent files under `content/locales/<lang>/`) — not authored.
314. [GAP] Tier-B 8 UI-only languages — not authored.
315. [GAP] Mod-localization layer — not implemented.

### I.4 T-LIVEOPS (DR-047)
316. [GAP] Telemetry endpoint (in-game crash + bug report + opt-in playtest data) — not implemented.
317. [GAP] Bug-report tool — not implemented.
318. [GAP] Marketing site / Steam page — not configured.
319. [GAP] Legal / compliance review — not started.
320. [GAP] Apple Developer Program signing cert ($99/year) — not procured.
321. [GAP] Authenticode signing cert ($200-$400/year) — not procured.
322. [GAP] Sustainability / sunset posture documented — not authored.

## J. Cross-cutting (Side Track) — Items deferred to BP4+ from BP0-BP3 status surface

### J.1 DR-005 — Multiplayer-architecture-from-day-one BP4+ implementation
323. [GAP] `cf-net` adapter trait + lightyear/renet/quinn candidate implementations — BP9 owns; not present.
324. [GAP] DR-005 "100% server-authoritative for sim state" — BP9 wires `cf-server`; in-process engine only at BP3.
325. [GAP] DR-005 per-client bandwidth floor 64 KB/s perf gate — BP10 owns; not measured.
326. [GAP] DR-005 LLM-mind server-side run path — BP6 + BP10 owns; not implemented.

### J.2 DR-021 — Mech-scale ladder BP4+ expansion
327. [GAP] DR-021 Medium mech tier (~4-5× human) — BP4+ expansion.
328. [GAP] DR-021 Heavy mech tier (~6-10× human) — BP4+ expansion.
329. [GAP] DR-021 archetype "Armored" — BP4+ expansion.
330. [GAP] DR-021 archetype "Shielded" — BP4+ expansion.
331. [GAP] DR-021 archetype "Assault" — BP4+ expansion.
332. [GAP] DR-021 archetype "Engineer / Siege" — BP4+ expansion.
333. [GAP] DR-021 archetype "Recon / Sensor" — BP4+ expansion.
334. [GAP] DR-021 archetype "Support / Repair" — BP4+ expansion.
335. [GAP] DR-021 archetype "Command" — BP4+ expansion (requires DR-015 command core too).
336. [GAP] DR-021 archetype "Experimental / Biomech" — BP4+ expansion.
337. [GAP] DR-021 module "Arm weapons" interchangeable — BP4+.
338. [GAP] DR-021 module "Shoulder weapons" — BP4+.
339. [GAP] DR-021 module "Reactors" — BP4+.
340. [GAP] DR-021 module "Batteries" — BP4+.
341. [GAP] DR-021 module "Cockpit upgrades" — BP4+.
342. [GAP] DR-021 module "Melee tools" — BP4+.
343. [GAP] DR-021 module "Cargo clamps" — BP4+.
344. [GAP] DR-021 module "Command relays" — BP4+.
345. [GAP] DR-021 module "Deployable turrets" — BP4+.
346. [GAP] DR-021 module "Special abilities" — BP4+.
347. [GAP] DR-021 module "swapped" verb — BP4+.
348. [GAP] DR-021 mission manifest "max chassis tier" parameter — M7 owns; BP4+ deferred.

### J.3 DR-029 — Save game BP4+ scope
349. [GAP] DR-029 multi-slot save support — BP5+ owns.
350. [GAP] DR-029 autosave before/after contracts — BP5+ owns.
351. [GAP] DR-029 mission suspend/resume — BP5+ owns.
352. [GAP] DR-029 same-seed retry — BP5+ owns.
353. [GAP] DR-029 ironman / scenario policy persistence — BP5+ owns.
354. [GAP] DR-029 replay archive linked to saves — BP5+ owns.
355. [GAP] DR-029 base modules + actors/veterans + mechs + salvage + faction state + enemy commander memory + mission manifests + scenario policy in cf-save — BP4+ owns.

### J.4 DR-011 — Progression / retention loop (M7)
356. [GAP] DR-011 veteran persistence across same campaign session — not in cf-save.
357. [GAP] DR-011 salvage as retention loop reward — not implemented.
358. [GAP] DR-011 template edits / next-contract suggestions — not implemented.

### J.5 DR-013 — Backend service scope (M9+)
359. [GAP] DR-013 lobby_directory schema — not declared.
360. [GAP] DR-013 account adapter trait — not declared.
361. [GAP] DR-013 anti-cheat profile schema — not declared.
362. [GAP] DR-013 telemetry endpoint — not declared.
363. [GAP] DR-013 audit log persistence — not declared.

### J.6 DR-038 — Universal gravity (M5.9 closes)
364. [GAP] DR-038 ballistic drag reads atmospheric ρ_local — no atmospheric ρ (cf-atmos stub).
365. [GAP] DR-038 projectile mass — projectiles have no mass field.
366. [GAP] DR-038 projectile cross-sectional area for drag — not implemented.
367. [GAP] DR-038 "No hardcoded 9.81" CI grep gate — not active.
368. [GAP] DR-038 GravityField per-cell sampling — only Uniform(f32) at BP3.
369. [GAP] DR-038 per-region overrides (gravity well / low-g lab / magnetic boots / damaged grav generator / reverse-g chamber) — not implemented.
370. [GAP] GRAV-A-01..GRAV-A-10 acceptance suite — not authored.

### J.7 DR-006 — Modding data model BP4+ scope (M8 closes)
371. [GAP] DR-006 mod-marketplace surface — BP8+ owns.
372. [GAP] DR-006 mod-author license declaration in `.cfpkg` manifest — BP8+ owns.

### J.8 Status surface drift items that BP3+ cannot fully close
373. [GAP] README "Layered Simulation" ASCII diagram claims Stationeers-grade atmospherics with PV=nRT — BP5 owns; BP3 cannot fix prose drift.
374. [GAP] README "Systemic Materials (Noita-grade chunked CA kernel)" claim — BP4 owns; BP3 cannot fix prose drift.
375. [GAP] README "Full Collision Physics (everything physical collides by default)" — BP4 owns.
376. [GAP] README "Universal Gravity Field (one source; sampled per-cell per-tick)" — BP5 owns.
377. [GAP] README "Multi-mode multiplayer ladder" — BP9+ owns.

### J.9 cf-equipment BP4+ scope
378. [GAP] cf-equipment shotgun preset — BP4+.
379. [GAP] cf-equipment SMG preset — BP4+.
380. [GAP] cf-equipment pistol preset — BP4+.
381. [GAP] cf-equipment launcher preset — BP4+.
382. [GAP] cf-equipment digger preset — BP4+ (referenced by M2 done-criteria; only soft-breach digger at BP3).
383. [GAP] cf-equipment medkit preset — BP4+.
384. [GAP] cf-equipment shield preset — BP4+.
385. [GAP] cf-equipment sensor preset — BP4+.
386. [GAP] cf-equipment grapple preset — BP4+.
387. [GAP] cf-equipment grenade-throw verb — BP4+.
388. [GAP] cf-equipment melee-strike verb — BP4+.

### J.10 cf-physics BP4+ scope
389. [GAP] cf-physics terminal_velocity per origin — M5.8+ owns.
390. [GAP] cf-physics friction per surface — M5.5+ owns.
391. [GAP] cf-physics slope-walking — M5.5+ owns.
392. [GAP] cf-physics ladder/wall-climb — M5.5+ owns (Climbing stance scaffolded at M5).
393. [GAP] cf-physics swim physics for water surfaces — M5.6+ owns.

### J.11 spec/comms-voice-and-radio-model (M9.5)
394. [GAP] T-COMMS side track — BP4+ surfaces are placeholder; full closure at M9.5.

### J.12 spec/persistent-mmo-architecture (M12)
395. [GAP] MMO shard architecture full — M12 owns.

### J.13 DR-031 / DR-057 — Monetization gaps (architectural-allowed-but-dormant)
396. [GAP] Optional cosmetic battle-pass scaffold (toggleable + default-off + no gameplay power lock) — BP12 owns.
397. [GAP] Anti-FOMO archive + earn-back path (DR-057) — BP10+ owns.
398. [GAP] Cosmetic locker UI (dormant default-off) — BP12 owns.
399. [GAP] Mod marketplace cut posture enforcement — BP8+ owns.

### J.14 DR-051 — Accessibility-plus (BP9..BP12)
400. [GAP] Cognitive + motor + hearing + reading + sensory presets — BP9+ owns.
401. [GAP] 8 color-blind protocols — BP9+ owns.
402. [GAP] Cinematic accessibility — BP9+ owns.
403. [GAP] 24h memory-leak soak (DR-051 + DR-054) — BP12+ scope per AGENTS.md staging note.

### J.15 DR-052 — Network sync rollback (M10/M11)
404. [GAP] `cfctl test sync-drift` — BP9+ owns.
405. [GAP] DR-052 client prediction + server reconciliation for player actor — BP10 owns.
406. [GAP] DR-052 lockstep input traces for online co-op — BP10 owns.
407. [GAP] DR-052 lag compensation — BP10 owns.

### J.16 DR-053 — AI audio pipeline (M4..M7 primary; BP6+ closes)
408. [GAP] DR-053 cf-audio integration with bevy_kira_audio — BP6 owns.
409. [GAP] DR-053 generative audio pipeline — BP12 owns.
410. [GAP] DR-053 usage-ledger + private/release mode check via `cf-asset-ledger check --mode private/release` — BP12 owns.

### J.17 DR-054 — Performance / profiling (M0..M12)
411. [GAP] DR-054 cf-bench regression harness — BP4+ wires; BP3 should have placeholder.
412. [GAP] DR-054 4K/120 perf gate per milestone — never measured.
413. [GAP] DR-054 1080p/60 perf gate per milestone — never measured.
414. [GAP] DR-054 Steam Deck 800p/60 perf gate per milestone — never measured.
415. [GAP] DR-054 SIMD material kernel update — M5.6+ owns.
416. [GAP] DR-054 GPU compute path for terrain carving — M5.6+ owns.

### J.18 DR-055 — Game feel / juice / flow state (every M1+ inherits)
417. [GAP] DR-055 recoil curves per weapon — M1+ scope; not authored.
418. [GAP] DR-055 camera punch on damage — M1+ scope; not implemented.
419. [GAP] DR-055 screen-space damage vignette — M1+ scope; not implemented.
420. [GAP] DR-055 hit-stop / hit-pause on impact — M5.5+ owns.
421. [GAP] DR-055 explosion camera shake — M5.5+ owns.

### J.19 DR-039 — Celestial bodies / worlds (M5.10/M7.7)
422. [GAP] DR-039 World catalog (`content/worlds/`) — directory missing.
423. [GAP] DR-039 per-planet astrography schema — not declared.
424. [GAP] DR-039 per-world ore deposit generator — M8.6 scope.

### J.20 DR-016 — Setting / world frame (BP4+ narrative)
425. [GAP] DR-016 launch worlds catalog (Earth/Mars/Phobos/Deimos/Moon/Mimas/Europa/Vulcan/Venus/Sol-zone habitats/belt asteroids/orbital stations) — BP5+ owns.
426. [GAP] DR-016 faction registry — BP7+ owns.

### J.21 spec/full-collision-physics-plan (M5.5 ATMOS-A-* and COLL-* tests)
427. [GAP] M5.5 ATMOS / COLL acceptance suites — see A.1 above.

### J.22 spec/atmospherics-and-chemistry-model (M5.9)
428. [GAP] M5.9 ATMOS-A acceptance suite — see B.1 above.

### J.23 spec/mining-and-extraction-model (M8.6)
429. [GAP] AI-MINE-A-01..AI-MINE-A-08 acceptance suite — see E.3 above.

### J.24 spec/persistent-mmo-architecture (M12)
430. [GAP] MMO-001..MMO-012 acceptance suite — see H.1 above.
