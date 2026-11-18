#!/home/venkatengineer/.local/share/solaris-voice-env/bin/python3
"""
================================================================================
🛸 TRUNI SPATIAL AI // CONTINUOUS VOICE WAKE DAEMON
Listens continuously for:
 - "TRUNI"
 - "Hey TRUNI"
 - "Hey TRUNI can you help me"
 - "TRUNI can you help me"
 - "Can you help me"
Brings TRUNI online, displays Solaris HUD overlay, and activates voice / agent.
================================================================================
"""

import os
import sys
import json
import time
import queue
import math
import struct
import subprocess
import signal
import sounddevice as sd
import vosk

MODEL_PATH = "/home/venkatengineer/.local/share/vosk-models/vosk-model-small-en-us-0.15"
LOG_FILE = "/tmp/truni-wake-daemon.log"

SAMPLE_RATE = 16000
BLOCK_SIZE = 4000  # 250ms per chunk
MIN_RMS_THRESHOLD = 140  # Ambient noise filter

WAKE_PATTERNS = [
    "hey truni can you help me",
    "truni can you help me",
    "can you help me",
    "hey truni",
    "truni",
    "true knee",
    "to knee",
    "true me",
    "turn me",
    "solaris truni",
    "hey solaris",
    "solaris",
    "open truni",
    "activate truni",
]

def log(msg: str):
    ts = time.strftime("%Y-%m-%d %H:%M:%S")
    line = f"[{ts}] {msg}"
    print(line, flush=True)
    try:
        with open(LOG_FILE, "a") as f:
            f.write(line + "\n")
    except Exception:
        pass

def optimize_audio():
    """Ensure PipeWire mic and sink are properly unmuted and set."""
    try:
        subprocess.run(["wpctl", "set-mute", "@DEFAULT_AUDIO_SOURCE@", "0"], check=False)
        subprocess.run(["wpctl", "set-volume", "@DEFAULT_AUDIO_SOURCE@", "1.0"], check=False)
        subprocess.run(["wpctl", "set-mute", "@DEFAULT_AUDIO_SINK@", "0"], check=False)
    except Exception as e:
        log(f"Audio device configuration warning: {e}")

def is_overlay_running() -> bool:
    try:
        out = subprocess.check_output(["pgrep", "-f", "truni overlay"]).decode()
        return bool(out.strip())
    except Exception:
        return False

def trigger_wake_event(recognized_text: str):
    log(f"🛸 Wake event matched: '{recognized_text}'")

    # Check if a command follows the wake word
    cleaned = recognized_text.lower()
    for w in ["hey truni can you help me", "truni can you help me", "hey truni", "truni", "can you help me", "true knee", "solaris"]:
        if w in cleaned:
            cleaned = cleaned.replace(w, "").strip()
            break

    # Speak acknowledgement
    subprocess.Popen([
        "/home/venkatengineer/.local/bin/truni", "ask", "acknowledge wake"
    ] if cleaned else [])

    # If overlay is not running, start it
    if not is_overlay_running():
        log("Launching TRUNI Solaris Overlay...")
        subprocess.Popen(["/home/venkatengineer/.local/bin/truni", "overlay"],
                         stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        time.sleep(1.0)

    # If the user included an instruction directly in the utterance
    if cleaned and len(cleaned) > 3:
        log(f"Dispatching inline instruction to TRUNI: '{cleaned}'")
        subprocess.Popen(["/home/venkatengineer/.local/bin/truni", "ask", cleaned])
    else:
        # Otherwise start voice listening on the deck
        log("Activating voice listening on flight deck...")
        # Send IPC command to trigger voice listen or invoke truni voice in background
        subprocess.Popen(["/home/venkatengineer/.local/bin/truni", "voice"])

def main():
    log("🛸 TRUNI Voice Wake Daemon starting up...")
    optimize_audio()

    if not os.path.exists(MODEL_PATH):
        log(f"ERROR: Vosk model not found at {MODEL_PATH}")
        sys.exit(1)

    vosk.SetLogLevel(-1)
    log(f"Loading acoustic model from {MODEL_PATH}...")
    model = vosk.Model(MODEL_PATH)
    rec = vosk.KaldiRecognizer(model, SAMPLE_RATE)
    log("Acoustic model loaded. Standby for voice invocation ('TRUNI' / 'Hey TRUNI can you help me')...")

    audio_q = queue.Queue()

    def audio_callback(indata, frames, time_info, status):
        if status:
            pass
        audio_q.put(bytes(indata))

    # Signal handlers
    def sig_handler(sig, frame):
        log("Received termination signal. Standing down...")
        sys.exit(0)

    signal.signal(signal.SIGINT, sig_handler)
    signal.signal(signal.SIGTERM, sig_handler)

    cooldown_until = 0.0

    with sd.RawInputStream(samplerate=SAMPLE_RATE, blocksize=BLOCK_SIZE, dtype='int16', channels=1, callback=audio_callback):
        while True:
            data = audio_q.get()
            now = time.time()
            if now < cooldown_until:
                # Discard audio during cooldown (prevent self-trigger from TRUNI's own TTS output)
                continue

            # Calculate RMS energy to reject low-level hiss
            count = len(data) // 2
            if count == 0:
                continue
            shorts = struct.unpack(f"{count}h", data)
            sum_squares = sum(s * s for s in shorts)
            rms = math.sqrt(sum_squares / count)

            if rms < MIN_RMS_THRESHOLD:
                continue

            if rec.AcceptWaveform(data):
                res = json.loads(rec.Result())
                text = res.get("text", "").lower().strip()
                if text:
                    log(f"Heard: '{text}' (RMS: {int(rms)})")
                    matched = False
                    for pattern in WAKE_PATTERNS:
                        if pattern in text:
                            matched = True
                            break
                    if matched:
                        trigger_wake_event(text)
                        rec.Reset()
                        cooldown_until = time.time() + 4.0  # 4s cooldown while system speaks
            else:
                partial = json.loads(rec.PartialResult()).get("partial", "").lower().strip()
                if partial:
                    for pattern in WAKE_PATTERNS:
                        if pattern in partial:
                            log(f"Heard partial match: '{partial}'")
                            trigger_wake_event(partial)
                            rec.Reset()
                            cooldown_until = time.time() + 4.0
                            break

if __name__ == "__main__":
    main()
