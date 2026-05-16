import numpy as np

SAMPLE_RATE = 48000

KEY_OFFSETS = {
    "C": 0, "C#": 1, "Db": 1, "D": 2, "D#": 3, "Eb": 3,
    "E": 4, "F": 5, "F#": 6, "Gb": 6, "G": 7, "G#": 8,
    "Ab": 8, "A": 9, "A#": 10, "Bb": 10, "B": 11,
}
SCALE_MAJOR = [0, 2, 4, 5, 7, 9, 11]
SCALE_MINOR = [0, 2, 3, 5, 7, 8, 10]


def note_freq(semitone_offset, octave=4):
    return 440.0 * 2.0 ** ((semitone_offset + (octave - 4) * 12 - 9) / 12.0)


def parse_key(key_str):
    parts = key_str.split()
    note = parts[0]
    scale = SCALE_MAJOR if "major" in key_str.lower() else SCALE_MINOR
    return KEY_OFFSETS[note], scale


def scale_notes(root_offset, scale, octave_low=2, octave_high=5):
    notes = []
    for octave in range(octave_low, octave_high + 1):
        for s in scale:
            notes.append(note_freq(root_offset + s, octave))
    return notes


def adsr(dur_sec, a=0.05, d=0.1, s=0.6, r=0.3):
    n = int(dur_sec * SAMPLE_RATE)
    if n <= 0:
        return np.zeros(0)
    env = np.zeros(n)
    a_n = max(1, int(a * SAMPLE_RATE))
    d_n = max(1, int(d * SAMPLE_RATE))
    r_n = max(1, int(r * SAMPLE_RATE))
    s_n = max(1, n - a_n - d_n - r_n)
    a_n = min(a_n, n)
    d_n = min(d_n, max(0, n - a_n))
    r_n = min(r_n, max(0, n - a_n - d_n))
    s_n = max(0, n - a_n - d_n - r_n)
    cursor = 0
    if a_n > 0:
        env[cursor:cursor + a_n] = np.linspace(0, 1, a_n)
        cursor += a_n
    if d_n > 0:
        env[cursor:cursor + d_n] = np.linspace(1, s, d_n)
        cursor += d_n
    if s_n > 0:
        env[cursor:cursor + s_n] = s
        cursor += s_n
    if r_n > 0:
        env[cursor:cursor + r_n] = np.linspace(s, 0, r_n)
        cursor += r_n
    return env[:n]


def sine_note(freq, dur_sec, amp=1.0, env=None):
    n = int(dur_sec * SAMPLE_RATE)
    if n <= 0:
        return np.zeros(0)
    t = np.arange(n) / SAMPLE_RATE
    samples = amp * np.sin(2 * np.pi * freq * t)
    if env is not None:
        samples = samples * env[:len(samples)]
    return samples


def saw_note(freq, dur_sec, amp=1.0, env=None):
    n = int(dur_sec * SAMPLE_RATE)
    if n <= 0:
        return np.zeros(0)
    t = np.arange(n) / SAMPLE_RATE
    samples = np.zeros(n)
    for h in range(1, 6):
        samples += (amp / h) * np.sin(2 * np.pi * freq * h * t)
    if env is not None:
        samples = samples * env[:len(samples)]
    return samples * 0.5


def triangle_note(freq, dur_sec, amp=1.0, env=None):
    n = int(dur_sec * SAMPLE_RATE)
    if n <= 0:
        return np.zeros(0)
    t = np.arange(n) / SAMPLE_RATE
    samples = np.zeros(n)
    for h in [1, 3, 5, 7]:
        samples += (amp / (h * h)) * np.sin(2 * np.pi * freq * h * t)
    if env is not None:
        samples = samples * env[:len(samples)]
    return samples * 0.5


def square_note(freq, dur_sec, amp=1.0, env=None):
    n = int(dur_sec * SAMPLE_RATE)
    if n <= 0:
        return np.zeros(0)
    t = np.arange(n) / SAMPLE_RATE
    samples = np.zeros(n)
    for h in [1, 3, 5, 7, 9]:
        samples += (amp / h) * np.sin(2 * np.pi * freq * h * t)
    if env is not None:
        samples = samples * env[:len(samples)]
    return samples * 0.4


def organ_note(freq, dur_sec, amp=1.0, env=None):
    n = int(dur_sec * SAMPLE_RATE)
    if n <= 0:
        return np.zeros(0)
    t = np.arange(n) / SAMPLE_RATE
    samples = np.zeros(n)
    for h, mul in [(1, 1.0), (2, 0.5), (3, 0.3), (4, 0.4), (6, 0.2), (8, 0.15)]:
        samples += amp * mul * np.sin(2 * np.pi * freq * h * t)
    samples /= 2.55
    if env is not None:
        samples = samples * env[:len(samples)]
    return samples


def bell_note(freq, dur_sec, amp=1.0, env=None):
    n = int(dur_sec * SAMPLE_RATE)
    if n <= 0:
        return np.zeros(0)
    t = np.arange(n) / SAMPLE_RATE
    samples = np.zeros(n)
    for h, mul, decay in [(1.0, 1.0, 1.0), (2.0, 0.5, 1.4), (3.0, 0.3, 1.8), (4.2, 0.4, 2.2), (5.4, 0.25, 2.6)]:
        samples += amp * mul * np.sin(2 * np.pi * freq * h * t) * np.exp(-t * decay)
    samples /= 2.45
    if env is not None:
        samples = samples * env[:len(samples)]
    return samples


def chord_freqs(root_offset, scale_degree, scale, octave=3):
    base = root_offset + scale[scale_degree % 7] + (scale_degree // 7) * 12
    third = root_offset + scale[(scale_degree + 2) % 7] + ((scale_degree + 2) // 7) * 12
    fifth = root_offset + scale[(scale_degree + 4) % 7] + ((scale_degree + 4) // 7) * 12
    return [note_freq(base, octave), note_freq(third, octave), note_freq(fifth, octave)]


def pad_chord(freqs, dur_sec, amp=0.3, voice="sine"):
    n = int(dur_sec * SAMPLE_RATE)
    if n <= 0:
        return np.zeros(0)
    env = adsr(dur_sec, a=min(0.5, dur_sec * 0.2), d=min(0.5, dur_sec * 0.1), s=0.7, r=min(0.5, dur_sec * 0.2))
    result = np.zeros(n)
    per_amp = amp / max(1, len(freqs))
    for f in freqs:
        if voice == "saw":
            result += saw_note(f, dur_sec, amp=per_amp, env=env)[:n]
        elif voice == "triangle":
            result += triangle_note(f, dur_sec, amp=per_amp, env=env)[:n]
        elif voice == "organ":
            result += organ_note(f, dur_sec, amp=per_amp, env=env)[:n]
        elif voice == "bell":
            result += bell_note(f, dur_sec, amp=per_amp, env=env)[:n]
        else:
            result += sine_note(f, dur_sec, amp=per_amp, env=env)[:n]
    return result


def kick(at_sec, dur_total_sec, amp=0.6):
    samples = np.zeros(int(dur_total_sec * SAMPLE_RATE))
    kick_dur = 0.15
    n = int(kick_dur * SAMPLE_RATE)
    t = np.arange(n) / SAMPLE_RATE
    freqs = 100 * np.exp(-t * 20) + 40
    pitch = np.cumsum(2 * np.pi * freqs / SAMPLE_RATE)
    kick_samples = amp * np.sin(pitch) * np.exp(-t * 15)
    start_n = int(at_sec * SAMPLE_RATE)
    end_n = min(start_n + n, len(samples))
    if start_n < len(samples) and end_n > start_n:
        samples[start_n:end_n] += kick_samples[:end_n - start_n]
    return samples


def snare(at_sec, dur_total_sec, amp=0.4, rng=None):
    samples = np.zeros(int(dur_total_sec * SAMPLE_RATE))
    snare_dur = 0.1
    n = int(snare_dur * SAMPLE_RATE)
    if rng is None:
        rng = np.random.default_rng(12345)
    noise = rng.standard_normal(n) * np.exp(-np.arange(n) / SAMPLE_RATE * 25)
    t = np.arange(n) / SAMPLE_RATE
    tonal = 0.3 * np.sin(2 * np.pi * 200 * t) * np.exp(-t * 20)
    snare_samples = amp * (noise * 0.7 + tonal)
    start_n = int(at_sec * SAMPLE_RATE)
    end_n = min(start_n + n, len(samples))
    if start_n < len(samples) and end_n > start_n:
        samples[start_n:end_n] += snare_samples[:end_n - start_n]
    return samples


def hihat(at_sec, dur_total_sec, amp=0.2, rng=None):
    samples = np.zeros(int(dur_total_sec * SAMPLE_RATE))
    hihat_dur = 0.05
    n = int(hihat_dur * SAMPLE_RATE)
    if rng is None:
        rng = np.random.default_rng(54321)
    noise = rng.standard_normal(n) * np.exp(-np.arange(n) / SAMPLE_RATE * 50)
    fft = np.fft.rfft(noise)
    freqs = np.fft.rfftfreq(n, 1.0 / SAMPLE_RATE)
    fft[freqs < 5000] *= 0.1
    noise_filtered = np.fft.irfft(fft, n) * amp
    start_n = int(at_sec * SAMPLE_RATE)
    end_n = min(start_n + n, len(samples))
    if start_n < len(samples) and end_n > start_n:
        samples[start_n:end_n] += noise_filtered[:end_n - start_n]
    return samples


def tom(at_sec, dur_total_sec, amp=0.4, base_freq=80.0):
    samples = np.zeros(int(dur_total_sec * SAMPLE_RATE))
    tom_dur = 0.18
    n = int(tom_dur * SAMPLE_RATE)
    t = np.arange(n) / SAMPLE_RATE
    freqs = base_freq * np.exp(-t * 8) + base_freq * 0.5
    pitch = np.cumsum(2 * np.pi * freqs / SAMPLE_RATE)
    tom_samples = amp * np.sin(pitch) * np.exp(-t * 8)
    start_n = int(at_sec * SAMPLE_RATE)
    end_n = min(start_n + n, len(samples))
    if start_n < len(samples) and end_n > start_n:
        samples[start_n:end_n] += tom_samples[:end_n - start_n]
    return samples


def melody_line(note_indices, freqs_in_key, note_dur=0.5, total_dur=None, amp=0.4, voice="saw"):
    if total_dur is None:
        total_dur = len(note_indices) * note_dur
    n_total = int(total_dur * SAMPLE_RATE)
    samples = np.zeros(n_total)
    n_per_note = int(note_dur * SAMPLE_RATE)
    n_indices = len(note_indices)
    if n_indices == 0 or n_per_note <= 0:
        return samples
    cycle_count = int(np.ceil(total_dur / (n_indices * note_dur)))
    for cyc in range(cycle_count):
        for i, n_idx in enumerate(note_indices):
            start_sec = cyc * n_indices * note_dur + i * note_dur
            if start_sec >= total_dur:
                break
            f = freqs_in_key[n_idx % len(freqs_in_key)]
            env = adsr(note_dur, a=0.02, d=0.1, s=0.5, r=0.2)
            if voice == "saw":
                note = saw_note(f, note_dur, amp=amp, env=env)
            elif voice == "triangle":
                note = triangle_note(f, note_dur, amp=amp, env=env)
            elif voice == "square":
                note = square_note(f, note_dur, amp=amp, env=env)
            elif voice == "bell":
                note = bell_note(f, note_dur, amp=amp, env=env)
            elif voice == "organ":
                note = organ_note(f, note_dur, amp=amp, env=env)
            else:
                note = sine_note(f, note_dur, amp=amp, env=env)
            start_n = int(start_sec * SAMPLE_RATE)
            end_n = min(start_n + len(note), n_total)
            if start_n < n_total and end_n > start_n:
                samples[start_n:end_n] += note[:end_n - start_n]
    return samples


def bass_line(note_indices, freqs_in_key, note_dur=1.0, total_dur=None, amp=0.5, octave_drop=2, voice="triangle"):
    bass_freqs = [f / (2 ** octave_drop) for f in freqs_in_key]
    return melody_line(note_indices, bass_freqs, note_dur, total_dur, amp, voice=voice)


def drum_pattern(pattern_str, bpm, total_dur, rng=None, kick_amp=0.5, snare_amp=0.3, hihat_amp=0.15):
    if rng is None:
        rng = np.random.default_rng(2718)
    samples = np.zeros(int(total_dur * SAMPLE_RATE))
    if not pattern_str:
        return samples
    step_sec = 60.0 / bpm / 4
    cycle_dur = len(pattern_str) * step_sec
    if cycle_dur <= 0:
        return samples
    n_cycles = int(total_dur / cycle_dur) + 2
    for cyc in range(n_cycles):
        for i, c in enumerate(pattern_str):
            t = cyc * cycle_dur + i * step_sec
            if t >= total_dur:
                break
            if c == "k":
                samples += kick(t, total_dur, amp=kick_amp)
            elif c == "s":
                samples += snare(t, total_dur, amp=snare_amp, rng=rng)
            elif c == "h":
                samples += hihat(t, total_dur, amp=hihat_amp, rng=rng)
            elif c == "t":
                samples += tom(t, total_dur, amp=kick_amp * 0.6, base_freq=120.0)
            elif c == "T":
                samples += tom(t, total_dur, amp=kick_amp * 0.6, base_freq=80.0)
    return samples


def noise_layer(dur_sec, amp=0.05, hp_cutoff_hz=200.0, rng=None):
    if rng is None:
        rng = np.random.default_rng(31415)
    n = int(dur_sec * SAMPLE_RATE)
    if n <= 0:
        return np.zeros(0)
    noise = rng.standard_normal(n) * amp
    fft = np.fft.rfft(noise)
    freqs = np.fft.rfftfreq(n, 1.0 / SAMPLE_RATE)
    fft[freqs < hp_cutoff_hz] *= 0.1
    return np.fft.irfft(fft, n)


def reverb(samples, decay=0.3, density=10):
    out = samples.copy()
    for i in range(1, density):
        delay_sec = 0.02 + i * 0.018
        delay_n = int(delay_sec * SAMPLE_RATE)
        if delay_n >= len(samples):
            continue
        amp = decay ** i
        out[delay_n:] += samples[:-delay_n] * amp
    return out


def stereo_pan(mono, pan=0.0):
    left = mono * (0.5 - pan * 0.5)
    right = mono * (0.5 + pan * 0.5)
    return left, right


def normalize(samples, peak_dbfs=-8.0):
    target = 10 ** (peak_dbfs / 20.0)
    max_val = np.max(np.abs(samples)) if samples.size else 0.0
    if max_val > 0:
        return samples * (target / max_val)
    return samples


def fade_in_out(samples, fade_ms=50.0):
    n_fade = int(fade_ms / 1000.0 * SAMPLE_RATE)
    if n_fade * 2 > len(samples):
        n_fade = len(samples) // 2
    if n_fade <= 0:
        return samples
    samples[:n_fade] *= np.linspace(0, 1, n_fade)
    samples[-n_fade:] *= np.linspace(1, 0, n_fade)
    return samples


def write_stereo(path, left, right, sample_rate=SAMPLE_RATE):
    import soundfile as sf
    n = max(len(left), len(right))
    if len(left) < n:
        left = np.concatenate([left, np.zeros(n - len(left))])
    if len(right) < n:
        right = np.concatenate([right, np.zeros(n - len(right))])
    stereo = np.stack([left, right], axis=1)
    sf.write(path, np.clip(stereo, -1.0, 1.0), sample_rate, subtype="PCM_16")
