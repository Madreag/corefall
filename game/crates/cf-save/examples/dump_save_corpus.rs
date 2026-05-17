//! **M4B § "Migration corpus matrix passes for every fixture"** —
//! generator for the canonical save corpus fixtures.
//!
//! Run with:
//!
//! ```bash
//! cargo run -p cf-save --example dump_save_corpus
//! cargo run -p cf-save --example dump_save_corpus -- --check  # CI: fail on drift
//! ```
//!
//! Emits these files under `game/content/save_corpus/`:
//!
//! - `v1_minimal.cfsave`        — minimal v1 save (one zero-valued actor).
//! - `v1_minimal.cfsave.checksum`
//! - `v1_full_squad.cfsave`     — full-squad v1 save (4 actors, mod_payload extension).
//! - `v1_full_squad.cfsave.checksum`
//! - `v2_minimal.cfsave`        — v1_minimal migrated to v2 (the golden contract for v1->v2).
//! - `v2_minimal.cfsave.checksum`
//! - `tampered_chain.cfsave`    — deliberately corrupted save body whose
//!   .checksum sidecar still points at the pre-tamper hash.

use std::{collections::BTreeMap, fs, path::PathBuf};

use cf_save::{
    migration::migrate, SaveBlob, SaveSchemaVersion, WorldSave, V1_0_0, V2_0_0,
};

fn main() -> anyhow::Result<()> {
    let check = std::env::args().any(|a| a == "--check");
    let dir = corpus_dir()?;
    fs::create_dir_all(&dir)?;
    let mut drift = Vec::new();

    write_or_check(
        &dir,
        "v1_minimal.cfsave",
        &build_v1_minimal(),
        check,
        &mut drift,
    )?;
    write_or_check(
        &dir,
        "v1_full_squad.cfsave",
        &build_v1_full_squad(),
        check,
        &mut drift,
    )?;
    let v1_to_v2 = migrate(build_v1_minimal(), V2_0_0)?;
    write_or_check(&dir, "v2_minimal.cfsave", &v1_to_v2.blob, check, &mut drift)?;
    let tampered_dir = dir.join("tampered");
    fs::create_dir_all(&tampered_dir)?;
    let tampered_blob = build_v1_minimal();
    let (pretty, real_checksum) = tampered_blob.serialize()?;
    // Tamper the body AFTER computing the checksum so the on-disk
    // checksum sidecar references the pre-tamper hash. Loader MUST
    // detect this on read.
    let tampered_text = pretty.replace("\"hp\": 100.0", "\"hp\": 999.0");
    let tampered_path = dir.join("tampered_chain.cfsave");
    let tampered_checksum_path = dir.join("tampered_chain.cfsave.checksum");
    if check {
        let on_disk = fs::read_to_string(&tampered_path).unwrap_or_default();
        if on_disk != tampered_text {
            drift.push(tampered_path.display().to_string());
        }
        let on_disk_checksum = fs::read_to_string(&tampered_checksum_path).unwrap_or_default();
        if on_disk_checksum != real_checksum {
            drift.push(tampered_checksum_path.display().to_string());
        }
    } else {
        fs::write(&tampered_path, &tampered_text)?;
        fs::write(&tampered_checksum_path, &real_checksum)?;
        eprintln!("wrote {}", tampered_path.display());
        eprintln!("wrote {} (intentionally references pre-tamper hash)", tampered_checksum_path.display());
    }

    if check {
        if drift.is_empty() {
            println!("save corpus check OK");
            Ok(())
        } else {
            anyhow::bail!("save corpus drifted: {}", drift.join(", "));
        }
    } else {
        println!("OK: wrote save corpus to {}", dir.display());
        Ok(())
    }
}

fn corpus_dir() -> anyhow::Result<PathBuf> {
    // The example is `cargo run -p cf-save --example dump_save_corpus`; CWD
    // is the workspace root `game/`. Climb to `game/content/save_corpus`.
    let cwd = std::env::current_dir()?;
    let game_root = if cwd.file_name().and_then(|n| n.to_str()) == Some("game") {
        cwd
    } else {
        cwd.join("game")
    };
    Ok(game_root.join("content/save_corpus"))
}

fn write_or_check(
    dir: &std::path::Path,
    name: &str,
    save: &WorldSave,
    check: bool,
    drift: &mut Vec<String>,
) -> anyhow::Result<()> {
    let (pretty, checksum) = save.serialize()?;
    let path = dir.join(name);
    let checksum_path = dir.join(format!("{name}.checksum"));
    if check {
        let on_disk = fs::read_to_string(&path).unwrap_or_default();
        if on_disk != pretty {
            drift.push(path.display().to_string());
        }
        let on_disk_checksum = fs::read_to_string(&checksum_path).unwrap_or_default();
        if on_disk_checksum != checksum {
            drift.push(checksum_path.display().to_string());
        }
    } else {
        fs::write(&path, &pretty)?;
        fs::write(&checksum_path, &checksum)?;
        eprintln!("wrote {}", path.display());
    }
    Ok(())
}

fn build_v1_minimal() -> WorldSave {
    let actor = SaveBlob {
        schema_version: V1_0_0,
        actor_id: 1,
        team: "blue".to_string(),
        origin_id: "human".to_string(),
        position: [0.0, 0.0],
        velocity: [0.0, 0.0],
        aim: [1.0, 0.0],
        hp: 100.0,
        hp_max: 100.0,
        on_ground: true,
        status: "Stable".to_string(),
        selected_slot: 0,
        rifle_preset: None,
        rifle_ammo: None,
        rifle_reload_remaining_ticks: None,
        chassis: None,
        gear_dropped_by_limb_loss: false,
        chassis_detached: false,
        afflictions: vec![],
        crouch_active: false,
        climb_active: false,
        jet_active: false,
        mod_payload: BTreeMap::new(),
    };
    WorldSave {
        schema_version: V1_0_0,
        world_tick: 0,
        actors: vec![actor],
        terrain_chunks: vec![],
        projectiles: vec![],
        mod_payload: BTreeMap::new(),
    }
}

fn build_v1_full_squad() -> WorldSave {
    let mut squad = Vec::new();
    for i in 1u64..=4 {
        let mut mod_payload = BTreeMap::new();
        mod_payload.insert(
            "acme_corp.actor".to_string(),
            serde_json::json!({"squad_rank": format!("rank_{i}"), "loadout": "fireteam"}),
        );
        let _ = SaveSchemaVersion::new; // satisfy borrow checker for `use`
        let i_f = i as f32;
        squad.push(SaveBlob {
            schema_version: V1_0_0,
            actor_id: i,
            team: if i <= 2 { "blue".to_string() } else { "red".to_string() },
            origin_id: "human".to_string(),
            position: [i_f * 10.0, 0.0],
            velocity: [0.0, 0.0],
            aim: [1.0, 0.0],
            hp: 80.0 + i_f,
            hp_max: 100.0,
            on_ground: true,
            status: "Stable".to_string(),
            selected_slot: 0,
            rifle_preset: Some(cf_equipment::RIFLE_M1_DEFAULT_ID.to_string()),
            rifle_ammo: Some(20),
            rifle_reload_remaining_ticks: None,
            chassis: None,
            gear_dropped_by_limb_loss: false,
            chassis_detached: false,
            afflictions: if i == 4 { vec!["Bleeding".to_string()] } else { vec![] },
            crouch_active: i == 1,
            climb_active: false,
            jet_active: false,
            mod_payload,
        });
    }
    let mut world_mod_payload = BTreeMap::new();
    world_mod_payload.insert(
        "acme_corp.world".to_string(),
        serde_json::json!({"weather": "snow", "seed_offset": 7}),
    );
    WorldSave {
        schema_version: V1_0_0,
        world_tick: 600,
        actors: squad,
        terrain_chunks: vec![cf_save::TerrainChunkSnapshot {
            chunk_id: "0,0".to_string(),
            state: serde_json::json!({"materials": [1, 1, 2, 2, 3], "dirty": false}),
        }],
        projectiles: vec![cf_save::ProjectileSnapshot {
            id: 1,
            state: serde_json::json!({"pos": [10.0, 5.0], "vel": [50.0, 0.0], "ttl": 30}),
        }],
        mod_payload: world_mod_payload,
    }
}
