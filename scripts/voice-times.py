"""When each word of the website's voices is said, so the show can follow them.

Recognizes site/audio/<id>.mp3 with a local Parakeet model, lines the result up with the words in
scripts/voice-lines.json, and writes site/audio/scenes.json. Needs ffmpeg, numpy and onnxruntime.

    python scripts/voice-times.py <folder that holds the models' folders>
"""
import json
import pathlib
import re
import subprocess
import sys

import numpy as np
import onnxruntime as ort

ROOT = pathlib.Path(__file__).resolve().parent.parent
FRAME = 0.08  # seconds per encoder frame
STATE = 640
sys.stdout.reconfigure(encoding="utf-8")


def session(path):
    return ort.InferenceSession(str(path), providers=["CPUExecutionProvider"])


def recognize(folder, samples):
    """The recognized words, each with the second it starts at."""
    encoder = folder / "encoder-model.int8.onnx"
    if not encoder.exists():
        encoder = folder / "encoder.int8.onnx"
    features, lengths = session(folder / "nemo128.onnx").run(
        None, {"waveforms": samples[None], "waveforms_lens": np.array([len(samples)], dtype=np.int64)}
    )
    encoded, encoded_lengths = session(encoder).run(None, {"audio_signal": features, "length": lengths})
    decoder = session(folder / "decoder_joint-model.int8.onnx")
    vocab = {}
    for line in open(folder / "vocab.txt", encoding="utf8"):
        piece, index = line.rstrip("\n").rsplit(" ", 1)
        vocab[int(index)] = piece.replace("▁", " ")
    blank = len(vocab) - 1
    state1 = np.zeros((2, 1, STATE), dtype=np.float32)
    state2 = np.zeros((2, 1, STATE), dtype=np.float32)
    words, last, t, emitted = [], blank, 0, 0
    while t < encoded_lengths[0]:
        out = decoder.run(None, {
            "encoder_outputs": encoded[:, :, t:t + 1],
            "targets": np.array([[last]], dtype=np.int32),
            "target_length": np.array([1], dtype=np.int32),
            "input_states_1": state1,
            "input_states_2": state2,
        })
        logits = out[0].squeeze()
        token = int(logits[:len(vocab)].argmax())
        skip = int(logits[len(vocab):].argmax()) if logits.shape[0] > len(vocab) else 0
        if token != blank:
            state1, state2, last = out[2], out[3], token
            emitted += 1
            piece = vocab[token]
            if piece.startswith(" ") or not words:
                words.append([piece.strip(), t * FRAME])
            else:
                words[-1][0] += piece
        if skip > 0:
            t += skip
            emitted = 0
        elif token == blank or emitted == 10:
            t += 1
            emitted = 0
    return words


def plain(word):
    return re.sub(r"[^\w]", "", word.lower())


def align(script, heard, duration):
    """Start times for the script's words: from the recognized words that match, between them evenly."""
    n, m = len(script), len(heard)
    table = np.zeros((n + 1, m + 1), dtype=int)
    for i in range(n - 1, -1, -1):
        for j in range(m - 1, -1, -1):
            same = plain(script[i]) == plain(heard[j][0])
            table[i, j] = table[i + 1, j + 1] + 1 if same else max(table[i + 1, j], table[i, j + 1])
    times = [None] * n
    i = j = 0
    while i < n and j < m:
        if plain(script[i]) == plain(heard[j][0]):
            times[i] = heard[j][1]
            i += 1
            j += 1
        elif table[i + 1, j] >= table[i, j + 1]:
            i += 1
        else:
            j += 1
    known = [(k, v) for k, v in enumerate(times) if v is not None]
    anchors = [(-1, 0.0)] + known + [(n, duration)]
    for (a, ta), (b, tb) in zip(anchors, anchors[1:]):
        for k in range(a + 1, b):
            times[k] = ta + (tb - ta) * (k - a) / (b - a)
    return times, len(known)


def main():
    models = pathlib.Path(sys.argv[1])
    scenes = {}
    for line in json.load(open(ROOT / "scripts" / "voice-lines.json", encoding="utf8")):
        path = ROOT / "site" / "audio" / f"{line['id']}.mp3"
        raw = subprocess.run(
            ["ffmpeg", "-v", "error", "-i", str(path), "-ac", "1", "-ar", "16000", "-f", "f32le", "-"],
            check=True, capture_output=True,
        ).stdout
        samples = np.frombuffer(raw, dtype=np.float32)
        duration = len(samples) / 16000
        heard = recognize(models / line["model"], samples)
        script = line["say"].split()
        times, matched = align(script, heard, duration)
        scenes[line["id"]] = {
            "duration": round(duration, 2),
            "words": [{"w": w, "t": round(t, 2)} for w, t in zip(script, times)],
        }
        print(f"{line['id']}: {duration:.1f} s, {matched}/{len(script)} words matched")
        print("   heard:", " ".join(w for w, _ in heard))
    with open(ROOT / "site" / "audio" / "scenes.json", "w", encoding="utf8") as out:
        json.dump(scenes, out, ensure_ascii=False, separators=(",", ":"))


main()
