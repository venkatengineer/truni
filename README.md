# 🛸 TRUNI: Spatial Spaceship AI & Autonomous Intelligence System for Linux

TRUNI is an autonomous onboard computer and personal AI agent for Linux, engineered specifically for the **Niri Wayland** desktop environment. It transforms your machine into an intelligent spatial spacecraft cockpit with real-time 3D telemetry, voice interaction, local LLM execution, filesystem access, internet research, and strict security controls.

---

## 🚀 Key Architectural Pillars

1. **Native Wayland Overlay (`truni-overlay`)**:
   - Built in pure Rust using **GTK4** and **gtk4-layer-shell**.
   - Integrates directly into Niri's `Overlay` layer (`zwlr_layer_shell_v1`).
   - Floating, non-intrusive, dismissible with `Escape` without killing the backend.

2. **Five Operational Interface States**:
   - **STATE A (STANDBY)**: Minimalist aerospace status pill with live telemetry badge and privacy indicator.
   - **STATE B (ACTIVATION)**: Cinematic initialization sequence and subsystem diagnostics.
   - **STATE C (CONVERSATION)**: Intelligence visual, conversation history, text entry, and quick telemetry chips.
   - **STATE D (TASK EXECUTION)**: Step-by-step mission planner with live progress, tool logs, and operator approval gate.
   - **STATE E (SPACESHIP INSPECTION)**: Real-time 3D interactive model of the laptop rendered via hardware-accelerated OpenGL (`glow`), with clickable components and zero-hallucination telemetry.

3. **Autonomous Computer-Operating Agent (`truni-agent` & `truni-tools`)**:
   - Multi-step planning: Intent detection → Tool selection → Execution → Verification → Aerospace summary.
   - Controlled Bash execution, recursive file search, document reading, directory organization, duplicate detection.
   - Web intelligence queries via DuckDuckGo extraction.
   - Security permission engine: Explicit operator approval required for file deletion or system-altering commands.
   - Persistent memory backed by SQLite (`truni-memory`).

4. **Zero-Hallucination Hardware Telemetry (`truni-telemetry`)**:
   - Direct kernel and hardware sensor reading (`sysfs`, `/sys/class/hwmon`, `dell_smm`, `alienware_wmi`).
   - Actual CPU package temp (e.g. 78°C–91°C), dual cooling turbine RPMs (e.g. 3700–6850 RPM).
   - SPD RAM temperatures (`spd5118`), NVMe drive temperatures, battery potential and charging state.
   - Automatic peripheral discovery: USB peripherals, Bluetooth devices, audio sinks, and Wayland displays.

5. **Sub-Second Voice Pipeline (`truni-voice`)**:
   - **Wake Word**: "TRUNI" standby listener.
   - **Speech Recognition**: Local Whisper.cpp (`ggml-tiny.en.bin`) transcribing in <400ms.
   - **Speech Synthesis**: Local neural Piper TTS (`en_US-lessac-medium.onnx`) with instant voice cancellation on user interruption.

---

## 🛠️ CLI & Runtime Usage

### 1. View Live System Telemetry
```bash
./target/debug/truni telemetry
```

### 2. Verify Subsystem Health & Online Status
```bash
./target/debug/truni status
```

### 3. Ask Questions or Assign Tasks via CLI
```bash
./target/debug/truni ask "what is consuming my RAM?"
./target/debug/truni ask "why is my laptop getting hot?"
./target/debug/truni ask "show me all connected devices"
./target/debug/truni ask "research the latest developments in quantum computing"
./target/debug/truni ask "find my college networking notes"
```

### 4. Launch the Interactive GUI Layer Overlay
```bash
./target/debug/truni overlay
```

### 5. Launch Primary Background Daemon
```bash
./target/debug/truni daemon
```

---

## ⌨️ Overlay Keybindings

| Key | Action |
|---|---|
| `F1` | Switch to **Standby Mode** |
| `F2` | Switch to **Conversation / Comm Deck** |
| `F3` | Switch to **Task Execution Matrix** |
| `F4` | Switch to **3D Spaceship Inspection** |
| `Space` | Cycle through 3D Hardware Modules |
| `Mouse Drag` | Orbit / Rotate 3D Spacecraft Viewport |
| `Scroll Wheel` | Zoom In / Out |
| `Escape` | Toggle / Dismiss Floating Overlay |

---

## 🛸 Niri Compositor Integration (Optional)

To bind TRUNI directly to your Niri keybindings in `~/.config/niri/config.kdl`, add:

```kdl
binds {
    Mod+Space hotkey-overlay-title="TRUNI SPACESHIP AI" { spawn "/mnt/d/truni/target/debug/truni" "toggle"; }
    Mod+Alt+V hotkey-overlay-title="TRUNI VOICE COMMAND" { spawn "/mnt/d/truni/target/debug/truni" "voice"; }
    Mod+Alt+S hotkey-overlay-title="TRUNI 3D SPACESHIP" { spawn "/mnt/d/truni/target/debug/truni" "inspect"; }
}
```
