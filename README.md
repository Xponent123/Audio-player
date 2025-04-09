# Rust Audio Player

A simple desktop audio player built in Rust with a basic GUI.

## Features

- **Local Playback**  
  Open single files or entire folders of audio.
- **YouTube Playback**  
  Paste a YouTube URL to stream audio directly.
- **Collections & Queue**  
  Build a library of tracks, search, and add to your play queue.
- **Playback Controls**  
  Previous / Play‑Pause / Next, volume slider, shuffle mode.
- **Modular GUI**  
  Tabs for Player and Equalizer (equalizer under development).
- **Extensible**  
  Code organized into `main.rs`, `theme.rs`, `visualizer.rs`, and `widgets.rs`.

---

## 📦 Dependencies

- **Rust** (1.65+)
- [Iced](https://github.com/iced-rs/iced) (GUI framework)
- [rodio](https://github.com/RustAudio/rodio) (audio playback)
- [youtube-dl](https://github.com/ytdl-org/youtube-dl) or [yt-dlp](https://github.com/yt-dlp/yt-dlp) (for YouTube audio extraction)
- [rfd](https://github.com/PolyMeilex/rfd) (file/folder dialogs)

Add these to your `Cargo.toml` under `[dependencies]`.

---


## Setup & Run

```bash
git clone https://github.com/your-username/rust-audio-player.git
cd rust-audio-player

# build
cargo build --release

# run
cargo run --release
