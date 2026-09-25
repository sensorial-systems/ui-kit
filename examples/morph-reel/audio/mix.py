"""Mix the song from its first downbeat with every UI sound on its cue.

    python mix.py --song song.wav --beats beats.json --cues cues.json \
        --sounds sounds --laps 2 --out mix.wav

Each sound is placed so its measured peak (from `analyze.py`) lands on the
cue's time, lap after lap. The song is trimmed to start on the downbeat the
reel starts on, and fades out over the last half second.
"""
import argparse
import json
import os

import numpy as np

from wavio import read, write

RATE = 48000
GAIN = {
    "press": 0.35,
    "release": 0.3,
    "click": 0.4,
    "toggle": 0.4,
    "success": 0.3,
    "type": 0.3,
    "enter": 0.4,
    "pop": 0.35,
}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--song", required=True)
    parser.add_argument("--beats", required=True)
    parser.add_argument("--cues", required=True)
    parser.add_argument("--sounds", required=True)
    parser.add_argument("--laps", type=int, default=2)
    parser.add_argument("--out", default="mix.wav")
    args = parser.parse_args()
    beats = json.load(open(args.beats))
    cues = json.load(open(args.cues))
    peaks = beats.get("peaks", {})

    song, _ = read(args.song, rate=RATE)
    if song.shape[1] == 1:
        song = np.repeat(song, 2, axis=1)
    length = int(cues["duration"] * args.laps * RATE)
    start = int(beats["first_downbeat"] * RATE)
    out = np.zeros((length, 2), dtype=np.float32)
    part = song[start : start + length]
    out[: len(part)] = part * 0.8

    missing = set()
    for lap in range(args.laps):
        for cue in cues["cues"]:
            path = os.path.join(args.sounds, cue["sound"] + ".wav")
            if not os.path.exists(path):
                missing.add(cue["sound"])
                continue
            sound, _ = read(path, rate=RATE)
            if sound.shape[1] == 1:
                sound = np.repeat(sound, 2, axis=1)
            at = lap * cues["duration"] + cue["time"] - peaks.get(cue["sound"], 0.0)
            i = int(round(at * RATE))
            if i < 0:
                sound, i = sound[-i:], 0
            end = min(length, i + len(sound))
            out[i:end] += sound[: end - i] * GAIN.get(cue["sound"], 0.35)

    fade = int(0.5 * RATE)
    out[-fade:] *= np.linspace(1.0, 0.0, fade)[:, None]
    out /= max(1.0, np.abs(out).max() / 0.98)
    write(args.out, out, RATE)
    note = f" (no file for: {', '.join(sorted(missing))})" if missing else ""
    print(f"Wrote {args.out}: {args.laps} laps, {len(cues['cues'])} cues each{note}")


if __name__ == "__main__":
    main()
