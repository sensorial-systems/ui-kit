"""Render the morph reel with sound, from a song to an MP4.

    python examples/morph-reel/render.py [--song path] [--sounds dir] [--laps 2]
        [--accent 3366ff] [--out target/morph-reel/reel.mp4]

Without `--song` a placeholder track and UI sounds are synthesised. Steps:
analyse the song's beat grid and each sound's peak (numpy), export the reel's
cues on that grid, mix, render one frame per beat for review, then render the
motion-blurred 60 fps video and mux the mix in.
"""
import argparse
import json
import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
AUDIO = os.path.join(HERE, "audio")


def run(*command):
    print("+", " ".join(command))
    subprocess.run(command, check=True)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--song")
    parser.add_argument("--sounds")
    parser.add_argument("--laps", default="2")
    parser.add_argument("--accent")
    parser.add_argument("--work", default=os.path.join("target", "morph-reel"))
    parser.add_argument("--out")
    parser.add_argument("--skip-beats", action="store_true")
    args = parser.parse_args()
    work = args.work
    os.makedirs(work, exist_ok=True)
    python = sys.executable

    song, sounds = args.song, args.sounds
    if not song or not sounds:
        run(python, os.path.join(AUDIO, "placeholder.py"), work)
        song = song or os.path.join(work, "song.wav")
        sounds = sounds or os.path.join(work, "sounds")

    beats = os.path.join(work, "beats.json")
    run(python, os.path.join(AUDIO, "analyze.py"), song, "--sounds", sounds, "--out", beats)
    bpm = str(json.load(open(beats))["bpm"])

    run("cargo", "build", "-q", "--release", "-p", "morph-reel")
    tool = os.path.join("target", "release", "morph-reel")
    common = ["--bpm", bpm] + (["--accent", args.accent] if args.accent else [])
    cues = os.path.join(work, "cues.json")
    run(tool, "cues", cues, *common)
    mix = os.path.join(work, "mix.wav")
    run(python, os.path.join(AUDIO, "mix.py"), "--song", song, "--beats", beats,
        "--cues", cues, "--sounds", sounds, "--laps", args.laps, "--out", mix)
    if not args.skip_beats:
        run(tool, "beats", os.path.join(work, "beats"), *common)
    out = args.out or os.path.join(work, "reel.mp4")
    run(tool, "render", out, "--audio", mix, "--laps", args.laps, *common)


if __name__ == "__main__":
    main()
