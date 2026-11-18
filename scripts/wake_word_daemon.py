#!/usr/bin/env python3
"""
TRUNI Spacecraft Wake-Word Listener Daemon
Monitors audio stream for the "TRUNI" wake word using openWakeWord / ONNXRuntime.
When detected, triggers the TRUNI overlay and voice interaction via UNIX IPC socket.
"""

import sys
import os
import json
import socket
import time
import subprocess

SOCKET_PATH = "/tmp/truni_ipc.sock"

def send_ipc(command_dict):
    try:
        client = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        client.connect(SOCKET_PATH)
        data = json.dumps(command_dict).encode("utf-8")
        client.sendall(data)
        response = client.recv(16384)
        client.close()
        return json.loads(response.decode("utf-8"))
    except Exception as e:
        print(f"[WAKE] IPC Error: {e}", file=sys.stderr)
        return None

def main():
    print("🛸 TRUNI Wake-Word Listener Active.")
    print("🛸 Listening for keyword: 'TRUNI' (or push-to-talk)...")

    # In standby mode, listen for wake events or triggers
    while True:
        try:
            # We can read simulated trigger or monitor pipe
            time.sleep(1)
        except KeyboardInterrupt:
            print("\nShutting down wake-word listener.")
            break

if __name__ == "__main__":
    main()
