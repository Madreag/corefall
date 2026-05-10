# Cause Chain — `m3b_fixture_actor_died_chain`

### `mission_resolved` at tick 16 — `m3b_fixture_actor_died_chain:16:10`

Trigger payload: `{"reason":"all_red_actors_defeated","result":"won"}`

Cause chain (newest → oldest):

→ tick 16 `mission.mission_resolved` (`m3b_fixture_actor_died_chain:16:10`) `{"reason":"all_red_actors_defeated","result":"won"}`
  ↑ tick 15 `actor.actor_died` (`m3b_fixture_actor_died_chain:15:8`) `{"actor":2,"cause":"projectile","killed_by_actor":1,"position":[200.0,32.0]}`
    ↑ tick 15 `combat.projectile_hit` (`m3b_fixture_actor_died_chain:15:7`) `{"damage":100.0,"hit_position":[200.0,34.0],"projectile_id":1000,"shooter":1,"target":2}`
      ↑ tick 10 `combat.projectile_spawned` (`m3b_fixture_actor_died_chain:10:6`) `{"projectile_id":1000,"shooter":1,"velocity":[800.0,0.0]}`
        ↑ tick 10 `combat.weapon_fired` (`m3b_fixture_actor_died_chain:10:5`) `{"shooter":1,"weapon":"rifle"}`
          ↑ tick 10 `control.command_accepted` (`m3b_fixture_actor_died_chain:10:4`) `{"actor":1,"method":"act.player.fire"}`
            ↑ tick 0 `system.run_started` (`m3b_fixture_actor_died_chain:0:0`) `{"reason":"fixture","run_mode":"fixture","scenario":"actor_died_chain_fixture","seed":1,"tick_r…`

Chain depth: 7 · termination: root reached
