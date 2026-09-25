"""Synthesise a placeholder 120 BPM track and the reel's UI sounds.

The track is only there so the pipeline can run end to end without a
download: swap in a real royalty-free song (Mixkit and similar libraries
license theirs for commercial use) and `analyze.py` finds its grid the same
way. It starts with a little silence and a pickup, so the analysis has to
find the first downbeat rather than assume it is at zero.

    python placeholder.py <out-dir>
"""
import os
import sys

import numpy as np

from wavio import write

RATE = 48000


def envelope(length, attack, decay):
    t = np.arange(length) / RATE
    return np.minimum(t / max(attack, 1e-4), 1.0) * np.exp(-t / decay)


def tone(freq, seconds, decay, attack=0.002, shape="sine"):
    t = np.arange(int(seconds * RATE)) / RATE
    phase = 2 * np.pi * np.cumsum(np.broadcast_to(freq, t.shape)) / RATE
    wave = np.sin(phase) if shape == "sine" else 2 * ((phase / (2 * np.pi)) % 1.0) - 1
    return wave * envelope(len(t), attack, decay)


def noise(seconds, decay, rng, color=0.0):
    n = rng.standard_normal(int(seconds * RATE))
    if color:
        # One-pole low-pass: `color` near 1 is darker.
        out = np.empty_like(n)
        acc = 0.0
        for i, x in enumerate(n):
            acc = color * acc + (1 - color) * x
            out[i] = acc
        n = out / (np.abs(out).max() + 1e-9)
    return n * envelope(len(n), 0.001, decay)


def highpass(x, amount=0.95):
    return np.concatenate([[x[0]], x[1:] - amount * x[:-1]])


def place(track, sound, at, gain=1.0):
    start = int(at * RATE)
    if start >= len(track):
        return
    end = min(len(track), start + len(sound))
    track[start:end] += sound[: end - start] * gain


def song(bars=16, bpm=120.0, lead=0.37, seed=7):
    rng = np.random.default_rng(seed)
    beat = 60.0 / bpm
    length = lead + (bars + 1) * 4 * beat
    track = np.zeros(int(length * RATE))
    kick = tone(np.geomspace(140, 42, int(0.35 * RATE)), 0.35, 0.16)
    clap = highpass(noise(0.2, 0.05, rng)) * 0.6
    hat = highpass(noise(0.05, 0.012, rng), 0.99) * 0.35
    # Am - F - C - G, one chord a bar.
    roots = [110.0, 87.31, 130.81, 98.0]
    chords = [[220.0, 261.63, 329.63], [174.61, 220.0, 261.63], [261.63, 329.63, 392.0], [196.0, 246.94, 293.66]]
    # A pickup: two hats before the first downbeat.
    place(track, hat, lead - beat, 0.6)
    place(track, hat, lead - beat / 2, 0.6)
    for bar in range(bars):
        start = lead + bar * 4 * beat
        chord = chords[bar % 4]
        pad = sum(tone(f, 4 * beat, 2.5, attack=0.08) for f in chord) / len(chord)
        place(track, pad, start, 0.16)
        for b in range(4):
            t = start + b * beat
            place(track, kick, t, 0.9)
            if b in (1, 3):
                place(track, clap, t, 0.7)
            for eighth in range(2):
                place(track, hat, t + eighth * beat / 2, 0.5 if eighth else 0.3)
                bass = tone(roots[bar % 4] / 2 * (1 if eighth == 0 else 2), beat / 2, 0.12, shape="saw")
                place(track, bass, t + eighth * beat / 2, 0.22)
    # Let the last bar ring out.
    track /= np.abs(track).max() + 1e-9
    return (track * 0.85).astype(np.float32)


def add(*parts, delays=None):
    """Sounds laid over each other, each `delays[i]` seconds in."""
    delays = delays or [0.0] * len(parts)
    length = max(int(d * RATE) + len(p) for p, d in zip(parts, delays))
    out = np.zeros(length)
    for p, d in zip(parts, delays):
        i = int(d * RATE)
        out[i : i + len(p)] += p
    return out


def sounds(seed=3):
    rng = np.random.default_rng(seed)
    blip = lambda f, d, g=1.0: tone(f, d * 6, d) * g
    tick = lambda s, d, g: highpass(noise(s, d, rng)) * g
    return {
        "press": add(blip(1300, 0.014), tick(0.03, 0.004, 0.3)),
        "release": add(blip(2600, 0.01, 0.7), tick(0.02, 0.003, 0.2)),
        "click": add(blip(2200, 0.012), tick(0.03, 0.004, 0.4)),
        "toggle": add(blip(1700, 0.01), blip(2500, 0.012), delays=[0.0, 0.035]),
        "success": add(tone(1318.5, 0.5, 0.12), tone(1975.5, 0.5, 0.14) * 0.8, delays=[0.0, 0.08]),
        "type": add(highpass(noise(0.03, 0.003, rng), 0.98) * 0.6, blip(3200, 0.004, 0.3)),
        "enter": add(blip(900, 0.02), tick(0.05, 0.006, 0.3)),
        "pop": tone(np.linspace(520, 980, int(0.09 * RATE)), 0.09, 0.03, attack=0.004),
    }


def main():
    out = sys.argv[1] if len(sys.argv) > 1 else "."
    os.makedirs(os.path.join(out, "sounds"), exist_ok=True)
    write(os.path.join(out, "song.wav"), np.repeat(song()[:, None], 2, axis=1), RATE)
    for name, sound in sounds().items():
        sound = sound / (np.abs(sound).max() + 1e-9) * 0.8
        write(os.path.join(out, "sounds", f"{name}.wav"), sound.astype(np.float32), RATE)
    print(f"Wrote {out}/song.wav and {out}/sounds/*.wav")


if __name__ == "__main__":
    main()
