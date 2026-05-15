"""Per-category SFX synthesis recipes.

Each recipe is a pure function: (entry_dict, rng) -> float32 numpy buffer at
SAMPLE_RATE Hz mono, peak <= 1.0 prior to final normalization + fade.
Recipes consume the prompt manifest entry's fields (weapon_class / action /
material / stance / origin / damage_type / category / hazard / target / etc.)
and dispatch onto a per-feature synth.

The bake driver (sfx_bake.py) handles ADSR fade, peak normalization to the
manifest's target_peak_dbfs, and (for loops) loop-aligned crossfade.
"""

from __future__ import annotations

from typing import Callable, Dict, Optional

import numpy as np

from . import synth_primitives as sp


# ─── Weapons ────────────────────────────────────────────────────────────────


def _gun_report(
    rng: np.random.RandomState,
    dur: float,
    band_low: float,
    band_high: float,
    tau: float,
    sub_freq: float,
    sub_amp: float,
    reverb_decay: float,
    reverb_density: int,
    transient_amp: float = 0.95,
) -> np.ndarray:
    click = sp.transient_click(0.003, amp=transient_amp)
    noise = sp.band_filter(sp.white_noise(dur, 1.0, rng), band_low, band_high)
    env = sp.envelope_exp_decay(dur, tau=tau)
    noise = noise * env
    sub = sp.sine(min(dur, 0.18), sub_freq, sub_amp) * sp.envelope_exp_decay(min(dur, 0.18), tau=tau * 1.5)
    body = sp.mix(click, noise, sub, normalize=False)
    if reverb_decay > 0 and reverb_density > 0:
        body = sp.reverb_simple(body, decay=reverb_decay, density=reverb_density, rng=rng)
    return sp.ensure_duration(body, dur)


def _dry_click(rng: np.random.RandomState, dur: float) -> np.ndarray:
    click = sp.transient_click(0.004, amp=0.8)
    metal = sp.band_filter(sp.white_noise(dur, 0.5, rng), 1500, 4500) * sp.envelope_exp_decay(dur, tau=0.02)
    return sp.mix(click, metal, normalize=False)


def _mag_drop(rng: np.random.RandomState, dur: float, low: int = 200, high: int = 1500) -> np.ndarray:
    parts = []
    burst = sp.band_filter(sp.pink_noise(min(dur, 0.1), 0.6, rng), low, high)
    burst = burst * sp.envelope_exp_decay(len(burst) / sp.SAMPLE_RATE, tau=0.05)
    parts.append(burst)
    if dur > 0.2:
        thump = sp.band_filter(sp.white_noise(min(0.15, dur - 0.15), 0.5, rng), 100, 1000)
        thump = thump * sp.envelope_exp_decay(len(thump) / sp.SAMPLE_RATE, tau=0.05)
        offset = int(0.2 * sp.SAMPLE_RATE)
        base = np.zeros(sp._n_samples(dur))
        base[:len(parts[0])] += parts[0]
        if offset + len(thump) <= len(base):
            base[offset:offset + len(thump)] += thump
        return base
    return sp.ensure_duration(parts[0], dur)


def _charging_handle(rng: np.random.RandomState, dur: float) -> np.ndarray:
    click1 = sp.transient_click(0.002, amp=0.8)
    n1 = sp.band_filter(sp.pink_noise(0.04, 0.7, rng), 1000, 3000) * sp.envelope_exp_decay(0.04, tau=0.015)
    click2 = sp.transient_click(0.002, amp=0.9)
    n2 = sp.band_filter(sp.pink_noise(0.05, 0.8, rng), 1500, 4000) * sp.envelope_exp_decay(0.05, tau=0.02)
    base = np.zeros(sp._n_samples(dur))
    a = sp.mix(click1, n1, normalize=False)
    b = sp.mix(click2, n2, normalize=False)
    base[:len(a)] += a
    offset = int(min(0.18, dur * 0.4) * sp.SAMPLE_RATE)
    if offset + len(b) <= len(base):
        base[offset:offset + len(b)] += b
    return base


def synth_weapon(entry: Dict, rng: np.random.RandomState) -> np.ndarray:
    wclass = entry.get("weapon_class", "")
    action = entry.get("action", "")
    dur = float(entry.get("duration_target_sec", 0.4))

    if wclass == "pistol":
        if action == "fire":
            return _gun_report(rng, dur, 200, 8000, 0.022, 90.0, 0.35, 0.15, 8)
        if action == "dry_fire":
            return _dry_click(rng, dur)
        if action == "reload_start":
            return _mag_drop(rng, dur, 250, 1800)
        if action == "reload_end":
            return _charging_handle(rng, dur)
        if action == "jam":
            grind = sp.band_filter(sp.brown_noise(dur, 0.9, rng), 200, 3000) * sp.envelope_attack_decay(dur, 0.02, 0.1)
            tick = sp.transient_click(0.003, 0.6)
            return sp.mix(grind, tick, normalize=False)
        if action == "swap":
            cloth = sp.band_filter(sp.pink_noise(dur, 0.5, rng), 200, 1500) * sp.envelope_attack_decay(dur, 0.05, 0.2)
            clink = sp.band_filter(sp.white_noise(0.04, 0.7, rng), 2000, 5000) * sp.envelope_exp_decay(0.04, tau=0.02)
            base = np.zeros(sp._n_samples(dur))
            base[:len(cloth)] += cloth
            offset = int(dur * 0.7 * sp.SAMPLE_RATE)
            if offset + len(clink) <= len(base):
                base[offset:offset + len(clink)] += clink
            return base

    if wclass == "smg":
        if action == "fire_burst":
            base = np.zeros(sp._n_samples(dur))
            n_shots = max(3, int(dur / 0.12))
            for i in range(n_shots):
                offset = int(i * 0.1 * sp.SAMPLE_RATE)
                shot = _gun_report(rng, min(0.2, dur - i * 0.1), 250, 7500, 0.018, 95.0, 0.3, 0.18, 6, transient_amp=0.85)
                end = offset + len(shot)
                if end <= len(base):
                    base[offset:end] += shot * 0.8
            return base
        if action == "fire_auto":
            base = np.zeros(sp._n_samples(dur))
            n_shots = max(8, int(dur / 0.085))
            for i in range(n_shots):
                offset = int(i * 0.08 * sp.SAMPLE_RATE)
                jitter = float(rng.uniform(-30, 30))
                shot = _gun_report(rng, 0.18, 250 + jitter, 7500 + jitter, 0.015, 95.0, 0.28, 0.12, 4, transient_amp=0.75)
                end = offset + len(shot)
                if end <= len(base):
                    base[offset:end] += shot * 0.65
            return base
        if action == "reload_start":
            return _mag_drop(rng, dur, 200, 1500)
        if action == "reload_end":
            return _charging_handle(rng, dur)

    if wclass == "rifle":
        if action == "fire":
            return _gun_report(rng, dur, 300, 6000, 0.025, 100.0, 0.4, 0.25, 10)
        if action == "fire_auto":
            base = np.zeros(sp._n_samples(dur))
            n_shots = max(6, int(dur / 0.12))
            for i in range(n_shots):
                offset = int(i * 0.11 * sp.SAMPLE_RATE)
                jitter = float(rng.uniform(-40, 40))
                shot = _gun_report(rng, 0.25, 300 + jitter, 6000 + jitter, 0.022, 100.0, 0.38, 0.2, 8, transient_amp=0.8)
                end = offset + len(shot)
                if end <= len(base):
                    base[offset:end] += shot * 0.7
            return base
        if action == "reload_start":
            return _mag_drop(rng, dur, 200, 1500)
        if action == "reload_end":
            return _charging_handle(rng, dur)

    if wclass == "sniper":
        if action == "fire":
            return _gun_report(rng, dur, 100, 4000, 0.04, 70.0, 0.5, 0.6, 14)
        if action == "bolt_pull":
            base = np.zeros(sp._n_samples(dur))
            c1 = sp.transient_click(0.004, 0.8)
            n1 = sp.band_filter(sp.pink_noise(0.15, 0.7, rng), 1200, 3500) * sp.envelope_attack_decay(0.15, 0.02, 0.08)
            stroke = sp.mix(c1, n1, normalize=False)
            base[:len(stroke)] += stroke
            offset = int(0.6 * sp.SAMPLE_RATE)
            c2 = sp.transient_click(0.004, 0.9)
            n2 = sp.band_filter(sp.pink_noise(0.12, 0.6, rng), 1500, 4500) * sp.envelope_attack_decay(0.12, 0.02, 0.07)
            stroke2 = sp.mix(c2, n2, normalize=False)
            if offset + len(stroke2) <= len(base):
                base[offset:offset + len(stroke2)] += stroke2
            return base
        if action == "reload_start":
            return _mag_drop(rng, dur, 150, 1200)
        if action == "reload_end":
            return _mag_drop(rng, dur, 150, 1500)
        if action == "scope_settle":
            tick = sp.transient_click(0.003, 0.4)
            breath = sp.band_filter(sp.pink_noise(dur, 0.3, rng), 150, 800) * sp.envelope_attack_decay(dur, 0.05, 0.3)
            return sp.mix(tick, breath, normalize=False)

    if wclass == "shotgun":
        if action == "fire":
            base = _gun_report(rng, dur, 50, 4000, 0.05, 60.0, 0.6, 0.3, 10)
            sub = sp.sine(min(dur, 0.25), 80.0, 0.5) * sp.envelope_exp_decay(min(dur, 0.25), tau=0.08)
            base = sp.mix(base, sub, normalize=False)
            return base
        if action == "pump":
            base = np.zeros(sp._n_samples(dur))
            c1 = sp.transient_click(0.004, 0.85)
            n1 = sp.band_filter(sp.pink_noise(0.15, 0.7, rng), 800, 2500) * sp.envelope_attack_decay(0.15, 0.02, 0.06)
            stroke = sp.mix(c1, n1, normalize=False)
            base[:len(stroke)] += stroke
            offset = int(0.4 * sp.SAMPLE_RATE)
            c2 = sp.transient_click(0.004, 0.9)
            n2 = sp.band_filter(sp.pink_noise(0.18, 0.75, rng), 1000, 3500) * sp.envelope_attack_decay(0.18, 0.02, 0.07)
            stroke2 = sp.mix(c2, n2, normalize=False)
            if offset + len(stroke2) <= len(base):
                base[offset:offset + len(stroke2)] += stroke2
            return base
        if action == "reload_shell":
            click = sp.transient_click(0.003, 0.7)
            thunk = sp.band_filter(sp.pink_noise(dur, 0.6, rng), 200, 1200) * sp.envelope_exp_decay(dur, tau=0.06)
            return sp.mix(click, thunk, normalize=False)

    if wclass == "gl":
        if action == "fire":
            sub = sp.sine(dur, 80.0, 0.7) * sp.envelope_adsr(dur, 0.05, 0.05, 0.5, 0.15)
            whistle = sp.chirp(min(dur, 0.4), 800, 400, amp=0.4) * sp.envelope_attack_decay(min(dur, 0.4), 0.05, 0.1)
            base = np.zeros(sp._n_samples(dur))
            base[:len(sub)] += sub
            offset = int(0.1 * sp.SAMPLE_RATE)
            if offset + len(whistle) <= len(base):
                base[offset:offset + len(whistle)] += whistle
            return base
        if action == "cylinder_rotate":
            base = np.zeros(sp._n_samples(dur))
            n_clicks = 6
            for i in range(n_clicks):
                offset = int(i * (dur / n_clicks) * sp.SAMPLE_RATE)
                click = sp.transient_click(0.003, 0.55)
                if offset + len(click) <= len(base):
                    base[offset:offset + len(click)] += click
            ratchet = sp.band_filter(sp.pink_noise(dur, 0.3, rng), 1500, 4000) * sp.envelope_attack_decay(dur, 0.02, 0.15)
            base = base + ratchet
            return base
        if action == "reload_start":
            base = np.zeros(sp._n_samples(dur))
            c1 = sp.transient_click(0.005, 0.85)
            n1 = sp.band_filter(sp.pink_noise(0.4, 0.7, rng), 400, 2500) * sp.envelope_attack_decay(0.4, 0.05, 0.15)
            stroke = sp.mix(c1, n1, normalize=False)
            base[:len(stroke)] += stroke
            return base
        if action == "reload_end":
            thunk = sp.band_filter(sp.brown_noise(dur, 0.8, rng), 100, 1500) * sp.envelope_exp_decay(dur, tau=0.08)
            click = sp.transient_click(0.004, 0.7)
            return sp.mix(thunk, click, normalize=False)

    if wclass == "heavy":
        if action == "fire_burst":
            base = np.zeros(sp._n_samples(dur))
            n_shots = max(4, int(dur / 0.15))
            for i in range(n_shots):
                offset = int(i * 0.18 * sp.SAMPLE_RATE)
                shot = _gun_report(rng, 0.3, 50, 5000, 0.05, 50.0, 0.55, 0.35, 12, transient_amp=0.95)
                end = offset + len(shot)
                if end <= len(base):
                    base[offset:end] += shot * 0.8
            return base
        if action == "belt_feed":
            base = np.zeros(sp._n_samples(dur))
            for i in range(8):
                offset = int(i * (dur / 8.0) * sp.SAMPLE_RATE)
                click = sp.transient_click(0.003, 0.5)
                jitter = sp.band_filter(sp.pink_noise(0.04, 0.4, rng), 1500, 5000) * sp.envelope_exp_decay(0.04, tau=0.015)
                seg = sp.mix(click, jitter, normalize=False)
                end = offset + len(seg)
                if end <= len(base):
                    base[offset:end] += seg
            return base
        if action == "barrel_swap":
            release = sp.transient_click(0.005, 0.7)
            release2 = sp.band_filter(sp.pink_noise(0.2, 0.6, rng), 1000, 3500) * sp.envelope_attack_decay(0.2, 0.02, 0.1)
            hiss = sp.band_filter(sp.white_noise(dur, 0.6, rng), 3000, 10000) * sp.envelope_attack_decay(dur, 0.2, 0.6)
            base = np.zeros(sp._n_samples(dur))
            seg = sp.mix(release, release2, normalize=False)
            base[:len(seg)] += seg
            base += hiss * 0.7
            removal = sp.band_filter(sp.brown_noise(0.4, 0.5, rng), 100, 1500) * sp.envelope_attack_decay(0.4, 0.05, 0.15)
            offset = int(1.5 * sp.SAMPLE_RATE)
            if offset + len(removal) <= len(base):
                base[offset:offset + len(removal)] += removal
            return base

    if wclass == "flamer":
        if action == "fire":
            roar = sp.band_filter(sp.brown_noise(dur, 0.9, rng), 200, 3000)
            roar = sp.amplitude_lfo(roar, rate_hz=3.0, depth=0.3)
            roar = roar * sp.envelope_attack_decay(dur, 0.05, 0.5)
            crackle = sp.band_filter(sp.pink_noise(dur, 0.5, rng), 1000, 5000)
            crackle = crackle * sp.crackle_pattern(dur, density_per_sec=20.0, peak=0.7, rng=rng)
            return sp.mix(roar, crackle, normalize=False)
        if action == "ignite":
            click = sp.transient_click(0.003, 0.7)
            whoof = sp.band_filter(sp.brown_noise(dur, 0.8, rng), 100, 2500) * sp.envelope_attack_decay(dur, 0.1, 0.2)
            return sp.mix(click, whoof, normalize=False)
        if action == "tank_refill":
            gurgle = sp.band_filter(sp.brown_noise(dur, 0.7, rng), 80, 1200)
            gurgle = sp.amplitude_lfo(gurgle, rate_hz=2.0, depth=0.4)
            gurgle = gurgle * sp.envelope_attack_decay(dur, 0.1, 0.7)
            return gurgle

    if wclass == "drill":
        if action == "spin":
            base = sp.fm_synth(dur, carrier_hz=200.0, mod_hz=20.0, mod_index=8.0, amp=0.5)
            ramp = np.linspace(0.0, 1.0, sp._n_samples(dur))
            base = base * ramp
            whir = sp.band_filter(sp.pink_noise(dur, 0.4, rng), 800, 3000) * ramp
            return sp.mix(base, whir, normalize=False)
        if action == "into_metal":
            base = sp.fm_synth(dur, carrier_hz=300.0, mod_hz=40.0, mod_index=12.0, amp=0.5)
            shriek = sp.band_filter(sp.white_noise(dur, 0.6, rng), 3000, 9000) * sp.envelope_attack_decay(dur, 0.1, 0.8)
            sparks = sp.crackle_pattern(dur, density_per_sec=15.0, peak=0.5, rng=rng)
            return sp.mix(base, shriek, sparks, normalize=False)
        if action == "into_concrete":
            base = sp.fm_synth(dur, carrier_hz=200.0, mod_hz=25.0, mod_index=10.0, amp=0.4)
            grind = sp.band_filter(sp.pink_noise(dur, 0.7, rng), 500, 3000)
            grind = sp.amplitude_lfo(grind, rate_hz=8.0, depth=0.3)
            grind = grind * sp.envelope_attack_decay(dur, 0.1, 0.8)
            return sp.mix(base, grind, normalize=False)
        if action == "into_dirt":
            base = sp.fm_synth(dur, carrier_hz=150.0, mod_hz=18.0, mod_index=8.0, amp=0.35)
            thud = sp.band_filter(sp.brown_noise(dur, 0.6, rng), 80, 800)
            thud = sp.amplitude_lfo(thud, rate_hz=5.0, depth=0.4)
            thud = thud * sp.envelope_attack_decay(dur, 0.1, 0.6)
            return sp.mix(base, thud, normalize=False)

    if wclass == "grappler":
        if action == "fire":
            twang = sp.sine(min(dur, 0.2), 600.0, 0.6) * sp.envelope_exp_decay(min(dur, 0.2), tau=0.04)
            whir = sp.band_filter(sp.pink_noise(dur, 0.5, rng), 400, 2500) * sp.envelope_attack_decay(dur, 0.05, 0.15)
            return sp.mix(twang, whir, normalize=False)
        if action == "anchor":
            clink = sp.band_filter(sp.white_noise(0.05, 0.8, rng), 1500, 4500) * sp.envelope_exp_decay(0.05, tau=0.02)
            ring = sp.sine(0.15, 1200.0, 0.4) * sp.envelope_exp_decay(0.15, tau=0.05)
            base = sp.ensure_duration(sp.mix(clink, ring, normalize=False), dur)
            return base
        if action == "reel":
            base = np.zeros(sp._n_samples(dur))
            ticks = max(8, int(dur / 0.06))
            for i in range(ticks):
                offset = int(i * (dur / ticks) * sp.SAMPLE_RATE)
                click = sp.transient_click(0.002, 0.4)
                if offset + len(click) <= len(base):
                    base[offset:offset + len(click)] += click
            motor = sp.band_filter(sp.pink_noise(dur, 0.3, rng), 500, 2000) * sp.envelope_attack_decay(dur, 0.05, 0.4)
            return base + motor

    if wclass == "drone_deployer":
        if action == "deploy":
            whir = sp.band_filter(sp.pink_noise(dur, 0.6, rng), 1500, 5000) * sp.envelope_attack_decay(dur, 0.1, 0.2)
            spool = sp.chirp(dur, 200.0, 600.0, amp=0.4) * sp.envelope_attack_decay(dur, 0.05, 0.15)
            return sp.mix(whir, spool, normalize=False)
        if action == "hover":
            base = sp.sine(dur, 240.0, 0.4)
            base = base + sp.sine(dur, 360.0, 0.25)
            base = base + sp.sine(dur, 480.0, 0.18)
            base = base + sp.fm_synth(dur, 200.0, 12.0, 4.0, amp=0.25)
            base = sp.amplitude_lfo(base, rate_hz=8.0, depth=0.15)
            whir = sp.band_filter(sp.pink_noise(dur, 0.3, rng), 1000, 4000)
            return base + whir
        if action == "target_lock":
            c1 = sp.sine(0.12, 1200.0, 0.5) * sp.envelope_adsr(0.12, 0.005, 0.02, 0.7, 0.04)
            c2 = sp.sine(0.12, 1800.0, 0.5) * sp.envelope_adsr(0.12, 0.005, 0.02, 0.7, 0.04)
            base = np.zeros(sp._n_samples(dur))
            base[:len(c1)] += c1
            offset = int(0.18 * sp.SAMPLE_RATE)
            if offset + len(c2) <= len(base):
                base[offset:offset + len(c2)] += c2
            return base

    if wclass == "melee":
        if action.endswith("_swing"):
            swing = sp.band_filter(sp.white_noise(dur, 0.6, rng), 500, 4000)
            env = sp.envelope_attack_decay(dur, 0.05, 0.15)
            return swing * env
        if action.endswith("_hit") or action in ("kick", "shoulder_check", "rifle_bash"):
            wet = sp.band_filter(sp.brown_noise(0.15, 0.8, rng), 80, 1200) * sp.envelope_exp_decay(0.15, tau=0.05)
            thump = sp.sine(0.1, 120.0, 0.6) * sp.envelope_exp_decay(0.1, tau=0.04)
            if action == "baton_hit":
                ring = sp.sine(0.2, 1500.0, 0.4) * sp.envelope_exp_decay(0.2, tau=0.05)
                return sp.ensure_duration(sp.mix(wet, thump, ring, normalize=False), dur)
            if action == "knife_hit":
                squelch = sp.band_filter(sp.brown_noise(0.1, 0.7, rng), 200, 1800) * sp.envelope_exp_decay(0.1, tau=0.03)
                return sp.ensure_duration(sp.mix(wet, squelch, normalize=False), dur)
            if action == "hatchet_hit":
                crack = sp.transient_click(0.003, 0.85)
                splinter = sp.band_filter(sp.pink_noise(0.08, 0.6, rng), 1500, 5000) * sp.envelope_exp_decay(0.08, tau=0.03)
                return sp.ensure_duration(sp.mix(wet, thump, crack, splinter, normalize=False), dur)
            return sp.ensure_duration(sp.mix(wet, thump, normalize=False), dur)

    if wclass == "grenade":
        if action == "pin_pull":
            click = sp.transient_click(0.003, 0.8)
            spring = sp.band_filter(sp.pink_noise(0.1, 0.5, rng), 2000, 5000) * sp.envelope_exp_decay(0.1, tau=0.03)
            return sp.ensure_duration(sp.mix(click, spring, normalize=False), dur)
        if action == "throw":
            rustle = sp.band_filter(sp.pink_noise(dur, 0.5, rng), 300, 2000) * sp.envelope_attack_decay(dur, 0.05, 0.2)
            whoosh = sp.band_filter(sp.white_noise(0.2, 0.4, rng), 500, 3000) * sp.envelope_attack_decay(0.2, 0.05, 0.1)
            base = np.zeros(sp._n_samples(dur))
            base[:len(rustle)] += rustle
            offset = int(0.25 * sp.SAMPLE_RATE)
            if offset + len(whoosh) <= len(base):
                base[offset:offset + len(whoosh)] += whoosh
            return base
        if action == "bounce":
            base = np.zeros(sp._n_samples(dur))
            for i in range(4):
                t = (i * 0.22)
                if t >= dur:
                    break
                offset = int(t * sp.SAMPLE_RATE)
                clink = sp.band_filter(sp.white_noise(0.04, 0.6, rng), 1500, 4000) * sp.envelope_exp_decay(0.04, tau=0.015)
                end = offset + len(clink)
                if end <= len(base):
                    base[offset:end] += clink * (1.0 - i * 0.18)
            return base
        if action == "explode_frag":
            boom = sp.sine(min(dur, 0.5), 60.0, 0.85) * sp.envelope_exp_decay(min(dur, 0.5), tau=0.15)
            crack = sp.transient_click(0.003, 0.95)
            noise = sp.band_filter(sp.white_noise(dur, 1.0, rng), 100, 6000) * sp.envelope_exp_decay(dur, tau=0.2)
            shrap = sp.band_filter(sp.white_noise(dur, 0.5, rng), 2000, 9000) * sp.envelope_attack_decay(dur, 0.05, 0.25)
            body = sp.mix(boom, crack, noise, shrap, normalize=False)
            return sp.reverb_simple(body, decay=0.6, density=14, rng=rng)
        if action == "smoke_pop":
            hiss = sp.band_filter(sp.white_noise(dur, 0.7, rng), 1500, 8000) * sp.envelope_attack_decay(dur, 0.1, 0.8)
            fizz = sp.band_filter(sp.pink_noise(dur, 0.4, rng), 800, 4000) * sp.envelope_attack_decay(dur, 0.1, 0.7)
            return sp.mix(hiss, fizz, normalize=False)
        if action == "flash_pop":
            crack = sp.transient_click(0.002, 0.95)
            noise = sp.band_filter(sp.white_noise(dur, 1.0, rng), 200, 9000) * sp.envelope_exp_decay(dur, tau=0.12)
            ring = sp.sine(dur, 6000.0, 0.4) * sp.envelope_exp_decay(dur, tau=0.2)
            return sp.mix(crack, noise, ring, normalize=False)
        if action == "stick_attach":
            squelch = sp.band_filter(sp.brown_noise(dur, 0.7, rng), 150, 1500) * sp.envelope_exp_decay(dur, tau=0.08)
            plop = sp.sine(0.1, 200.0, 0.5) * sp.envelope_exp_decay(0.1, tau=0.04)
            return sp.ensure_duration(sp.mix(squelch, plop, normalize=False), dur)

    return _generic_placeholder(rng, dur)


# ─── Movement ───────────────────────────────────────────────────────────────


def _footstep_human_base(rng: np.random.RandomState, dur: float, material: str) -> np.ndarray:
    if material == "concrete":
        noise = sp.band_filter(sp.pink_noise(dur, 0.7, rng), 200, 2000) * sp.envelope_exp_decay(dur, tau=0.02)
        thump = sp.sine(min(dur, 0.04), 100.0, 0.5) * sp.envelope_exp_decay(min(dur, 0.04), tau=0.015)
    elif material == "dirt":
        noise = sp.band_filter(sp.brown_noise(dur, 0.75, rng), 100, 800) * sp.envelope_exp_decay(dur, tau=0.04)
        thump = sp.sine(min(dur, 0.05), 90.0, 0.4) * sp.envelope_exp_decay(min(dur, 0.05), tau=0.025)
    elif material == "metal":
        noise = sp.band_filter(sp.pink_noise(dur, 0.7, rng), 1500, 5000) * sp.envelope_exp_decay(dur, tau=0.012)
        thump = sp.sine(min(dur, 0.04), 800.0, 0.4) * sp.envelope_exp_decay(min(dur, 0.04), tau=0.02)
    elif material == "wood":
        noise = sp.band_filter(sp.pink_noise(dur, 0.6, rng), 200, 1500) * sp.envelope_exp_decay(dur, tau=0.03)
        creak = sp.sine(min(dur, 0.08), 80.0, 0.45) * sp.envelope_attack_decay(min(dur, 0.08), 0.01, 0.03)
        thump = creak
    elif material == "sand":
        noise = sp.band_filter(sp.pink_noise(dur, 0.65, rng), 500, 3000) * sp.envelope_attack_decay(dur, 0.02, 0.06)
        thump = np.zeros_like(noise)
    elif material == "snow":
        noise = sp.band_filter(sp.pink_noise(dur, 0.7, rng), 4000, 12000) * sp.envelope_attack_decay(dur, 0.02, 0.06)
        thump = sp.band_filter(sp.brown_noise(min(dur, 0.05), 0.4, rng), 100, 1000) * sp.envelope_exp_decay(min(dur, 0.05), tau=0.03)
    elif material == "ice":
        noise = sp.band_filter(sp.pink_noise(dur, 0.65, rng), 2000, 8000) * sp.envelope_exp_decay(dur, tau=0.012)
        thump = sp.sine(min(dur, 0.03), 600.0, 0.4) * sp.envelope_exp_decay(min(dur, 0.03), tau=0.01)
    elif material == "water":
        noise = sp.band_filter(sp.brown_noise(dur, 0.7, rng), 100, 3000) * sp.envelope_attack_decay(dur, 0.02, 0.06)
        bubbles = np.zeros_like(noise)
        for offset_sec in (0.05, 0.1, 0.15):
            if offset_sec < dur:
                start = int(offset_sec * sp.SAMPLE_RATE)
                tone = sp.sine(0.02, 800.0, 0.4) * sp.envelope_exp_decay(0.02, tau=0.008)
                if start + len(tone) <= len(bubbles):
                    bubbles[start:start + len(tone)] += tone
        thump = bubbles
    elif material == "mud":
        noise = sp.band_filter(sp.brown_noise(dur, 0.7, rng), 80, 600) * sp.envelope_attack_decay(dur, 0.03, 0.1)
        suck = sp.pitch_envelope(min(dur, 0.15), 400.0, 100.0, curve="exp", amp=0.4) * sp.envelope_attack_decay(min(dur, 0.15), 0.05, 0.06)
        thump = suck
    elif material == "oil":
        noise = sp.band_filter(sp.brown_noise(dur, 0.65, rng), 100, 1500) * sp.envelope_attack_decay(dur, 0.03, 0.1)
        thump = sp.sine(min(dur, 0.05), 90.0, 0.4) * sp.envelope_exp_decay(min(dur, 0.05), tau=0.04)
    elif material == "acid":
        noise = sp.band_filter(sp.white_noise(dur, 0.7, rng), 1000, 8000) * sp.envelope_attack_decay(dur, 0.03, 0.1)
        splash = sp.band_filter(sp.brown_noise(min(dur, 0.1), 0.6, rng), 200, 2000) * sp.envelope_exp_decay(min(dur, 0.1), tau=0.04)
        thump = splash
    elif material == "lava":
        noise = sp.band_filter(sp.brown_noise(dur, 0.75, rng), 200, 3000) * sp.envelope_attack_decay(dur, 0.05, 0.2)
        sizzle = sp.band_filter(sp.white_noise(dur, 0.4, rng), 5000, 12000) * sp.envelope_attack_decay(dur, 0.05, 0.3)
        thump = sizzle
    elif material == "alien_resin":
        noise = sp.band_filter(sp.brown_noise(dur, 0.6, rng), 150, 1500) * sp.envelope_attack_decay(dur, 0.05, 0.25)
        squelch = sp.pitch_envelope(min(dur, 0.3), 600.0, 200.0, curve="exp", amp=0.4) * sp.envelope_attack_decay(min(dur, 0.3), 0.05, 0.15)
        thump = squelch
    else:
        noise = sp.band_filter(sp.pink_noise(dur, 0.6, rng), 200, 2500) * sp.envelope_exp_decay(dur, tau=0.03)
        thump = sp.sine(min(dur, 0.05), 110.0, 0.4) * sp.envelope_exp_decay(min(dur, 0.05), tau=0.025)
    return sp.mix(noise, thump, normalize=False)


def _stance_adjust(samples: np.ndarray, stance: str) -> np.ndarray:
    if stance in ("crouching",):
        return sp.low_pass(samples, 1500.0) * 0.55
    if stance == "running":
        return samples * 1.05
    if stance == "sprinting":
        return samples * 1.15
    if stance == "slip":
        return sp.amplitude_lfo(samples, 3.0, 0.5)
    return samples


def _origin_adjust(rng: np.random.RandomState, samples: np.ndarray, origin: str, dur: float) -> np.ndarray:
    if origin == "robot":
        ring = sp.sine(min(dur, 0.06), 1200.0, 0.3) * sp.envelope_exp_decay(min(dur, 0.06), tau=0.02)
        hiss = sp.band_filter(sp.white_noise(min(dur, 0.08), 0.4, rng), 200, 1500) * sp.envelope_attack_decay(min(dur, 0.08), 0.02, 0.04)
        return sp.mix(samples, ring, hiss, normalize=False)
    if origin == "heavy_biomech":
        thud = sp.sine(min(dur, 0.12), 60.0, 0.7) * sp.envelope_exp_decay(min(dur, 0.12), tau=0.06)
        squelch = sp.band_filter(sp.brown_noise(min(dur, 0.1), 0.5, rng), 300, 1500) * sp.envelope_attack_decay(min(dur, 0.1), 0.02, 0.05)
        return sp.mix(samples, thud, squelch, normalize=False)
    if origin == "aqueous":
        bubble = sp.band_filter(sp.white_noise(dur, 0.5, rng), 1000, 6000) * sp.envelope_attack_decay(dur, 0.05, 0.1)
        return sp.mix(samples, bubble, normalize=False)
    if origin == "insectoid":
        base = np.zeros(sp._n_samples(dur))
        for i in range(4):
            offset = int(i * 0.02 * sp.SAMPLE_RATE)
            click = sp.band_filter(sp.pink_noise(0.02, 0.6, rng), 2000, 8000) * sp.envelope_exp_decay(0.02, tau=0.008)
            end = offset + len(click)
            if end <= len(base):
                base[offset:end] += click
        return sp.mix(samples * 0.5, base, normalize=False)
    return samples


def synth_footstep(entry: Dict, rng: np.random.RandomState) -> np.ndarray:
    dur = float(entry.get("duration_target_sec", 0.2))
    material = entry.get("material", "concrete")
    stance = entry.get("stance", "walking")
    origin = entry.get("origin", "human")
    base = _footstep_human_base(rng, dur, material)
    base = _stance_adjust(base, stance)
    base = _origin_adjust(rng, base, origin, dur)
    return base


def synth_locomotion(entry: Dict, rng: np.random.RandomState) -> np.ndarray:
    action = entry.get("action", "")
    dur = float(entry.get("duration_target_sec", 0.4))

    if action == "jump":
        grunt = sp.voice_formant(dur, f0=180.0, formants=[400, 800, 1500], vibrato_hz=4.0, vibrato_depth_hz=3.0, rng=rng) * sp.envelope_attack_decay(dur, 0.05, 0.2)
        cloth = sp.band_filter(sp.pink_noise(dur, 0.4, rng), 200, 1500) * sp.envelope_attack_decay(dur, 0.02, 0.08)
        return sp.mix(grunt, cloth, normalize=False)
    if action == "land_soft":
        thud = sp.sine(min(dur, 0.12), 80.0, 0.8) * sp.envelope_exp_decay(min(dur, 0.12), tau=0.05)
        body = sp.band_filter(sp.pink_noise(dur, 0.5, rng), 100, 800) * sp.envelope_exp_decay(dur, tau=0.06)
        return sp.mix(thud, body, normalize=False)
    if action == "land_hard":
        thud = sp.sine(min(dur, 0.25), 60.0, 0.95) * sp.envelope_exp_decay(min(dur, 0.25), tau=0.1)
        grunt = sp.voice_formant(min(dur, 0.3), f0=150.0, formants=[400, 900, 1500], rng=rng) * sp.envelope_attack_decay(min(dur, 0.3), 0.05, 0.1)
        clatter = sp.band_filter(sp.pink_noise(dur, 0.4, rng), 800, 3500) * sp.envelope_exp_decay(dur, tau=0.07)
        return sp.mix(thud, grunt, clatter, normalize=False)
    if action == "jet_ignite":
        whoosh = sp.chirp(min(dur, 0.4), 100, 800, amp=0.7) * sp.envelope_attack_decay(min(dur, 0.4), 0.05, 0.15)
        roar = sp.band_filter(sp.brown_noise(dur, 0.85, rng), 60, 2000) * sp.envelope_attack_decay(dur, 0.1, 0.5)
        return sp.mix(whoosh, roar, normalize=False)
    if action == "jet_hover":
        roar = sp.band_filter(sp.brown_noise(dur, 0.85, rng), 80, 3000)
        roar = sp.amplitude_lfo(roar, rate_hz=0.5, depth=0.15)
        fund = sp.sine(dur, 120.0, 0.35)
        harm1 = sp.sine(dur, 240.0, 0.22)
        harm2 = sp.sine(dur, 360.0, 0.15)
        return sp.mix(roar, fund, harm1, harm2, normalize=False)
    if action == "jet_cutoff":
        descent = sp.pitch_envelope(dur, 800.0, 100.0, curve="exp", amp=0.5) * sp.envelope_attack_decay(dur, 0.05, 0.3)
        roar = sp.band_filter(sp.brown_noise(dur, 0.6, rng), 80, 2500) * sp.envelope_attack_decay(dur, 0.05, 0.3)
        return sp.mix(descent, roar, normalize=False)
    if action == "slide":
        friction = sp.band_filter(sp.brown_noise(dur, 0.8, rng), 200, 2000)
        env = sp.envelope_adsr(dur, 0.05, 0.2, 0.6, 0.15)
        friction = friction * env
        whine = sp.sine(dur, 200.0, 0.3) * sp.envelope_attack_decay(dur, 0.1, 0.3)
        return sp.mix(friction, whine, normalize=False)
    if action == "climb_ladder":
        ring = sp.sine(min(dur, 0.05), 1000.0, 0.55) * sp.envelope_exp_decay(min(dur, 0.05), tau=0.02)
        rustle = sp.band_filter(sp.white_noise(dur, 0.4, rng), 300, 1500) * sp.envelope_attack_decay(dur, 0.02, 0.1)
        return sp.mix(ring, rustle, normalize=False)
    if action == "climb_rope":
        creak = sp.sine(min(dur, 0.1), 150.0, 0.4) * sp.envelope_attack_decay(min(dur, 0.1), 0.02, 0.05)
        rustle = sp.band_filter(sp.pink_noise(dur, 0.5, rng), 200, 2000) * sp.envelope_attack_decay(dur, 0.02, 0.1)
        return sp.mix(creak, rustle, normalize=False)
    if action == "climb_pipe":
        ring = sp.sine(min(dur, 0.05), 1500.0, 0.5) * sp.envelope_exp_decay(min(dur, 0.05), tau=0.025)
        rub = sp.band_filter(sp.pink_noise(dur, 0.45, rng), 500, 2500) * sp.envelope_attack_decay(dur, 0.02, 0.1)
        return sp.mix(ring, rub, normalize=False)
    if action == "vault":
        slap = sp.band_filter(sp.pink_noise(0.04, 0.7, rng), 200, 2500) * sp.envelope_exp_decay(0.04, tau=0.015)
        swing = sp.band_filter(sp.white_noise(min(dur, 0.2), 0.5, rng), 400, 3000) * sp.envelope_attack_decay(min(dur, 0.2), 0.05, 0.1)
        thud = sp.sine(min(dur, 0.12), 90.0, 0.6) * sp.envelope_exp_decay(min(dur, 0.12), tau=0.05)
        base = np.zeros(sp._n_samples(dur))
        base[:len(slap)] += slap
        offset_s = int(0.15 * sp.SAMPLE_RATE)
        if offset_s + len(swing) <= len(base):
            base[offset_s:offset_s + len(swing)] += swing
        offset_t = int(min(dur - 0.15, 0.4) * sp.SAMPLE_RATE)
        offset_t = max(0, offset_t)
        if offset_t + len(thud) <= len(base):
            base[offset_t:offset_t + len(thud)] += thud
        return base
    if action == "dive_prone":
        slam = sp.sine(min(dur, 0.18), 70.0, 0.9) * sp.envelope_exp_decay(min(dur, 0.18), tau=0.08)
        body = sp.band_filter(sp.pink_noise(dur, 0.6, rng), 150, 2000) * sp.envelope_exp_decay(dur, tau=0.1)
        grunt = sp.voice_formant(min(dur, 0.3), f0=160.0, formants=[400, 800, 1500], rng=rng) * sp.envelope_attack_decay(min(dur, 0.3), 0.05, 0.1)
        return sp.mix(slam, body, grunt, normalize=False)
    if action in ("stance_stand", "stance_crouch"):
        rustle = sp.band_filter(sp.white_noise(dur, 0.5, rng), 200, 1500) * sp.envelope_attack_decay(dur, 0.05, 0.25)
        creak = sp.sine(min(dur, 0.08), 200.0, 0.3) * sp.envelope_attack_decay(min(dur, 0.08), 0.02, 0.04)
        return sp.mix(rustle, creak, normalize=False)
    if action == "stance_prone":
        settle = sp.band_filter(sp.pink_noise(dur, 0.55, rng), 100, 1500) * sp.envelope_attack_decay(dur, 0.05, 0.4)
        grunt = sp.voice_formant(min(dur, 0.3), f0=160.0, formants=[300, 700, 1200], rng=rng) * sp.envelope_attack_decay(min(dur, 0.3), 0.05, 0.15)
        return sp.mix(settle, grunt, normalize=False)
    if action == "stamina_breath":
        out = np.zeros(sp._n_samples(dur))
        cycles = max(2, int(dur / 0.6))
        for i in range(cycles):
            start = int(i * 0.6 * sp.SAMPLE_RATE)
            in_breath = sp.band_filter(sp.white_noise(0.25, 0.6, rng), 200, 2500) * sp.envelope_attack_decay(0.25, 0.05, 0.1)
            out_breath = sp.band_filter(sp.white_noise(0.25, 0.5, rng), 150, 2000) * sp.envelope_attack_decay(0.25, 0.05, 0.1)
            if start + len(in_breath) <= len(out):
                out[start:start + len(in_breath)] += in_breath
            mid = start + int(0.3 * sp.SAMPLE_RATE)
            if mid + len(out_breath) <= len(out):
                out[mid:mid + len(out_breath)] += out_breath
        return out
    if action == "chassis_walk":
        servo = sp.sine(dur, 240.0, 0.4) * sp.amplitude_lfo(np.ones(sp._n_samples(dur)), rate_hz=2.0, depth=0.6)
        step = np.zeros(sp._n_samples(dur))
        ticks = max(2, int(dur / 0.5))
        for i in range(ticks):
            offset = int(i * 0.5 * sp.SAMPLE_RATE)
            thump = sp.sine(0.1, 80.0, 0.7) * sp.envelope_exp_decay(0.1, tau=0.05)
            end = offset + len(thump)
            if end <= len(step):
                step[offset:end] += thump
        whir = sp.band_filter(sp.pink_noise(dur, 0.3, rng), 300, 2500)
        return sp.mix(servo, step, whir, normalize=False)
    if action == "chassis_turn":
        pivot = sp.band_filter(sp.brown_noise(dur, 0.6, rng), 80, 800) * sp.envelope_attack_decay(dur, 0.1, 0.3)
        creak = sp.sine(dur, 200.0, 0.35) * sp.envelope_attack_decay(dur, 0.1, 0.2)
        return sp.mix(pivot, creak, normalize=False)
    if action == "eject_pilot":
        crack = sp.transient_click(0.005, 0.95)
        rocket = sp.band_filter(sp.brown_noise(dur, 0.85, rng), 80, 2500) * sp.envelope_attack_decay(dur, 0.05, 0.5)
        rocket_fund = sp.sine(dur, 120.0, 0.4) * sp.envelope_attack_decay(dur, 0.05, 0.4)
        glass = sp.band_filter(sp.white_noise(0.4, 0.7, rng), 3000, 12000) * sp.envelope_exp_decay(0.4, tau=0.08)
        base = sp.mix(crack, rocket, rocket_fund, normalize=False)
        base = sp.ensure_duration(base, dur)
        glass_offset = int(0.3 * sp.SAMPLE_RATE)
        if glass_offset + len(glass) <= len(base):
            base[glass_offset:glass_offset + len(glass)] += glass
        return base

    return _generic_placeholder(rng, dur)


# ─── Impact / Combat ────────────────────────────────────────────────────────


def _impact_kinetic(rng: np.random.RandomState, dur: float, material: str, intensity: str) -> np.ndarray:
    big = intensity == "large"
    if material == "concrete":
        body = sp.band_filter(sp.pink_noise(dur, 0.8, rng), 200, 3000) * sp.envelope_exp_decay(dur, tau=0.04 if big else 0.025)
        dust = sp.band_filter(sp.brown_noise(dur, 0.5, rng), 50, 800) * sp.envelope_attack_decay(dur, 0.05, 0.15)
        sub = sp.sine(min(dur, 0.15), 70.0 if big else 110.0, 0.6 if big else 0.4) * sp.envelope_exp_decay(min(dur, 0.15), tau=0.06)
        return sp.mix(body, dust, sub, normalize=False)
    if material == "metal":
        ping1 = sp.sine(min(dur, 0.15), 2000.0, 0.7) * sp.envelope_exp_decay(min(dur, 0.15), tau=0.05)
        ping2 = sp.sine(min(dur, 0.15), 3500.0, 0.5) * sp.envelope_exp_decay(min(dur, 0.15), tau=0.04)
        ping3 = sp.sine(min(dur, 0.15), 5000.0, 0.4) * sp.envelope_exp_decay(min(dur, 0.15), tau=0.03)
        ricochet = sp.band_filter(sp.white_noise(dur, 0.6, rng), 1500, 8000) * sp.envelope_exp_decay(dur, tau=0.08)
        if big:
            boom = sp.sine(min(dur, 0.3), 80.0, 0.8) * sp.envelope_exp_decay(min(dur, 0.3), tau=0.1)
            return sp.mix(ping1, ping2, ping3, ricochet, boom, normalize=False)
        return sp.mix(ping1, ping2, ping3, ricochet, normalize=False)
    if material == "wood":
        body = sp.band_filter(sp.pink_noise(dur, 0.7, rng), 300, 3000) * sp.envelope_exp_decay(dur, tau=0.03)
        thunk = sp.sine(min(dur, 0.12), 200.0, 0.6) * sp.envelope_exp_decay(min(dur, 0.12), tau=0.05)
        splinter = sp.band_filter(sp.pink_noise(dur, 0.4, rng), 2500, 7000) * sp.envelope_exp_decay(dur, tau=0.05 if big else 0.02)
        return sp.mix(body, thunk, splinter, normalize=False)
    if material == "glass":
        crack = sp.transient_click(0.003, 0.9)
        shatter = sp.band_filter(sp.white_noise(dur, 0.85, rng), 2000, 12000) * sp.envelope_exp_decay(dur, tau=0.18)
        base = sp.mix(crack, shatter, normalize=False)
        for offset_sec in (0.05, 0.15, 0.3):
            if offset_sec < dur:
                offset = int(offset_sec * sp.SAMPLE_RATE)
                shard = sp.band_filter(sp.white_noise(0.06, 0.5, rng), 3000, 10000) * sp.envelope_exp_decay(0.06, tau=0.02)
                end = offset + len(shard)
                if end <= len(base):
                    base[offset:end] += shard
        return base
    if material == "dirt":
        body = sp.band_filter(sp.brown_noise(dur, 0.7, rng), 50, 500) * sp.envelope_exp_decay(dur, tau=0.04 if big else 0.025)
        puff = sp.band_filter(sp.brown_noise(dur, 0.4, rng), 100, 1000) * sp.envelope_attack_decay(dur, 0.05, 0.2)
        return sp.mix(body, puff, normalize=False)
    if material == "water":
        body = sp.band_filter(sp.white_noise(dur, 0.75, rng), 1000, 8000) * sp.envelope_exp_decay(dur, tau=0.06)
        for offset_sec in (0.1, 0.2, 0.3):
            if offset_sec < dur:
                offset = int(offset_sec * sp.SAMPLE_RATE)
                drop = sp.sine(0.03, 1500.0, 0.4) * sp.envelope_exp_decay(0.03, tau=0.01)
                end = offset + len(drop)
                if end <= len(body):
                    body[offset:end] += drop
        return body
    if material == "ice":
        ring1 = sp.sine(min(dur, 0.3), 3000.0, 0.5) * sp.envelope_exp_decay(min(dur, 0.3), tau=0.08)
        ring2 = sp.sine(min(dur, 0.3), 5000.0, 0.4) * sp.envelope_exp_decay(min(dur, 0.3), tau=0.07)
        ring3 = sp.sine(min(dur, 0.3), 7000.0, 0.3) * sp.envelope_exp_decay(min(dur, 0.3), tau=0.06)
        crack = sp.transient_click(0.003, 0.7)
        return sp.mix(crack, ring1, ring2, ring3, normalize=False)
    if material == "sand":
        body = sp.band_filter(sp.brown_noise(dur, 0.65, rng), 100, 1500) * sp.envelope_attack_decay(dur, 0.03, 0.15)
        return body
    body = sp.band_filter(sp.pink_noise(dur, 0.6, rng), 100, 2500) * sp.envelope_exp_decay(dur, tau=0.03)
    return body


def synth_projectile(entry: Dict, rng: np.random.RandomState) -> np.ndarray:
    cat = entry.get("category", "")
    dur = float(entry.get("duration_target_sec", 0.2))
    if cat == "flyby":
        dist = float(entry.get("distance_m", 3.0))
        f_start = max(800.0, 3000.0 - dist * 250.0)
        f_end = max(400.0, 1200.0 - dist * 80.0)
        zip_amp = max(0.25, 1.0 - dist * 0.1)
        zip_ = sp.chirp(dur, f_start, f_end, amp=zip_amp) * sp.envelope_attack_decay(dur, 0.005, 0.04)
        whoosh = sp.band_filter(sp.white_noise(dur, 0.4 + 0.2 / max(dist, 0.1), rng), 500, 4000) * sp.envelope_attack_decay(dur, 0.005, 0.05)
        return sp.mix(zip_, whoosh, normalize=False)
    if cat == "supersonic":
        crack = sp.transient_click(0.002, 0.95)
        ring = sp.sine(dur, 6000.0, 0.5) * sp.envelope_exp_decay(dur, tau=0.01)
        return sp.mix(crack, ring, normalize=False)
    if cat == "subsonic":
        whoosh = sp.band_filter(sp.white_noise(dur, 0.65, rng), 300, 2500) * sp.envelope_attack_decay(dur, 0.05, 0.15)
        hum = sp.sine(dur, 400.0, 0.3) * sp.envelope_attack_decay(dur, 0.05, 0.1)
        return sp.mix(whoosh, hum, normalize=False)
    if cat == "tracer":
        hiss = sp.band_filter(sp.white_noise(dur, 0.7, rng), 1500, 8000) * sp.envelope_attack_decay(dur, 0.05, 0.2)
        burn = sp.band_filter(sp.pink_noise(dur, 0.4, rng), 500, 2500) * sp.envelope_attack_decay(dur, 0.05, 0.2)
        return sp.mix(hiss, burn, normalize=False)
    if cat == "rocket":
        roar = sp.band_filter(sp.brown_noise(dur, 0.85, rng), 60, 2000)
        roar = sp.amplitude_lfo(roar, rate_hz=0.7, depth=0.2)
        sub = sp.sine(dur, 100.0, 0.4)
        return sp.mix(roar, sub, normalize=False)
    return _generic_placeholder(rng, dur)


def synth_impact(entry: Dict, rng: np.random.RandomState) -> np.ndarray:
    dur = float(entry.get("duration_target_sec", 0.4))
    material = entry.get("material", "")
    damage = entry.get("damage_type", "kinetic")
    intensity = entry.get("intensity", "small")

    if damage == "kinetic":
        return _impact_kinetic(rng, dur, material, intensity)
    if damage == "thermal":
        sizzle = sp.band_filter(sp.pink_noise(dur, 0.7, rng), 2000, 10000) * sp.envelope_attack_decay(dur, 0.05, 0.4)
        pops = sp.crackle_pattern(dur, density_per_sec=12.0, peak=0.5, rng=rng)
        if material == "oil" or material == "methane":
            whoof = sp.band_filter(sp.brown_noise(dur, 0.7, rng), 100, 2500) * sp.envelope_attack_decay(dur, 0.05, 0.3)
            return sp.mix(sizzle, pops, whoof, normalize=False)
        if material == "metal":
            warp = sp.band_filter(sp.pink_noise(dur, 0.4, rng), 800, 3000) * sp.envelope_attack_decay(dur, 0.1, 0.4)
            return sp.mix(sizzle, pops, warp, normalize=False)
        if material == "wood":
            ignite = sp.band_filter(sp.brown_noise(dur, 0.5, rng), 200, 2000) * sp.envelope_attack_decay(dur, 0.05, 0.3)
            return sp.mix(sizzle, pops, ignite, normalize=False)
        return sp.mix(sizzle, pops, normalize=False)
    if damage == "electric":
        hum = sp.sine(dur, 60.0, 0.4) + sp.sine(dur, 120.0, 0.25)
        zap = sp.transient_click(0.003, 0.7)
        bursts = np.zeros(sp._n_samples(dur))
        for offset_sec in (0.1, 0.25, 0.4):
            if offset_sec < dur:
                offset = int(offset_sec * sp.SAMPLE_RATE)
                burst = sp.band_filter(sp.white_noise(0.05, 0.7, rng), 1000, 5000) * sp.envelope_exp_decay(0.05, tau=0.02)
                end = offset + len(burst)
                if end <= len(bursts):
                    bursts[offset:end] += burst
        if material == "water":
            bubble = sp.band_filter(sp.brown_noise(dur, 0.5, rng), 200, 2000) * sp.envelope_attack_decay(dur, 0.05, 0.3)
            return sp.mix(hum, zap, bursts, bubble, normalize=False)
        return sp.mix(hum, zap, bursts, normalize=False)
    if damage == "chemical":
        hiss = sp.band_filter(sp.white_noise(dur, 0.75, rng), 2000, 8000) * sp.envelope_attack_decay(dur, 0.1, 0.6)
        bubbles = sp.crackle_pattern(dur, density_per_sec=6.0, peak=0.5, rng=rng)
        return sp.mix(hiss, bubbles, normalize=False)
    if damage == "explosive":
        if intensity == "small":
            tau, low_f, decay = 0.2, 90.0, 0.4
        elif intensity == "large":
            tau, low_f, decay = 0.5, 40.0, 1.0
        else:
            tau, low_f, decay = 0.3, 60.0, 0.6
        boom = sp.sine(min(dur, 1.0), low_f, 0.9) * sp.envelope_exp_decay(min(dur, 1.0), tau=tau)
        boom2 = sp.sine(min(dur, 1.0), low_f * 1.5, 0.6) * sp.envelope_exp_decay(min(dur, 1.0), tau=tau * 0.8)
        click = sp.transient_click(0.003, 0.9)
        noise = sp.band_filter(sp.white_noise(dur, 0.8, rng), 100, 5000) * sp.envelope_exp_decay(dur, tau=tau * 1.5)
        body = sp.mix(click, boom, boom2, noise, normalize=False)
        body = sp.reverb_simple(body, decay=decay, density=12, rng=rng)
        return body

    return _impact_kinetic(rng, dur, material or "concrete", intensity)


def synth_body_hit(entry: Dict, rng: np.random.RandomState) -> np.ndarray:
    dur = float(entry.get("duration_target_sec", 0.4))
    target = entry.get("target", "flesh")
    intensity = entry.get("intensity", "small")
    big = intensity in ("large", "critical", "defeat", "penetrate")

    if target == "flesh":
        wet = sp.band_filter(sp.brown_noise(dur, 0.85, rng), 80, 1000) * sp.envelope_exp_decay(dur, tau=0.05 if big else 0.03)
        thump = sp.sine(min(dur, 0.12), 90.0, 0.65 if big else 0.5) * sp.envelope_exp_decay(min(dur, 0.12), tau=0.05)
        return sp.mix(wet, thump, normalize=False)
    if target == "armor":
        if intensity == "glance":
            ring1 = sp.sine(dur, 2500.0, 0.6) * sp.envelope_attack_decay(dur, 0.01, 0.08)
            ring2 = sp.sine(dur, 3500.0, 0.45) * sp.envelope_attack_decay(dur, 0.01, 0.06)
            ricochet = sp.band_filter(sp.white_noise(dur, 0.5, rng), 2000, 8000) * sp.envelope_exp_decay(dur, tau=0.05)
            return sp.mix(ring1, ring2, ricochet, normalize=False)
        if intensity == "penetrate":
            crack = sp.transient_click(0.003, 0.85)
            ring = sp.sine(min(dur, 0.05), 1500.0, 0.6) * sp.envelope_exp_decay(min(dur, 0.05), tau=0.02)
            wet = sp.band_filter(sp.brown_noise(dur, 0.7, rng), 80, 1000) * sp.envelope_exp_decay(dur, tau=0.05)
            wet_offset = int(0.05 * sp.SAMPLE_RATE)
            base = sp.mix(crack, ring, normalize=False)
            base = sp.ensure_duration(base, dur)
            if wet_offset + len(wet) <= len(base):
                base[wet_offset:wet_offset + len(wet)] += wet
            return base
        crack = sp.transient_click(0.003, 0.9)
        clang = sp.sine(min(dur, 0.2), 900.0, 0.6) * sp.envelope_exp_decay(min(dur, 0.2), tau=0.07)
        ring = sp.sine(min(dur, 0.2), 1500.0, 0.5) * sp.envelope_exp_decay(min(dur, 0.2), tau=0.06)
        return sp.mix(crack, clang, ring, normalize=False)
    if target == "robot":
        clang1 = sp.sine(min(dur, 0.2), 800.0, 0.55) * sp.envelope_exp_decay(min(dur, 0.2), tau=0.05)
        clang2 = sp.sine(min(dur, 0.2), 1200.0, 0.4) * sp.envelope_exp_decay(min(dur, 0.2), tau=0.05)
        clang3 = sp.sine(min(dur, 0.2), 1600.0, 0.3) * sp.envelope_exp_decay(min(dur, 0.2), tau=0.05)
        spark = sp.band_filter(sp.white_noise(0.05, 0.6, rng), 3000, 8000) * sp.envelope_exp_decay(0.05, tau=0.02)
        if intensity == "critical":
            fry = sp.band_filter(sp.white_noise(dur, 0.7, rng), 2000, 10000) * sp.envelope_attack_decay(dur, 0.05, 0.3)
            pop = sp.sine(min(dur, 0.1), 200.0, 0.5) * sp.envelope_exp_decay(min(dur, 0.1), tau=0.04)
            return sp.mix(clang1, clang2, clang3, spark, fry, pop, normalize=False)
        return sp.mix(clang1, clang2, clang3, spark, normalize=False)
    if target == "chassis":
        if intensity == "module_break":
            snap = sp.transient_click(0.004, 0.85)
            creak = sp.sine(min(dur, 0.15), 800.0, 0.45) * sp.envelope_attack_decay(min(dur, 0.15), 0.01, 0.04)
            hiss = sp.band_filter(sp.white_noise(dur, 0.55, rng), 200, 1500) * sp.envelope_attack_decay(dur, 0.05, 0.25)
            return sp.mix(snap, creak, hiss, normalize=False)
        snap = sp.transient_click(0.003, 0.85)
        ceramic = sp.sine(min(dur, 0.15), 3500.0, 0.55) * sp.envelope_exp_decay(min(dur, 0.15), tau=0.04)
        ceramic2 = sp.sine(min(dur, 0.15), 5000.0, 0.4) * sp.envelope_exp_decay(min(dur, 0.15), tau=0.035)
        return sp.mix(snap, ceramic, ceramic2, normalize=False)

    return _generic_placeholder(rng, dur)


def synth_dismember(entry: Dict, rng: np.random.RandomState) -> np.ndarray:
    dur = float(entry.get("duration_target_sec", 0.7))
    limb = entry.get("limb", "")

    if limb in ("arm", "leg"):
        tear = sp.band_filter(sp.brown_noise(dur, 0.85, rng), 80, 1500) * sp.envelope_attack_decay(dur, 0.05, 0.3)
        bone = sp.transient_click(0.003, 0.85)
        spray = sp.band_filter(sp.pink_noise(dur, 0.5, rng), 200, 2500) * sp.envelope_attack_decay(dur, 0.05, 0.3)
        return sp.mix(tear, bone, spray, normalize=False)
    if limb == "head":
        thunk = sp.band_filter(sp.brown_noise(dur, 0.85, rng), 80, 1000) * sp.envelope_exp_decay(dur, tau=0.1)
        boom = sp.sine(min(dur, 0.18), 60.0, 0.7) * sp.envelope_exp_decay(min(dur, 0.18), tau=0.06)
        spray = sp.band_filter(sp.pink_noise(dur, 0.4, rng), 500, 3500) * sp.envelope_attack_decay(dur, 0.05, 0.15)
        return sp.mix(thunk, boom, spray, normalize=False)
    if limb in ("robot_arm", "robot_leg"):
        click = sp.transient_click(0.004, 0.85)
        creak = sp.sine(min(dur, 0.2), 800.0, 0.55) * sp.envelope_attack_decay(min(dur, 0.2), 0.01, 0.05)
        sparks = sp.band_filter(sp.white_noise(dur, 0.6, rng), 3000, 12000) * sp.envelope_attack_decay(dur, 0.05, 0.2)
        hiss = sp.band_filter(sp.white_noise(dur, 0.55, rng), 200, 1500) * sp.envelope_attack_decay(dur, 0.05, 0.3)
        return sp.mix(click, creak, sparks, hiss, normalize=False)
    if limb == "chassis_arm":
        bolts = sp.band_filter(sp.white_noise(min(dur, 0.2), 0.6, rng), 800, 3500) * sp.envelope_exp_decay(min(dur, 0.2), tau=0.06)
        drop = sp.sine(min(dur, 0.3), 100.0, 0.7) * sp.envelope_exp_decay(min(dur, 0.3), tau=0.12)
        clang = sp.sine(min(dur, 0.25), 600.0, 0.5) * sp.envelope_exp_decay(min(dur, 0.25), tau=0.08)
        base = sp.ensure_duration(sp.mix(bolts, drop, clang, normalize=False), dur)
        return base
    if limb == "torso_gib":
        base = np.zeros(sp._n_samples(dur))
        for offset_sec in (0.0, 0.08, 0.16, 0.25, 0.35, 0.45, 0.55):
            if offset_sec < dur:
                offset = int(offset_sec * sp.SAMPLE_RATE)
                splat = sp.band_filter(sp.brown_noise(0.12, 0.7, rng), 100, 1500) * sp.envelope_exp_decay(0.12, tau=0.05)
                end = offset + len(splat)
                if end <= len(base):
                    base[offset:end] += splat * 0.7
        mist = sp.band_filter(sp.white_noise(dur, 0.4, rng), 2000, 8000) * sp.envelope_attack_decay(dur, 0.05, 0.3)
        return base + mist
    if limb == "chassis_gib":
        boom = sp.sine(min(dur, 0.4), 50.0, 0.95) * sp.envelope_exp_decay(min(dur, 0.4), tau=0.2)
        click = sp.transient_click(0.004, 0.95)
        debris = np.zeros(sp._n_samples(dur))
        for offset_sec in (0.1, 0.2, 0.35, 0.5, 0.7, 0.9):
            if offset_sec < dur:
                offset = int(offset_sec * sp.SAMPLE_RATE)
                clang = sp.sine(0.08, float(rng.uniform(700, 1500)), 0.55) * sp.envelope_exp_decay(0.08, tau=0.04)
                end = offset + len(clang)
                if end <= len(debris):
                    debris[offset:end] += clang
        return sp.mix(click, boom, debris, normalize=False)

    return _generic_placeholder(rng, dur)


def synth_death(entry: Dict, rng: np.random.RandomState) -> np.ndarray:
    dur = float(entry.get("duration_target_sec", 1.0))
    target = entry.get("target", "")

    if target == "male_grunt":
        vox = sp.voice_formant(dur, f0=110.0, formants=[220, 440, 880, 1200], vibrato_hz=5.0, vibrato_depth_hz=4.0, rng=rng)
        return vox * sp.envelope_attack_decay(dur, 0.05, 0.3)
    if target == "female_grunt":
        vox = sp.voice_formant(dur, f0=200.0, formants=[400, 800, 1600, 2400], vibrato_hz=5.0, vibrato_depth_hz=5.0, rng=rng)
        return vox * sp.envelope_attack_decay(dur, 0.05, 0.3)
    if target == "male_scream":
        t = sp.t_axis(dur)
        sweep_t = t / max(dur, 1e-6)
        f0 = 200.0 + 150.0 * np.sin(np.pi * sweep_t) - 50.0 * sweep_t
        vibrato = 6.0 * np.sin(2.0 * np.pi * 7.0 * t)
        phase = 2.0 * np.pi * np.cumsum(f0 + vibrato) / sp.SAMPLE_RATE
        vox = np.sin(phase) * 0.7
        vox = vox + np.sin(phase * 2.0) * 0.3
        vox = vox + np.sin(phase * 3.5) * 0.2
        vox = sp.band_filter(vox, 150, 4000)
        return vox * sp.envelope_attack_decay(dur, 0.05, 0.5)
    if target == "female_scream":
        t = sp.t_axis(dur)
        sweep_t = t / max(dur, 1e-6)
        f0 = 350.0 + 250.0 * np.sin(np.pi * sweep_t) - 80.0 * sweep_t
        vibrato = 8.0 * np.sin(2.0 * np.pi * 7.5 * t)
        phase = 2.0 * np.pi * np.cumsum(f0 + vibrato) / sp.SAMPLE_RATE
        vox = np.sin(phase) * 0.7
        vox = vox + np.sin(phase * 2.0) * 0.35
        vox = vox + np.sin(phase * 3.5) * 0.22
        vox = sp.band_filter(vox, 200, 5000)
        return vox * sp.envelope_attack_decay(dur, 0.05, 0.5)
    if target == "robot_shutdown":
        descent = sp.pitch_envelope(dur, 1000.0, 100.0, curve="exp", amp=0.7) * sp.envelope_attack_decay(dur, 0.05, 0.4)
        whine = sp.sine(dur, 800.0, 0.3) * sp.envelope_attack_decay(dur, 0.05, 0.4)
        pop = np.zeros(sp._n_samples(dur))
        offset = int(min(dur - 0.05, dur * 0.9) * sp.SAMPLE_RATE)
        click = sp.transient_click(0.003, 0.85)
        if offset + len(click) <= len(pop):
            pop[offset:offset + len(click)] += click
        zap = sp.band_filter(sp.white_noise(0.1, 0.6, rng), 1000, 6000) * sp.envelope_exp_decay(0.1, tau=0.04)
        if offset + len(zap) <= len(pop):
            pop[offset:offset + len(zap)] += zap
        return sp.mix(descent, whine, pop, normalize=False)
    if target == "robot_explode":
        ascent = sp.pitch_envelope(min(dur, 0.5), 100.0, 2000.0, curve="exp", amp=0.6) * sp.envelope_attack_decay(min(dur, 0.5), 0.05, 0.2)
        boom = sp.sine(min(dur, 0.5), 50.0, 0.9) * sp.envelope_exp_decay(min(dur, 0.5), tau=0.2)
        click = sp.transient_click(0.003, 0.95)
        crackle = sp.crackle_pattern(dur, density_per_sec=20.0, peak=0.5, rng=rng)
        base = sp.ensure_duration(sp.mix(ascent, click, boom, normalize=False), dur)
        crack_offset = int(min(dur, 0.5) * sp.SAMPLE_RATE)
        if crack_offset + len(crackle) <= len(base):
            base[crack_offset:crack_offset + len(crackle)] += crackle
        else:
            base = base + crackle[:len(base)]
        return base
    if target == "biomech":
        vox = sp.voice_formant(dur, f0=60.0, formants=[120, 240, 480, 800], vibrato_hz=3.0, vibrato_depth_hz=5.0, rng=rng)
        hiss = sp.band_filter(sp.white_noise(dur, 0.5, rng), 200, 1500) * sp.envelope_attack_decay(dur, 0.1, 0.4)
        return sp.mix(vox * 0.8, hiss, normalize=False)
    if target == "chassis_eject":
        bolt = sp.transient_click(0.005, 0.95)
        rocket = sp.band_filter(sp.brown_noise(dur, 0.85, rng), 80, 2500) * sp.envelope_attack_decay(dur, 0.05, 0.6)
        fund = sp.sine(dur, 120.0, 0.4) * sp.envelope_attack_decay(dur, 0.05, 0.5)
        glass = sp.band_filter(sp.white_noise(0.4, 0.7, rng), 3000, 12000) * sp.envelope_exp_decay(0.4, tau=0.08)
        base = sp.mix(bolt, rocket, fund, normalize=False)
        base = sp.ensure_duration(base, dur)
        glass_offset = int(0.4 * sp.SAMPLE_RATE)
        if glass_offset + len(glass) <= len(base):
            base[glass_offset:glass_offset + len(glass)] += glass
        return base
    if target == "aqueous":
        splash = sp.band_filter(sp.white_noise(dur, 0.7, rng), 800, 6000) * sp.envelope_attack_decay(dur, 0.05, 0.3)
        bubble = sp.crackle_pattern(dur, density_per_sec=8.0, peak=0.5, rng=rng)
        liquid = sp.band_filter(sp.brown_noise(dur, 0.5, rng), 100, 1500) * sp.envelope_attack_decay(dur, 0.05, 0.5)
        return sp.mix(splash, bubble, liquid, normalize=False)
    if target == "crystalline":
        crack = sp.transient_click(0.003, 0.85)
        rings = sum(
            sp.sine(dur, f, 0.4) * sp.envelope_exp_decay(dur, tau=0.3 - i * 0.04)
            for i, f in enumerate([2000.0, 3000.0, 4500.0, 6000.0, 8000.0])
        )
        return sp.mix(crack, rings, normalize=False)

    return _generic_placeholder(rng, dur)


# ─── Ambient / Weather / Hazard / UI / Chatter ─────────────────────────────


def synth_ambient(entry: Dict, rng: np.random.RandomState) -> np.ndarray:
    dur = float(entry.get("duration_target_sec", 30.0))
    world = entry.get("world", "")
    biome = entry.get("biome", "")
    name = entry.get("id", "")

    if name == "amb_earth_outpost" or (world == "earth" and biome == "outpost"):
        hum = sp.sine(dur, 60.0, 0.35) + sp.sine(dur, 80.0, 0.25) + sp.sine(dur, 120.0, 0.18)
        hum = sp.amplitude_lfo(hum, rate_hz=0.1, depth=0.05)
        wind = sp.band_filter(sp.brown_noise(dur, 0.45, rng), 60, 1500)
        wind = sp.amplitude_lfo(wind, rate_hz=0.2, depth=0.3)
        statics = np.zeros(sp._n_samples(dur))
        for offset_sec in np.arange(0, dur, 7.0):
            offset = int(offset_sec * sp.SAMPLE_RATE)
            burst = sp.band_filter(sp.white_noise(0.05, 0.4, rng), 2000, 5000) * sp.envelope_exp_decay(0.05, tau=0.02)
            end = offset + len(burst)
            if end <= len(statics):
                statics[offset:end] += burst
        clanks = np.zeros(sp._n_samples(dur))
        for offset_sec in np.arange(2.0, dur, 11.0):
            offset = int(offset_sec * sp.SAMPLE_RATE)
            clank = sp.sine(0.12, 200.0, 0.45) * sp.envelope_exp_decay(0.12, tau=0.04)
            end = offset + len(clank)
            if end <= len(clanks):
                clanks[offset:end] += clank
        return sp.mix(hum, wind, statics, clanks, normalize=False)
    if name == "amb_earth_urban":
        wind = sp.band_filter(sp.brown_noise(dur, 0.5, rng), 80, 2500)
        wind = sp.amplitude_lfo(wind, rate_hz=0.15, depth=0.4)
        siren = sp.sine(dur, 600.0, 0.1) * sp.amplitude_lfo(np.ones(sp._n_samples(dur)), rate_hz=0.05, depth=0.9)
        drips = np.zeros(sp._n_samples(dur))
        for offset_sec in np.arange(3.0, dur, 4.0):
            offset = int(offset_sec * sp.SAMPLE_RATE)
            drip = sp.sine(0.04, 1500.0, 0.35) * sp.envelope_exp_decay(0.04, tau=0.015)
            end = offset + len(drip)
            if end <= len(drips):
                drips[offset:end] += drip
        return sp.mix(wind, siren, drips, normalize=False)
    if name == "amb_mars_dust" or (world == "mars" and biome == "dust_plain"):
        wind = sp.band_filter(sp.brown_noise(dur, 0.75, rng), 100, 2000)
        wind = sp.amplitude_lfo(wind, rate_hz=0.15, depth=0.45)
        gusts = np.zeros(sp._n_samples(dur))
        for offset_sec in np.arange(5.0, dur, 18.0):
            offset = int(offset_sec * sp.SAMPLE_RATE)
            gust = sp.band_filter(sp.pink_noise(3.0, 0.6, rng), 200, 3500) * sp.envelope_attack_decay(3.0, 0.3, 1.0)
            end = offset + len(gust)
            if end <= len(gusts):
                gusts[offset:end] += gust
        creak = np.zeros(sp._n_samples(dur))
        for offset_sec in np.arange(8.0, dur, 12.0):
            offset = int(offset_sec * sp.SAMPLE_RATE)
            crk = sp.sine(0.25, 90.0, 0.3) * sp.envelope_attack_decay(0.25, 0.05, 0.07)
            end = offset + len(crk)
            if end <= len(creak):
                creak[offset:end] += crk
        return sp.mix(wind, gusts, creak, normalize=False)
    if name == "amb_mars_storm":
        roar = sp.band_filter(sp.brown_noise(dur, 0.95, rng), 80, 4000)
        roar = sp.amplitude_lfo(roar, rate_hz=0.2, depth=0.4)
        sand = sp.band_filter(sp.pink_noise(dur, 0.7, rng), 3000, 10000)
        sand = sp.amplitude_lfo(sand, rate_hz=4.0, depth=0.5)
        sub = sp.sine(dur, 40.0, 0.4) * sp.amplitude_lfo(np.ones(sp._n_samples(dur)), rate_hz=0.1, depth=0.3)
        return sp.mix(roar, sand, sub, normalize=False)
    if name == "amb_moon_vacuum":
        breath = np.zeros(sp._n_samples(dur))
        cycle = 4.0
        for offset_sec in np.arange(0.5, dur, cycle):
            start = int(offset_sec * sp.SAMPLE_RATE)
            in_breath = sp.band_filter(sp.white_noise(0.8, 0.35, rng), 200, 2000) * sp.envelope_attack_decay(0.8, 0.1, 0.4)
            end = start + len(in_breath)
            if end <= len(breath):
                breath[start:end] += in_breath
            out_start = start + int(1.6 * sp.SAMPLE_RATE)
            out_breath = sp.band_filter(sp.white_noise(0.9, 0.3, rng), 150, 1800) * sp.envelope_attack_decay(0.9, 0.1, 0.5)
            out_end = out_start + len(out_breath)
            if out_end <= len(breath):
                breath[out_start:out_end] += out_breath
        pump = sp.sine(dur, 50.0, 0.18) * sp.amplitude_lfo(np.ones(sp._n_samples(dur)), rate_hz=0.5, depth=0.4)
        statics = np.zeros(sp._n_samples(dur))
        for offset_sec in np.arange(3.0, dur, 9.0):
            offset = int(offset_sec * sp.SAMPLE_RATE)
            burst = sp.band_filter(sp.white_noise(0.04, 0.4, rng), 1500, 5000) * sp.envelope_exp_decay(0.04, tau=0.02)
            end = offset + len(burst)
            if end <= len(statics):
                statics[offset:end] += burst
        return sp.mix(breath, pump, statics, normalize=False)
    if name == "amb_phobos_lowg":
        dust = sp.band_filter(sp.pink_noise(dur, 0.3, rng), 1500, 6000)
        dust = sp.amplitude_lfo(dust, rate_hz=0.2, depth=0.3)
        creak = np.zeros(sp._n_samples(dur))
        for offset_sec in np.arange(5.0, dur, 10.0):
            offset = int(offset_sec * sp.SAMPLE_RATE)
            crk = sp.sine(0.4, 100.0, 0.3) * sp.envelope_attack_decay(0.4, 0.1, 0.15)
            end = offset + len(crk)
            if end <= len(creak):
                creak[offset:end] += crk
        return sp.mix(dust, creak, normalize=False)
    if name == "amb_mimas_methane_sea":
        lap = sp.band_filter(sp.brown_noise(dur, 0.65, rng), 200, 2000)
        lap = sp.amplitude_lfo(lap, rate_hz=0.3, depth=0.4)
        whistle = sp.sine(dur, 300.0, 0.2) * sp.amplitude_lfo(np.ones(sp._n_samples(dur)), rate_hz=0.4, depth=0.5)
        bubbles = np.zeros(sp._n_samples(dur))
        for offset_sec in np.arange(2.0, dur, 5.0):
            offset = int(offset_sec * sp.SAMPLE_RATE)
            bub = sp.sine(0.12, 800.0, 0.35) * sp.envelope_exp_decay(0.12, tau=0.04)
            end = offset + len(bub)
            if end <= len(bubbles):
                bubbles[offset:end] += bub
        rumble = sp.sine(dur, 45.0, 0.3)
        return sp.mix(lap, whistle, bubbles, rumble, normalize=False)
    if name == "amb_europa_ice_cavern":
        hum = sp.sine(dur, 50.0, 0.4) + sp.sine(dur, 80.0, 0.3)
        drip = np.zeros(sp._n_samples(dur))
        for offset_sec in np.arange(1.0, dur, 3.5):
            offset = int(offset_sec * sp.SAMPLE_RATE)
            dr = sp.sine(0.04, 1000.0, 0.4) * sp.envelope_exp_decay(0.04, tau=0.015)
            end = offset + len(dr)
            if end <= len(drip):
                drip[offset:end] += dr
        crackle = sp.crackle_pattern(dur, density_per_sec=2.0, peak=0.4, rng=rng)
        crackle = sp.band_filter(crackle, 4000, 12000)
        creature = np.zeros(sp._n_samples(dur))
        for offset_sec in np.arange(15.0, dur, 18.0):
            offset = int(offset_sec * sp.SAMPLE_RATE)
            call = sp.voice_formant(2.0, f0=180.0, formants=[400, 800, 1500], vibrato_hz=3.0, vibrato_depth_hz=10.0, rng=rng) * 0.3
            end = offset + len(call)
            if end <= len(creature):
                creature[offset:end] += call
        return sp.mix(hum, drip, crackle, creature, normalize=False)
    if name == "amb_europa_ocean_depth":
        rumble = sp.sine(dur, 35.0, 0.5) + sp.sine(dur, 55.0, 0.3)
        rumble = sp.amplitude_lfo(rumble, rate_hz=0.15, depth=0.2)
        sonar = np.zeros(sp._n_samples(dur))
        for offset_sec in np.arange(7.0, dur, 9.0):
            offset = int(offset_sec * sp.SAMPLE_RATE)
            ping = sp.sine(0.3, 1500.0, 0.4) * sp.envelope_attack_decay(0.3, 0.01, 0.1)
            end = offset + len(ping)
            if end <= len(sonar):
                sonar[offset:end] += ping
        creature = np.zeros(sp._n_samples(dur))
        for offset_sec in np.arange(10.0, dur, 13.0):
            offset = int(offset_sec * sp.SAMPLE_RATE)
            call = sp.voice_formant(1.5, f0=120.0, formants=[300, 600, 1100], vibrato_hz=2.0, vibrato_depth_hz=6.0, rng=rng) * 0.25
            end = offset + len(call)
            if end <= len(creature):
                creature[offset:end] += call
        return sp.mix(rumble, sonar, creature, normalize=False)
    if name == "amb_vulcan_magma":
        bubble = sp.band_filter(sp.brown_noise(dur, 0.7, rng), 100, 1500)
        bubble = sp.amplitude_lfo(bubble, rate_hz=0.6, depth=0.4)
        heat = sp.sine(dur, 2000.0, 0.1) + sp.sine(dur, 3000.0, 0.08)
        crackles = sp.crackle_pattern(dur, density_per_sec=4.0, peak=0.4, rng=rng)
        return sp.mix(bubble, heat, crackles, normalize=False)
    if name == "amb_venus_clouds":
        wind = sp.band_filter(sp.brown_noise(dur, 0.85, rng), 100, 3000)
        wind = sp.amplitude_lfo(wind, rate_hz=0.1, depth=0.4)
        acid = sp.band_filter(sp.white_noise(dur, 0.6, rng), 5000, 15000)
        acid = sp.amplitude_lfo(acid, rate_hz=3.0, depth=0.6)
        thunder = np.zeros(sp._n_samples(dur))
        for offset_sec in np.arange(8.0, dur, 14.0):
            offset = int(offset_sec * sp.SAMPLE_RATE)
            boom = sp.sine(1.5, 50.0, 0.55) * sp.envelope_attack_decay(1.5, 0.2, 0.6)
            end = offset + len(boom)
            if end <= len(thunder):
                thunder[offset:end] += boom
        return sp.mix(wind, acid, thunder, normalize=False)
    if name == "amb_belt_asteroid":
        thuds = np.zeros(sp._n_samples(dur))
        for offset_sec in np.arange(3.0, dur, 8.0):
            offset = int(offset_sec * sp.SAMPLE_RATE)
            thud = sp.sine(0.4, 70.0, 0.5) * sp.envelope_exp_decay(0.4, tau=0.15)
            end = offset + len(thud)
            if end <= len(thuds):
                thuds[offset:end] += thud
        conveyor = sp.sine(dur, 80.0, 0.3)
        clicks = np.zeros(sp._n_samples(dur))
        for offset_sec in np.arange(0.0, dur, 1.0):
            offset = int(offset_sec * sp.SAMPLE_RATE)
            click = sp.transient_click(0.002, 0.25)
            end = offset + len(click)
            if end <= len(clicks):
                clicks[offset:end] += click
        chatter = sp.band_filter(sp.voice_formant(dur, 200.0, [400, 800, 1500, 2400], 5.0, 8.0, rng) * 0.18, 300, 3400)
        return sp.mix(thuds, conveyor, clicks, chatter, normalize=False)
    if name == "amb_orbital_station":
        recycler = sp.sine(dur, 60.0, 0.32) + sp.sine(dur, 120.0, 0.22)
        beeps = np.zeros(sp._n_samples(dur))
        for offset_sec in np.arange(4.0, dur, 8.5):
            offset = int(offset_sec * sp.SAMPLE_RATE)
            beep = sp.sine(0.1, 1000.0, 0.35) * sp.envelope_adsr(0.1, 0.01, 0.02, 0.7, 0.04)
            end = offset + len(beep)
            if end <= len(beeps):
                beeps[offset:end] += beep
        steps = np.zeros(sp._n_samples(dur))
        for offset_sec in np.arange(6.0, dur, 5.5):
            offset = int(offset_sec * sp.SAMPLE_RATE)
            step = sp.low_pass(sp.transient_click(0.005, 0.4), 800.0)
            end = offset + len(step)
            if end <= len(steps):
                steps[offset:end] += step
        return sp.mix(recycler, beeps, steps, normalize=False)
    if name == "amb_sol_zone_habitat":
        wind = sp.band_filter(sp.white_noise(dur, 0.5, rng), 5000, 15000)
        wind = sp.amplitude_lfo(wind, rate_hz=0.2, depth=0.3)
        cryst = sp.sine(dur, 2000.0, 0.18) + sp.sine(dur, 3000.0, 0.14) + sp.sine(dur, 4000.0, 0.1)
        hum = sp.sine(dur, 60.0, 0.3)
        return sp.mix(wind, cryst, hum, normalize=False)
    if name == "amb_reactor_room":
        hum = sp.sine(dur, 60.0, 0.55) + sp.sine(dur, 120.0, 0.42) + sp.sine(dur, 240.0, 0.28)
        pumps = sp.sine(dur, 30.0, 0.4) * sp.amplitude_lfo(np.ones(sp._n_samples(dur)), rate_hz=1.0, depth=0.5)
        ticks = sp.crackle_pattern(dur, density_per_sec=2.0, peak=0.4, rng=rng)
        warning = np.zeros(sp._n_samples(dur))
        for offset_sec in np.arange(10.0, dur, 15.0):
            offset = int(offset_sec * sp.SAMPLE_RATE)
            beep = sp.sine(0.25, 1000.0, 0.5) * sp.envelope_adsr(0.25, 0.01, 0.02, 0.7, 0.05)
            end = offset + len(beep)
            if end <= len(warning):
                warning[offset:end] += beep
        return sp.mix(hum, pumps, ticks, warning, normalize=False)
    if name == "amb_command_core":
        holo = sp.sine(dur, 100.0, 0.3) * sp.amplitude_lfo(np.ones(sp._n_samples(dur)), rate_hz=0.4, depth=0.2)
        ticks = np.zeros(sp._n_samples(dur))
        for offset_sec in np.arange(2.0, dur, 3.5):
            offset = int(offset_sec * sp.SAMPLE_RATE)
            click = sp.transient_click(0.003, 0.5)
            end = offset + len(click)
            if end <= len(ticks):
                ticks[offset:end] += click
        chatter = sp.band_filter(sp.voice_formant(dur, 220.0, [400, 800, 1500, 2400], 5.0, 8.0, rng) * 0.12, 300, 3400)
        vent = sp.band_filter(sp.brown_noise(dur, 0.35, rng), 50, 500)
        return sp.mix(holo, ticks, chatter, vent, normalize=False)

    return sp.amplitude_lfo(sp.band_filter(sp.brown_noise(dur, 0.5, rng), 80, 3000), rate_hz=0.2, depth=0.3)


def synth_weather(entry: Dict, rng: np.random.RandomState) -> np.ndarray:
    dur = float(entry.get("duration_target_sec", 15.0))
    kind = entry.get("type", "")
    intensity = entry.get("intensity", "any")

    if kind == "rain":
        density = 30.0 if intensity == "light" else 200.0
        body = sp.band_filter(sp.white_noise(dur, 0.8 if intensity == "heavy" else 0.5, rng), 2000, 12000)
        body = sp.amplitude_lfo(body, rate_hz=2.0, depth=0.2)
        drops = sp.crackle_pattern(dur, density_per_sec=density, peak=0.5, rng=rng)
        drops = sp.band_filter(drops, 1500, 8000)
        out = sp.mix(body, drops, normalize=False)
        if intensity == "heavy":
            thunder = np.zeros(sp._n_samples(dur))
            for offset_sec in np.arange(4.0, dur, 7.0):
                offset = int(offset_sec * sp.SAMPLE_RATE)
                boom = sp.sine(1.2, 60.0, 0.4) * sp.envelope_attack_decay(1.2, 0.1, 0.5)
                end = offset + len(boom)
                if end <= len(thunder):
                    thunder[offset:end] += boom
            out = sp.mix(out, thunder, normalize=False)
        return out
    if kind == "thunder":
        rumble = sp.sine(dur, 50.0, 0.5) * sp.envelope_attack_decay(dur, 0.4, 0.8)
        rumble = rumble + sp.sine(dur, 70.0, 0.3) * sp.envelope_attack_decay(dur, 0.4, 0.8)
        crack = sp.transient_click(0.005, 0.85)
        noise = sp.band_filter(sp.white_noise(dur, 0.5, rng), 100, 3000) * sp.envelope_attack_decay(dur, 0.3, 0.7)
        body = sp.mix(rumble, crack, noise, normalize=False)
        return sp.reverb_simple(body, decay=0.7, density=12, rng=rng)
    if kind == "lightning":
        crack = sp.transient_click(0.003, 0.95)
        zap = sp.band_filter(sp.white_noise(dur, 0.8, rng), 1000, 8000) * sp.envelope_exp_decay(dur, tau=0.1)
        rumble = sp.sine(dur, 60.0, 0.4) * sp.envelope_attack_decay(dur, 0.2, 0.7)
        body = sp.mix(crack, zap, rumble, normalize=False)
        return sp.reverb_simple(body, decay=0.6, density=10, rng=rng)
    if kind == "wind":
        amp = 0.95 if intensity == "storm" else 0.55
        wind = sp.band_filter(sp.brown_noise(dur, amp, rng), 100, 3000)
        wind = sp.amplitude_lfo(wind, rate_hz=0.2 if intensity == "storm" else 0.1, depth=0.4)
        whistle = sp.sine(dur, 800.0, 0.15) * sp.amplitude_lfo(np.ones(sp._n_samples(dur)), rate_hz=0.3, depth=0.5)
        return sp.mix(wind, whistle, normalize=False)
    if kind == "dust":
        wind = sp.band_filter(sp.brown_noise(dur, 0.9, rng), 100, 5000)
        wind = sp.amplitude_lfo(wind, rate_hz=0.3, depth=0.5)
        sand = sp.band_filter(sp.pink_noise(dur, 0.7, rng), 3000, 10000)
        sand = sp.amplitude_lfo(sand, rate_hz=5.0, depth=0.6)
        return sp.mix(wind, sand, normalize=False)
    if kind == "acid_rain":
        body = sp.band_filter(sp.white_noise(dur, 0.65, rng), 3000, 8000)
        body = sp.amplitude_lfo(body, rate_hz=2.0, depth=0.3)
        sizzle = sp.band_filter(sp.white_noise(dur, 0.4, rng), 5000, 12000)
        sizzle = sp.amplitude_lfo(sizzle, rate_hz=0.5, depth=0.3)
        return sp.mix(body, sizzle, normalize=False)
    if kind == "snow":
        body = sp.band_filter(sp.pink_noise(dur, 0.25, rng), 100, 1500)
        body = sp.amplitude_lfo(body, rate_hz=0.1, depth=0.3)
        return body
    if kind == "fog":
        body = sp.band_filter(sp.brown_noise(dur, 0.3, rng), 50, 500)
        drips = np.zeros(sp._n_samples(dur))
        for offset_sec in np.arange(2.0, dur, 4.0):
            offset = int(offset_sec * sp.SAMPLE_RATE)
            dr = sp.sine(0.04, 1200.0, 0.3) * sp.envelope_exp_decay(0.04, tau=0.015)
            end = offset + len(dr)
            if end <= len(drips):
                drips[offset:end] += dr
        return sp.mix(body, drips, normalize=False)
    return _generic_placeholder(rng, dur)


def synth_hazard(entry: Dict, rng: np.random.RandomState) -> np.ndarray:
    dur = float(entry.get("duration_target_sec", 5.0))
    hazard = entry.get("hazard", "")

    if hazard == "fire":
        roar = sp.band_filter(sp.brown_noise(dur, 0.7, rng), 100, 2000)
        roar = sp.amplitude_lfo(roar, rate_hz=1.5, depth=0.3)
        crackle = sp.crackle_pattern(dur, density_per_sec=20.0, peak=0.5, rng=rng)
        crackle = sp.band_filter(crackle, 2000, 8000)
        whoof = sp.sine(dur, 200.0, 0.25) * sp.amplitude_lfo(np.ones(sp._n_samples(dur)), rate_hz=0.5, depth=0.4)
        return sp.mix(roar, crackle, whoof, normalize=False)
    if hazard == "smoke":
        whoosh = sp.band_filter(sp.brown_noise(dur, 0.45, rng), 50, 1500)
        whoosh = sp.amplitude_lfo(whoosh, rate_hz=0.3, depth=0.5)
        return whoosh
    if hazard == "acid":
        hiss = sp.band_filter(sp.white_noise(dur, 0.7, rng), 3000, 10000)
        hiss = sp.amplitude_lfo(hiss, rate_hz=0.2, depth=0.2)
        bubbles = np.zeros(sp._n_samples(dur))
        for offset_sec in np.arange(0.5, dur, 1.5):
            offset = int(offset_sec * sp.SAMPLE_RATE)
            pop = sp.sine(0.05, float(rng.uniform(400, 800)), 0.45) * sp.envelope_exp_decay(0.05, tau=0.02)
            end = offset + len(pop)
            if end <= len(bubbles):
                bubbles[offset:end] += pop
        return sp.mix(hiss, bubbles, normalize=False)
    if hazard == "electric":
        buzz = sp.sine(dur, 60.0, 0.5) + sp.sine(dur, 120.0, 0.35) + sp.sine(dur, 180.0, 0.2)
        cracks = np.zeros(sp._n_samples(dur))
        for offset_sec in np.arange(1.0, dur, 3.0):
            offset = int(offset_sec * sp.SAMPLE_RATE)
            zap = sp.band_filter(sp.white_noise(0.08, 0.7, rng), 1500, 6000) * sp.envelope_exp_decay(0.08, tau=0.03)
            end = offset + len(zap)
            if end <= len(cracks):
                cracks[offset:end] += zap
        return sp.mix(buzz, cracks, normalize=False)
    if hazard == "lava":
        bubble = sp.band_filter(sp.brown_noise(dur, 0.7, rng), 80, 1500)
        bubble = sp.amplitude_lfo(bubble, rate_hz=0.7, depth=0.4)
        pops = sp.crackle_pattern(dur, density_per_sec=3.0, peak=0.4, rng=rng)
        return sp.mix(bubble, pops, normalize=False)
    if hazard == "radiation":
        clicks = sp.crackle_pattern(dur, density_per_sec=10.0, peak=0.6, rng=rng)
        clicks = sp.band_filter(clicks, 1500, 6000)
        return clicks
    if hazard == "vacuum_breach":
        wind = sp.band_filter(sp.white_noise(dur, 0.8, rng), 800, 8000) * sp.envelope_attack_decay(dur, 0.05, 0.6)
        rumble = sp.band_filter(sp.brown_noise(dur, 0.6, rng), 50, 1000) * sp.envelope_attack_decay(dur, 0.1, 0.5)
        whistle = sp.chirp(dur, 2000, 500, amp=0.45) * sp.envelope_attack_decay(dur, 0.1, 0.6)
        return sp.mix(wind, rumble, whistle, normalize=False)
    if hazard == "psy_storm":
        whispers = sp.band_filter(sp.voice_formant(dur, 80.0, [200, 400, 800, 1500], 3.0, 5.0, rng) * 0.4, 200, 3500)
        static_ = sp.band_filter(sp.white_noise(dur, 0.45, rng), 1000, 5000)
        static_ = sp.amplitude_lfo(static_, rate_hz=4.0, depth=0.4)
        heart = np.zeros(sp._n_samples(dur))
        for offset_sec in np.arange(0.0, dur, 1.0):
            offset = int(offset_sec * sp.SAMPLE_RATE)
            beat = sp.sine(0.08, 70.0, 0.55) * sp.envelope_exp_decay(0.08, tau=0.03)
            end = offset + len(beat)
            if end <= len(heart):
                heart[offset:end] += beat
        return sp.mix(whispers, static_, heart, normalize=False)
    if hazard == "gravity_well":
        sub = sp.sine(dur, 35.0, 0.5) + sp.sine(dur, 50.0, 0.35) + sp.sine(dur, 70.0, 0.25)
        shimmer = sp.sine(dur, 2000.0, 0.1) + sp.sine(dur, 3000.0, 0.08)
        shimmer = sp.amplitude_lfo(shimmer, rate_hz=0.5, depth=0.4)
        return sp.mix(sub, shimmer, normalize=False)
    if hazard == "time_warp":
        t = sp.t_axis(dur)
        lfo = 50.0 * np.sin(2.0 * np.pi * 5.0 * t)
        freqs = 200.0 + lfo
        phase = 2.0 * np.pi * np.cumsum(freqs) / sp.SAMPLE_RATE
        warble = np.sin(phase) * 0.6
        resonance = sp.sine(dur, 628.0, 0.3) + sp.sine(dur, 942.0, 0.2)
        return sp.mix(warble, resonance, normalize=False)
    if hazard == "bloodsucker":
        growl = sp.band_filter(sp.brown_noise(dur, 0.75, rng), 100, 800)
        growl = sp.amplitude_lfo(growl, rate_hz=2.0, depth=0.3)
        fund = sp.sine(dur, 80.0, 0.4)
        formant = sp.sine(dur, 120.0, 0.25) + sp.sine(dur, 200.0, 0.18)
        formant = formant * sp.amplitude_lfo(np.ones(sp._n_samples(dur)), rate_hz=4.0, depth=0.3)
        breath = sp.band_filter(sp.white_noise(dur, 0.4, rng), 200, 1500) * sp.envelope_attack_decay(dur, 0.1, 0.4)
        return sp.mix(growl, fund, formant, breath, normalize=False)

    return _generic_placeholder(rng, dur)


def synth_ui(entry: Dict, rng: np.random.RandomState) -> np.ndarray:
    dur = float(entry.get("duration_target_sec", 0.2))
    kind = entry.get("type", "")

    if kind == "button_hover":
        a = sp.sine(dur, 1500.0, 0.55) * sp.envelope_adsr(dur, 0.005, 0.02, 0.5, 0.04)
        b = sp.sine(dur, 2000.0, 0.4) * sp.envelope_adsr(dur, 0.005, 0.015, 0.5, 0.03)
        return sp.mix(a, b, normalize=False)
    if kind == "button_click":
        click = sp.transient_click(0.002, 0.8)
        tone = sp.sine(dur, 800.0, 0.45) * sp.envelope_adsr(dur, 0.001, 0.02, 0.3, 0.04)
        noise = sp.band_filter(sp.pink_noise(0.02, 0.4, rng), 1000, 4000) * sp.envelope_exp_decay(0.02, tau=0.01)
        return sp.ensure_duration(sp.mix(click, tone, noise, normalize=False), dur)
    if kind == "button_disabled":
        thud = sp.sine(dur, 200.0, 0.6) * sp.envelope_attack_decay(dur, 0.01, 0.04)
        return sp.low_pass(thud, 800.0)
    if kind == "menu_open":
        sweep = sp.chirp(dur, 200.0, 2000.0, amp=0.55) * sp.envelope_attack_decay(dur, 0.05, 0.2)
        whoosh = sp.band_filter(sp.white_noise(dur, 0.4, rng), 200, 3000) * sp.envelope_attack_decay(dur, 0.05, 0.15)
        return sp.mix(sweep, whoosh, normalize=False)
    if kind == "menu_close":
        sweep = sp.chirp(dur, 2000.0, 200.0, amp=0.55) * sp.envelope_attack_decay(dur, 0.05, 0.2)
        whoosh = sp.band_filter(sp.white_noise(dur, 0.4, rng), 200, 3000) * sp.envelope_attack_decay(dur, 0.05, 0.15)
        return sp.mix(sweep, whoosh, normalize=False)
    if kind == "tab_switch":
        click = sp.transient_click(0.002, 0.6)
        tone = sp.sine(dur, 1200.0, 0.4) * sp.envelope_adsr(dur, 0.001, 0.02, 0.4, 0.05)
        return sp.mix(click, tone, normalize=False)
    if kind == "typing":
        click = sp.transient_click(0.002, 0.5)
        tone = sp.sine(dur, 1500.0, 0.35) * sp.envelope_adsr(dur, 0.001, 0.01, 0.3, 0.02)
        return sp.mix(click, tone, normalize=False)
    if kind == "radio_static":
        noise = sp.band_filter(sp.white_noise(dur, 0.7, rng), 1500, 4000)
        noise = sp.amplitude_lfo(noise, rate_hz=8.0, depth=0.6)
        return noise * sp.envelope_attack_decay(dur, 0.02, 0.15)
    if kind == "objective_complete":
        half = dur / 2.0
        a = sp.sine(half, 800.0, 0.55) * sp.envelope_adsr(half, 0.02, 0.1, 0.6, 0.2)
        b = sp.sine(half, 1200.0, 0.5) * sp.envelope_adsr(half, 0.02, 0.1, 0.6, 0.2)
        base = np.zeros(sp._n_samples(dur))
        base[:len(a)] += a
        offset = int(half * sp.SAMPLE_RATE * 0.4)
        if offset + len(b) <= len(base):
            base[offset:offset + len(b)] += b
        return base
    if kind == "objective_fail":
        half = dur / 2.0
        a = sp.sine(half, 600.0, 0.55) * sp.envelope_adsr(half, 0.02, 0.1, 0.6, 0.2)
        b = sp.sine(half, 400.0, 0.5) * sp.envelope_adsr(half, 0.02, 0.1, 0.6, 0.2)
        base = np.zeros(sp._n_samples(dur))
        base[:len(a)] += a
        offset = int(half * sp.SAMPLE_RATE * 0.4)
        if offset + len(b) <= len(base):
            base[offset:offset + len(b)] += b
        return base
    if kind == "warning_health":
        n = sp._n_samples(dur)
        base = np.zeros(n)
        beep = sp.sine(0.2, 1000.0, 0.55) * sp.envelope_adsr(0.2, 0.01, 0.04, 0.5, 0.08)
        base[:len(beep)] += beep
        for offset_sec in np.arange(0.5, dur, 0.5):
            offset = int(offset_sec * sp.SAMPLE_RATE)
            heartbeat = sp.sine(0.1, 60.0, 0.6) * sp.envelope_exp_decay(0.1, tau=0.03)
            end = offset + len(heartbeat)
            if end <= n:
                base[offset:end] += heartbeat
        return base
    if kind == "warning_ammo":
        click = sp.transient_click(0.003, 0.7)
        tone = sp.sine(dur, 800.0, 0.5) * sp.envelope_adsr(dur, 0.01, 0.05, 0.5, 0.1)
        return sp.mix(click, tone, normalize=False)
    if kind == "damage_taken":
        sub = sp.sine(min(dur, 0.15), 50.0, 0.85) * sp.envelope_exp_decay(min(dur, 0.15), tau=0.05)
        click = sp.transient_click(0.003, 0.7)
        mid = sp.sine(min(dur, 0.15), 200.0, 0.55) * sp.envelope_exp_decay(min(dur, 0.15), tau=0.04)
        return sp.mix(sub, click, mid, normalize=False)
    if kind == "kill_confirm":
        ring1 = sp.sine(min(dur, 0.1), 2000.0, 0.55) * sp.envelope_adsr(min(dur, 0.1), 0.005, 0.02, 0.6, 0.04)
        ring2 = sp.sine(min(dur, 0.1), 3000.0, 0.4) * sp.envelope_adsr(min(dur, 0.1), 0.005, 0.02, 0.6, 0.04)
        click = sp.transient_click(0.002, 0.6)
        reward = sp.sine(min(dur, 0.15), 1500.0, 0.45) * sp.envelope_adsr(min(dur, 0.15), 0.01, 0.04, 0.5, 0.06)
        return sp.mix(ring1, ring2, click, reward, normalize=False)
    if kind == "headshot":
        ring1 = sp.sine(dur, 1500.0, 0.55) * sp.envelope_adsr(dur, 0.005, 0.04, 0.6, 0.1)
        ring2 = sp.sine(dur, 2500.0, 0.5) * sp.envelope_adsr(dur, 0.005, 0.04, 0.6, 0.1)
        body = sp.mix(ring1, ring2, normalize=False)
        return sp.reverb_simple(body, decay=0.35, density=8, rng=rng)
    if kind == "level_up":
        freqs = [800.0, 1000.0, 1200.0, 1500.0, 2000.0]
        base = np.zeros(sp._n_samples(dur))
        for i, f in enumerate(freqs):
            tone = sp.sine(0.4, f, 0.5) * sp.envelope_adsr(0.4, 0.01, 0.05, 0.5, 0.15)
            offset = int(i * 0.12 * sp.SAMPLE_RATE)
            end = offset + len(tone)
            if end <= len(base):
                base[offset:end] += tone
            elif offset < len(base):
                base[offset:] += tone[:len(base) - offset]
        return base
    if kind == "pickup":
        t = sp.t_axis(dur)
        sweep_t = t / max(dur, 1e-6)
        f1 = 1500.0 + 1500.0 * sweep_t
        f2 = 2000.0 + 1500.0 * sweep_t
        f3 = 3000.0 + 1500.0 * sweep_t
        phase1 = 2.0 * np.pi * np.cumsum(f1) / sp.SAMPLE_RATE
        phase2 = 2.0 * np.pi * np.cumsum(f2) / sp.SAMPLE_RATE
        phase3 = 2.0 * np.pi * np.cumsum(f3) / sp.SAMPLE_RATE
        sparkle = np.sin(phase1) * 0.4 + np.sin(phase2) * 0.3 + np.sin(phase3) * 0.2
        env = sp.envelope_adsr(dur, 0.02, 0.05, 0.6, 0.1)
        return sparkle * env
    if kind == "drop":
        thud = sp.sine(min(dur, 0.15), 200.0, 0.55) * sp.envelope_exp_decay(min(dur, 0.15), tau=0.06)
        clink = sp.sine(min(dur - 0.1, 0.05), 1500.0, 0.4) * sp.envelope_exp_decay(min(dur - 0.1, 0.05), tau=0.02)
        base = np.zeros(sp._n_samples(dur))
        base[:len(thud)] += thud
        offset = int(0.1 * sp.SAMPLE_RATE)
        if offset + len(clink) <= len(base):
            base[offset:offset + len(clink)] += clink
        return base
    if kind == "settings_save":
        a = sp.sine(dur, 1000.0, 0.5) * sp.envelope_adsr(dur, 0.02, 0.05, 0.5, 0.15)
        b = sp.sine(dur, 1500.0, 0.45) * sp.envelope_adsr(dur, 0.04, 0.05, 0.5, 0.15)
        return sp.mix(a, b, normalize=False)
    if kind == "loading_complete":
        a = sp.sine(dur, 1000.0, 0.55) * sp.envelope_adsr(dur, 0.02, 0.05, 0.5, 0.15)
        b = sp.sine(dur, 1500.0, 0.5) * sp.envelope_adsr(dur, 0.04, 0.05, 0.5, 0.15)
        base = np.zeros(sp._n_samples(dur))
        base[:len(a)] += a
        offset = int(min(dur - 0.3, 0.3) * sp.SAMPLE_RATE)
        offset = max(0, offset)
        if offset + len(b) <= len(base):
            base[offset:offset + len(b)] += b
        return base

    return _generic_placeholder(rng, dur)


def synth_chatter(entry: Dict, rng: np.random.RandomState) -> np.ndarray:
    dur = float(entry.get("duration_target_sec", 1.5))
    actor_role = entry.get("actor_role", "any")
    prompt = entry.get("prompt", "")
    female = "female voice" in prompt
    f0 = 220.0 if female else 130.0
    formants = [400.0, 800.0, 1600.0, 2400.0] if female else [300.0, 700.0, 1200.0, 2000.0]

    t = sp.t_axis(dur)
    sweep_t = t / max(dur, 1e-6)
    prosody = 1.0 + 0.35 * np.sin(np.pi * sweep_t * 2.0) - 0.15 * sweep_t
    vibrato = 5.0 * np.sin(2.0 * np.pi * 5.0 * t)
    freqs = f0 * prosody + vibrato
    phase = 2.0 * np.pi * np.cumsum(freqs) / sp.SAMPLE_RATE
    base = np.sin(phase) * 0.55
    for fm in formants:
        ratio = fm / f0
        base = base + np.sin(phase * ratio) * 0.25 / max(ratio, 1.0)

    syllable_lfo = np.zeros_like(base)
    syll_per_sec = 4.5
    n_syll = max(2, int(dur * syll_per_sec))
    for i in range(n_syll):
        center = (i + 0.5) / n_syll
        width = 0.7 / n_syll
        gauss = np.exp(-((sweep_t - center) ** 2) / (2.0 * (width ** 2)))
        syllable_lfo += gauss
    syllable_lfo = syllable_lfo / max(np.max(syllable_lfo), 1e-6)
    base = base * (0.4 + 0.7 * syllable_lfo)

    base = sp.band_filter(base, 300, 3400)
    base = base * sp.envelope_attack_decay(dur, 0.02, 0.3)
    crackle_start = sp.band_filter(sp.white_noise(0.06, 0.35, rng), 1500, 4500) * sp.envelope_exp_decay(0.06, tau=0.02)
    crackle_end = sp.band_filter(sp.white_noise(0.05, 0.3, rng), 1500, 4500) * sp.envelope_exp_decay(0.05, tau=0.02)
    out = base.copy()
    out[:len(crackle_start)] += crackle_start
    end_off = max(0, len(out) - len(crackle_end))
    out[end_off:end_off + len(crackle_end)] += crackle_end
    return out


# ─── Fallback ───────────────────────────────────────────────────────────────


def _generic_placeholder(rng: np.random.RandomState, dur: float) -> np.ndarray:
    body = sp.band_filter(sp.pink_noise(dur, 0.6, rng), 500, 4000)
    env = sp.envelope_attack_decay(dur, 0.02, 0.1)
    return body * env


# ─── Dispatch ───────────────────────────────────────────────────────────────


def dispatch(section: str, entry: Dict, rng: np.random.RandomState) -> np.ndarray:
    if section == "weapon_action_sfx":
        return synth_weapon(entry, rng)
    if section == "footstep_sfx":
        return synth_footstep(entry, rng)
    if section == "locomotion_sfx":
        return synth_locomotion(entry, rng)
    if section == "projectile_sfx":
        return synth_projectile(entry, rng)
    if section == "impact_sfx_by_material":
        return synth_impact(entry, rng)
    if section == "body_hit_sfx":
        return synth_body_hit(entry, rng)
    if section == "dismemberment_sfx":
        return synth_dismember(entry, rng)
    if section == "death_sfx":
        return synth_death(entry, rng)
    if section == "ambient_loops":
        return synth_ambient(entry, rng)
    if section == "weather_sfx":
        return synth_weather(entry, rng)
    if section == "hazard_sfx":
        return synth_hazard(entry, rng)
    if section == "ui_sfx":
        return synth_ui(entry, rng)
    if section == "ai_chatter_prompts":
        return synth_chatter(entry, rng)
    return _generic_placeholder(rng, float(entry.get("duration_target_sec", 1.0)))


__all__ = ["dispatch"]
