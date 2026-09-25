"""Find a song's beat grid, and where each UI sound peaks, with numpy.

    python analyze.py <song> [--sounds <dir>] [--out beats.json]

The tempo comes from the autocorrelation of an onset envelope (spectral flux),
kept near 120 BPM; the phase is the offset whose beats land on the strongest
onsets; the downbeat is the beat of the bar the kick drum leans on hardest.
Every UI sound's peak is measured so the mix can put that peak -- not the
file's first sample -- on the moment the UI does its thing.
"""
import argparse
import glob
import json
import os

import numpy as np

from wavio import read

RATE = 22050
HOP = 256
FFT = 2048


def onsets(x):
    """Spectral flux per hop, over all bins and over the low band only."""
    window = np.hanning(FFT)
    frames = 1 + (len(x) - FFT) // HOP
    index = np.arange(FFT)[None, :] + HOP * np.arange(frames)[:, None]
    spectrum = np.abs(np.fft.rfft(x[index] * window, axis=1))
    log = np.log1p(spectrum * 10.0)
    flux = np.maximum(np.diff(log, axis=0), 0.0)
    low_bins = int(160 / (RATE / FFT))
    full = flux.sum(axis=1)
    low = flux[:, :low_bins].sum(axis=1)
    norm = lambda e: (e - e.mean()) / (e.std() + 1e-9)
    return norm(full), norm(low)


def tempo(envelope, lo=90.0, hi=150.0, prefer=120.0):
    """The beat period in hops, from the envelope's autocorrelation."""
    e = envelope - envelope.mean()
    ac = np.correlate(e, e, mode="full")[len(e) - 1 :]
    hop_seconds = HOP / RATE
    lags = np.arange(len(ac))
    bpm = 60.0 / np.maximum(lags * hop_seconds, 1e-9)
    usable = (bpm >= lo) & (bpm <= hi)
    # A gentle preference for the expected tempo breaks octave ties.
    weight = np.exp(-0.5 * (np.log2(np.maximum(bpm, 1e-9) / prefer) / 0.4) ** 2)
    score = np.where(usable, ac * weight, -np.inf)
    lag = int(np.argmax(score))
    # Parabolic refinement around the peak.
    if 1 <= lag < len(ac) - 1:
        a, b, c = ac[lag - 1], ac[lag], ac[lag + 1]
        denominator = a - 2 * b + c
        if denominator != 0:
            lag = lag + 0.5 * (a - c) / denominator
    return float(lag)


def phase(envelope, period):
    """The offset, in hops, whose beats sit on the strongest onsets."""
    positions = np.arange(0, len(envelope) - 1, period)
    best, best_score = 0.0, -np.inf
    for offset in np.arange(0, period, 0.25):
        at = positions + offset
        at = at[at < len(envelope) - 1]
        i = at.astype(int)
        f = at - i
        score = ((1 - f) * envelope[i] + f * envelope[i + 1]).sum()
        if score > best_score:
            best, best_score = offset, score
    return best


def refine(x, beats, reach=0.06):
    """The median distance from predicted beats to the attacks near them."""
    width = int(0.002 * RATE)
    envelope = np.convolve(np.abs(x), np.ones(width) / width, mode="same")
    rise = np.diff(envelope, prepend=envelope[0])
    offsets = []
    for b in beats:
        lo, hi = int((b - reach) * RATE), int((b + reach) * RATE)
        if lo < 0 or hi >= len(rise):
            continue
        window = rise[lo:hi]
        if window.max() <= 0:
            continue
        offsets.append((lo + int(np.argmax(window))) / RATE - b)
    return float(np.median(offsets)) if offsets else 0.0


def peak(path):
    """Seconds from the start of a sound to its loudest moment."""
    x, rate = read(path, mono=True)
    x = np.abs(x[:, 0])
    width = max(1, int(0.002 * rate))
    smooth = np.convolve(x, np.ones(width) / width, mode="same")
    return float(np.argmax(smooth) / rate)


def analyze(song, beats_per_bar=4):
    x, _ = read(song, rate=RATE, mono=True)
    x = x[:, 0]
    full, low = onsets(x)
    period = tempo(full)
    offset = phase(full, period)
    hop_seconds = HOP / RATE
    beat_seconds = period * hop_seconds
    # Frames are coarse (a window is ~93 ms), so put the grid where the
    # attacks actually are: the steepest rise of a millisecond envelope near
    # each predicted beat, the median over the whole song.
    shift = FFT / 2 / RATE
    beats = np.arange(offset, len(full), period) * hop_seconds + shift
    beats = beats + refine(x, beats)
    # The downbeat: the position in the bar with the most low-end energy.
    strength = []
    for k in range(beats_per_bar):
        idx = ((beats[k::beats_per_bar] - shift) / hop_seconds).astype(int)
        idx = idx[idx < len(low)]
        strength.append(low[idx].mean() if len(idx) else -np.inf)
    first = int(np.argmax(strength))
    # Skip any silence before the music starts.
    loud = np.flatnonzero(np.abs(x) > 0.02 * np.abs(x).max())
    start = loud[0] / RATE if len(loud) else 0.0
    downbeats = beats[first::beats_per_bar]
    downbeats = downbeats[downbeats >= start - beat_seconds * 0.25]
    return {
        "bpm": round(60.0 / beat_seconds, 3),
        "first_downbeat": round(float(downbeats[0]), 4),
        "beat_seconds": round(beat_seconds, 5),
        "beats": [round(float(b), 4) for b in beats if b >= 0],
    }


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("song")
    parser.add_argument("--sounds")
    parser.add_argument("--out", default="beats.json")
    args = parser.parse_args()
    result = analyze(args.song)
    if args.sounds:
        result["peaks"] = {
            os.path.splitext(os.path.basename(p))[0]: round(peak(p), 5)
            for p in sorted(glob.glob(os.path.join(args.sounds, "*.wav")))
        }
    with open(args.out, "w") as f:
        json.dump(result, f, indent=2)
    print(
        f"{result['bpm']} BPM, first downbeat at {result['first_downbeat']}s"
        + (f", {len(result.get('peaks', {}))} sound peaks" if args.sounds else "")
    )


if __name__ == "__main__":
    main()
