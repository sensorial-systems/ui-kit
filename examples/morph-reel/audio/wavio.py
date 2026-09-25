"""Reading and writing audio as float arrays, with ffmpeg for anything that is
not already a plain WAV."""
import os
import subprocess
import tempfile
import wave

import numpy as np


def read(path, rate=None, mono=False):
    """`path` as (samples, rate): float32 in -1..1, shaped (frames, channels).

    Anything other than 16-bit PCM WAV at the wanted rate (an MP3 from a
    royalty-free library, say) is converted with ffmpeg first."""
    try:
        with wave.open(path, "rb") as w:
            usable = w.getsampwidth() == 2 and (rate is None or w.getframerate() == rate)
    except (wave.Error, EOFError):
        usable = False
    if not usable:
        handle, converted = tempfile.mkstemp(suffix=".wav")
        os.close(handle)
        command = ["ffmpeg", "-y", "-loglevel", "error", "-i", path, "-acodec", "pcm_s16le"]
        if rate:
            command += ["-ar", str(rate)]
        subprocess.run(command + [converted], check=True)
        try:
            return read(converted, rate, mono)
        finally:
            os.remove(converted)
    with wave.open(path, "rb") as w:
        channels = w.getnchannels()
        frames = np.frombuffer(w.readframes(w.getnframes()), dtype="<i2")
        samples = frames.reshape(-1, channels).astype(np.float32) / 32768.0
        rate = w.getframerate()
    if mono:
        samples = samples.mean(axis=1, keepdims=True)
    return samples, rate


def write(path, samples, rate):
    """Write float samples, (frames,) or (frames, channels), as 16-bit WAV."""
    if samples.ndim == 1:
        samples = samples[:, None]
    data = (np.clip(samples, -1.0, 1.0) * 32767.0).astype("<i2")
    with wave.open(path, "wb") as w:
        w.setnchannels(data.shape[1])
        w.setsampwidth(2)
        w.setframerate(rate)
        w.writeframes(data.tobytes())
