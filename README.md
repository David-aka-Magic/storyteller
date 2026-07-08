# StoryEngine

**A locally-run, AI-powered interactive fiction engine with contextually consistent character art.**

StoryEngine is a desktop application that combines AI-driven narrative generation with AI image generation to create an immersive, offline interactive fiction experience. Set a premise, define your characters, and generate story turns collaboratively — with each scene illustrated by an image generator that keeps every character visually consistent across the entire story.

Everything runs locally. No cloud APIs, no subscriptions, no accounts — just your GPU.

---

## Features

- **Interactive turn-based storytelling** — powered by a local LLM via Ollama, running a custom fine-tuned model built for narrative consistency and dialogue formatting.
- **POV character mode** — play as one of your characters, or sit back and watch the story unfold in passive/observer mode.
- **Persistent character profiles** — appearance sliders (height, build), personality and backstory notes, default clothing, and a master reference image used to keep each character's face and look consistent in every generated image.
- **Scene-based world model** — scenes represent *places*, not moments. The same location persists across time-of-day changes and mood shifts, rather than being regenerated from scratch every turn.
- **AI-generated scene art** — each story turn can produce a matching image using ComfyUI, with IP-Adapter FaceID and ControlNet pose guidance so characters stay recognizable turn after turn.
- **Fully offline** — all inference (text and image) runs on your own hardware.
- **Local persistence** — stories, characters, and scenes are saved to a local SQLite database.

---

## Tech Stack

| Layer | Technology |
|---|---|
| Frontend | SvelteKit, Svelte 5 (runes) |
| Backend | Rust, Tauri v2 |
| Database | SQLite (via `sqlx`) |
| LLM inference | Ollama, running a custom Qwen3-based model |
| Image generation | ComfyUI — JuggernautXL, IP-Adapter FaceID Plus v2, ControlNet OpenPose, InsightFace (`buffalo_l`) |

---

## Requirements

- **Rust** toolchain (stable) + Cargo
- **Node.js** and npm
- **Ollama**, with the project's custom model built from the included Modelfile
- **ComfyUI**, with the following installed:
  - JuggernautXL checkpoint
  - IP-Adapter FaceID Plus v2
  - ControlNet OpenPose
  - InsightFace (`buffalo_l`)
- **GPU with 12GB+ VRAM recommended.** Ollama and ComfyUI are not designed to be resident in VRAM simultaneously on smaller cards — StoryEngine handles swapping between them automatically, but more VRAM means less swapping and faster turns.
- Currently developed and tested on **Windows**.

---

## Getting Started

```bash
# Install frontend dependencies
npm install

# Build the custom Ollama model from the included Modelfile
ollama create Story_v27 -f StoryEngine-modelfile

# Run the app in development mode
npm run tauri dev
```

Make sure Ollama and ComfyUI are both installed and reachable before starting the app, and close any external SQLite database viewers — an open lock on the database can stall the generation pipeline.

---

## Project Structure

```
├── src/                # SvelteKit frontend (UI, components, stores)
├── src-tauri/           # Rust backend
│   ├── src/
│   │   ├── text_gen/    # LLM prompt building, parsing, and orchestration
│   │   └── ...          # Image generation, database, and command handlers
│   └── Cargo.toml
├── StoryEngine-modelfile # Ollama Modelfile for the custom narrative model
└── README.md
```

---

## Status

StoryEngine is an actively developed, solo-built personal project (pre-1.0). It's designed for local, single-user use rather than production deployment. Expect rough edges — features and internals are still evolving.

---

## License

*No license has been specified yet. All rights reserved by the author unless stated otherwise.*
