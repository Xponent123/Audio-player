use std::fs;
use std::io::BufReader;
use std::path::PathBuf;
use std::time::Duration;
use std::process::Command;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use std::sync::{Arc, Mutex}; // Add these imports for thread-safe shared state

use eframe::egui;
use egui::RichText;
use egui::ViewportBuilder;
use rand::seq::SliceRandom;
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};
use rfd::FileDialog;

use rdev::{listen, Event, EventType, Key};

// Add this to your Cargo.toml:
// biquad = "0.3"
use biquad::{Biquad, Coefficients, DirectForm1, Hertz}; // Add Hertz here

mod theme;
mod visualizer;
mod widgets;


/// A helper function to remove extra tags or info from a raw title.
fn clean_title(raw_title: &str) -> String {
    let cleaned = raw_title
        .replace("[Official Music Video]", "")
        .replace("(Official Music Video)", "")
        .replace("Official Music Video", "")
        .replace("[Official Video]", "")
        .replace("(Official Video)", "")
        .replace("Official Video", "")
        .replace("[Lyrics]", "")
        .replace("(Lyrics)", "")
        .replace("Lyrics", "")
        .trim()
        .to_string();
    cleaned.split_whitespace().take(6).collect::<Vec<_>>().join(" ")
}

/// Commands sent by the global key listener.
enum KeyCommand {
    IncreaseVolume,
    TogglePause,
    DecreaseVolume,
}

/// Struct to represent a media item.
#[derive(Clone)]
struct MediaItem {
    file_path: PathBuf,
    display_name: String,
    artist: Option<String>,
}

/// Enum to represent the active UI tab.
#[derive(PartialEq)]
enum AppTab {
    Player,
    Equalizer,
}

/// Enum for Equalizer presets.
#[derive(Debug, Clone, PartialEq)]
enum EqualizerPreset {
    Flat,
    Classical,
    HipHop,
    Pop,
    Rock,
    HeavyMetal,
    Folk,
    Custom,
}

/// Struct to hold equalizer settings (assumes a 10-band equalizer).
#[derive(Clone)]
struct EqualizerSettings {
    preset: EqualizerPreset,
    bands: Vec<f32>, // gain in dB for each band
}

impl EqualizerSettings {
    fn new() -> Self {
        Self {
            preset: EqualizerPreset::Flat,
            bands: vec![0.0; 10],
        }
    }

    /// Apply predefined gain values for each preset.
    fn apply_preset(&mut self) {
        match self.preset {
            EqualizerPreset::Flat => self.bands = vec![0.0; 10],
            EqualizerPreset::Classical => {
                self.bands = vec![-2.0, -1.0, 0.0, 1.0, 2.0, 2.0, 1.0, 0.0, -1.0, -2.0]
            }
            EqualizerPreset::HipHop => {
                self.bands = vec![3.0, 2.0, 0.0, -1.0, -2.0, -2.0, -1.0, 0.0, 2.0, 3.0]
            }
            EqualizerPreset::Pop => {
                self.bands = vec![1.0, 1.5, 2.0, 2.5, 3.0, 3.0, 2.5, 2.0, 1.5, 1.0]
            }
            EqualizerPreset::Rock => {
                self.bands = vec![2.0, 1.5, 1.0, 0.0, -1.0, -1.0, 0.0, 1.0, 1.5, 2.0]
            }
            EqualizerPreset::HeavyMetal => {
                self.bands = vec![4.0, 3.0, 2.0, 1.0, 0.0, 0.0, 1.0, 2.0, 3.0, 4.0]
            }
            EqualizerPreset::Folk => {
                self.bands = vec![0.0, 0.5, 1.0, 1.5, 2.0, 2.0, 1.5, 1.0, 0.5, 0.0]
            }
            EqualizerPreset::Custom => {
                // Leave bands unchanged.
            }
        }
    }
}

/// DSP chain using a series of biquad peak filters.
struct EqualizerDSP {
    filters: Vec<DirectForm1<f32>>,
}

impl EqualizerDSP {
    /// Create a new DSP chain based on the equalizer settings.
    fn new(equalizer_settings: &EqualizerSettings, sample_rate: f32) -> Self {
        // Typical 10-band equalizer center frequencies in Hz.
        let center_frequencies = vec![
            31.25, 62.5, 125.0, 250.0, 500.0,
            1000.0, 2000.0, 4000.0, 8000.0, 16000.0,
        ];
        let mut filters = Vec::new();
        for (i, &gain_db) in equalizer_settings.bands.iter().enumerate() {
            // Create a peaking EQ filter.
            // The biquad::Type::PeakingEQ takes the gain value as a parameter.
            let coef = Coefficients::<f32>::from_params(
                biquad::Type::PeakingEQ(gain_db),
                Hertz::<f32>::from_hz(sample_rate).unwrap(),          // Use from_hz instead of new
                Hertz::<f32>::from_hz(center_frequencies[i]).unwrap(), // Use from_hz instead of new
                1.0, // Q factor (adjust as needed)
            ).unwrap();
            // Specify the type to be f32 explicitly.
            let filter = DirectForm1::<f32>::new(coef);
            filters.push(filter);
        }
        Self { filters }
    }

    /// Process a single sample through the filter chain.
    fn process_sample(&mut self, sample: f32) -> f32 {
        self.filters.iter_mut().fold(sample, |s, filter| filter.run(s))
    }
}

/// Custom rodio source that processes samples with the equalizer DSP chain.
struct EqualizedSource<S>
where
    S: Source<Item = f32>,
{
    inner: S,
    dsp: EqualizerDSP,
    // Add shared equalizer settings reference
    equalizer_settings: Arc<Mutex<EqualizerSettings>>,
    sample_rate: f32,
    // Track when settings have changed to rebuild the DSP chain
    last_update: usize,
}

impl<S> Iterator for EqualizedSource<S>
where
    S: Source<Item = f32>,
{
    type Item = f32;
    fn next(&mut self) -> Option<Self::Item> {
        // Check if equalizer settings have changed
        let current_update = {
            let settings = self.equalizer_settings.lock().unwrap();
            // Just accessing the lock will tell us if settings changed
            settings.bands.len() // Using length as a simple hash
        };
        
        // If settings changed, rebuild the DSP chain
        if current_update != self.last_update {
            let settings = self.equalizer_settings.lock().unwrap().clone();
            self.dsp = EqualizerDSP::new(&settings, self.sample_rate);
            self.last_update = current_update;
        }
        
        self.inner.next().map(|sample| self.dsp.process_sample(sample))
    }
}

impl<S> Source for EqualizedSource<S>
where
    S: Source<Item = f32>,
{
    fn current_frame_len(&self) -> Option<usize> {
        self.inner.current_frame_len()
    }
    fn channels(&self) -> u16 {
        self.inner.channels()
    }
    fn sample_rate(&self) -> u32 {
        self.inner.sample_rate()
    }
    fn total_duration(&self) -> Option<Duration> {
        self.inner.total_duration()
    }
}

/// Main application struct.
struct AudioPlayerApp {
    queue: Vec<MediaItem>,
    current_index: Option<usize>,
    stream: Option<OutputStream>,
    stream_handle: Option<OutputStreamHandle>,
    sink: Option<Sink>,
    is_paused: bool,
    volume: f32,
    shuffle: bool,
    youtube_url: String,
    download_status: String,
    youtube_sender: Option<Sender<(MediaItem, String)>>,
    youtube_receiver: Option<Receiver<(MediaItem, String)>>,
    key_receiver: Receiver<KeyCommand>,
    collections_path: PathBuf,
    show_collections: bool,
    collections_search: String,
    show_youtube_input: bool,
    youtube_search_url: String,
    current_position: f32,
    total_duration: f32, // dummy value for demonstration
    current_tab: AppTab,
    equalizer: EqualizerSettings,
    // Add shared state for real-time adjustments
    shared_equalizer: Arc<Mutex<EqualizerSettings>>,
}

impl AudioPlayerApp {
    fn new() -> Self {
        let (stream, stream_handle) = OutputStream::try_default().unwrap();
        let (yt_tx, yt_rx) = channel::<(MediaItem, String)>();
        let (key_tx, key_rx) = channel::<KeyCommand>();

        // Global key listener thread.
        thread::spawn(move || {
            let mut ctrl_pressed = false;
            if let Err(e) = listen(move |event: Event| {
                match event.event_type {
                    EventType::KeyPress(key) => {
                        if key == Key::ControlLeft || key == Key::ControlRight {
                            ctrl_pressed = true;
                        }
                        if key == Key::KeyU && ctrl_pressed {
                            let _ = key_tx.send(KeyCommand::IncreaseVolume);
                        }
                        if key == Key::KeyD && ctrl_pressed {
                            let _ = key_tx.send(KeyCommand::DecreaseVolume);
                        }
                        if key == Key::KeyP && ctrl_pressed {
                            let _ = key_tx.send(KeyCommand::TogglePause);
                        }
                    }
                    EventType::KeyRelease(key) => {
                        if key == Key::ControlLeft || key == Key::ControlRight {
                            ctrl_pressed = false;
                        }
                    }
                    _ => {}
                }
            }) {
                eprintln!("Global key listener error: {:?}", e);
            }
        });

        let collections_path = std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("my_collections");
        fs::create_dir_all(&collections_path).unwrap();

        let equalizer = EqualizerSettings::new();
        let shared_equalizer = Arc::new(Mutex::new(equalizer.clone()));
        
        Self {
            queue: Vec::new(),
            current_index: None,
            stream: Some(stream),
            stream_handle: Some(stream_handle),
            sink: None,
            is_paused: false,
            volume: 0.5,
            shuffle: false,
            youtube_url: String::new(),
            download_status: String::new(),
            youtube_sender: Some(yt_tx),
            youtube_receiver: Some(yt_rx),
            key_receiver: key_rx,
            collections_path,
            show_collections: true,
            collections_search: String::new(),
            show_youtube_input: false,
            youtube_search_url: String::new(),
            current_position: 0.0,
            total_duration: 240.0, // Dummy 4-minute duration.
            current_tab: AppTab::Player,
            equalizer,
            shared_equalizer,
        }
    }

    /// Load and play the current track.
    /// Wrap the decoded audio with EqualizedSource to process samples.
    fn play_current(&mut self) {
        if let Some(idx) = self.current_index {
            if idx < self.queue.len() {
                if let Some(sink) = self.sink.take() {
                    sink.stop();
                }
                let item = &self.queue[idx];
                if let Ok(file) = fs::File::open(&item.file_path) {
                    if let Ok(decoder) = Decoder::new(BufReader::new(file)) {
                        if let Some(ref handle) = self.stream_handle {
                            self.current_position = 0.0;
                            let sample_rate = decoder.sample_rate() as f32;
                            
                            // Update shared settings before creating the source
                            {
                                let mut shared = self.shared_equalizer.lock().unwrap();
                                *shared = self.equalizer.clone();
                            }
                            
                            let equalized_source = EqualizedSource {
                                inner: decoder.convert_samples(),
                                dsp: EqualizerDSP::new(&self.equalizer, sample_rate),
                                equalizer_settings: self.shared_equalizer.clone(),
                                sample_rate,
                                last_update: self.equalizer.bands.len(),
                            };
                            
                            let sink = Sink::try_new(handle).unwrap();
                            sink.append(equalized_source);
                            sink.set_volume(self.volume);
                            self.sink = Some(sink);
                            self.is_paused = false;
                        }
                    }
                }
            }
        }
    }

    fn next_track(&mut self) {
        if self.queue.is_empty() {
            return;
        }
        if self.shuffle {
            let mut indices: Vec<usize> = (0..self.queue.len()).collect();
            if let Some(current) = self.current_index {
                indices.retain(|&i| i != current);
            }
            if let Some(&next) = indices.choose(&mut rand::thread_rng()) {
                self.current_index = Some(next);
            }
        } else {
            self.current_index = Some(match self.current_index {
                Some(i) if i + 1 < self.queue.len() => i + 1,
                _ => 0,
            });
        }
        self.play_current();
    }

    fn prev_track(&mut self) {
        if self.queue.is_empty() {
            return;
        }
        if self.shuffle {
            let mut indices: Vec<usize> = (0..self.queue.len()).collect();
            if let Some(current) = self.current_index {
                indices.retain(|&i| i != current);
            }
            if let Some(&prev) = indices.choose(&mut rand::thread_rng()) {
                self.current_index = Some(prev);
            }
        } else {
            self.current_index = Some(match self.current_index {
                Some(i) if i > 0 => i - 1,
                _ => self.queue.len() - 1,
            });
        }
        self.play_current();
    }

    fn pause(&mut self) {
        if let Some(ref sink) = self.sink {
            sink.pause();
            self.is_paused = true;
        }
    }

    fn resume(&mut self) {
        if let Some(ref sink) = self.sink {
            sink.play();
            self.is_paused = false;
        }
    }

    fn set_volume(&mut self, vol: f32) {
        self.volume = vol;
        if let Some(ref sink) = self.sink {
            sink.set_volume(vol);
        }
    }

    fn add_file(&mut self, item: MediaItem) {
        self.queue.push(item);
        if self.current_index.is_none() {
            self.current_index = Some(0);
            self.play_current();
        }
    }

    fn add_folder(&mut self, folder: PathBuf) {
        if let Ok(entries) = fs::read_dir(folder) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if ["mp3", "wav", "flac", "ogg"].contains(&ext.to_lowercase().as_str()) {
                        let display_name = path
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("Unknown")
                            .to_string();
                        self.queue.push(MediaItem {
                            file_path: path,
                            display_name,
                            artist: None,
                        });
                    }
                }
            }
        }
        if self.current_index.is_none() && !self.queue.is_empty() {
            self.current_index = Some(0);
            self.play_current();
        }
    }

    fn add_youtube_audio(&mut self, url: String) {
        if url.is_empty() {
            self.download_status = "Please enter a valid YouTube URL".to_string();
            return;
        }
        self.download_status = "Downloading...".to_string();
        let output_template = format!("{}/%(title)s.%(ext)s", self.collections_path.display());
        let url_clone = url.clone();
        let tx = self.youtube_sender.clone();
        thread::spawn(move || {
            let cmd_output = Command::new("yt-dlp")
                .args(&[
                    "--print", "after_move:filepath",
                    "--extract-audio",
                    "--audio-format", "mp3",
                    "-o", &output_template,
                    &url_clone,
                ])
                .output();
            if let Ok(cmd_output) = cmd_output {
                if cmd_output.status.success() {
                    let final_path = String::from_utf8_lossy(&cmd_output.stdout)
                        .trim()
                        .to_string();
                    let final_path_buf = PathBuf::from(&final_path);
                    if final_path_buf.exists() {
                        let raw_title = final_path_buf
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("Unknown Title")
                            .to_string();
                        let display_name = clean_title(&raw_title);
                        let item = MediaItem {
                            file_path: final_path_buf,
                            display_name,
                            artist: None,
                        };
                        if let Some(tx) = tx {
                            let _ = tx.send((item, url_clone));
                        }
                    }
                }
            }
        });
    }

    fn process_youtube_result(&mut self) {
        if let Some(ref rx) = self.youtube_receiver {
            let mut new_items = Vec::new();
            while let Ok((item, url)) = rx.try_recv() {
                self.download_status = format!("Added YouTube audio: {}", url);
                new_items.push(item);
            }
            for item in new_items {
                self.add_file(item);
            }
        }
    }

    fn process_key_commands(&mut self) {
        while let Ok(cmd) = self.key_receiver.try_recv() {
            match cmd {
                KeyCommand::IncreaseVolume => {
                    self.volume = (self.volume + 0.05).min(1.0);
                    self.set_volume(self.volume);
                    println!("Volume increased to {:.2}", self.volume);
                }
                KeyCommand::DecreaseVolume => {
                    self.volume = (self.volume - 0.05).max(0.0);
                    self.set_volume(self.volume);
                    println!("Volume decreased to {:.2}", self.volume);
                }
                KeyCommand::TogglePause => {
                    if self.is_paused {
                        self.resume();
                        println!("Playback resumed");
                    } else {
                        self.pause();
                        println!("Playback paused");
                    }
                }
            }
        }
    }

    fn load_collections(&self) -> Vec<MediaItem> {
        let mut items = Vec::new();
        if let Ok(entries) = fs::read_dir(&self.collections_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if ext.to_lowercase() == "mp3" {
                        let raw_title = path
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("Unknown")
                            .to_string();
                        let display_name = clean_title(&raw_title);
                        items.push(MediaItem {
                            file_path: path,
                            display_name,
                            artist: None,
                        });
                    }
                }
            }
        }
        items
    }

    fn check_track_finished(&mut self) {
        if let Some(ref sink) = self.sink {
            if !self.is_paused && sink.empty() {
                self.next_track();
            }
        }
    }

    fn seek_to(&mut self, new_time: f32) {
        if let Some(idx) = self.current_index {
            if idx < self.queue.len() && self.total_duration > 0.0 {
                if let Ok(metadata) = fs::metadata(&self.queue[idx].file_path) {
                    let file_size = metadata.len() as f32;
                    let offset = ((new_time / self.total_duration) * file_size) as u64;
                    if let Ok(buffer) = fs::read(&self.queue[idx].file_path) {
                        use std::io::{Cursor, Seek, SeekFrom};
                        let mut cursor = Cursor::new(buffer);
                        if cursor.seek(SeekFrom::Start(offset)).is_ok() {
                            if let Ok(decoder) = Decoder::new(BufReader::new(cursor)) {
                                if let Some(ref handle) = self.stream_handle {
                                    let sample_rate = decoder.sample_rate() as f32;
                                    let equalized_source = EqualizedSource {
                                        inner: decoder.convert_samples(),
                                        dsp: EqualizerDSP::new(&self.equalizer, sample_rate),
                                        equalizer_settings: self.shared_equalizer.clone(),
                                        sample_rate,
                                        last_update: self.equalizer.bands.len(),
                                    };
                                    let sink = Sink::try_new(handle).unwrap();
                                    sink.append(equalized_source);
                                    sink.set_volume(self.volume);
                                    self.sink = Some(sink);
                                    self.current_position = new_time;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Update the equalizer settings and apply them in real-time
    fn update_equalizer_settings(&mut self) {
        // Update the shared state so audio processing can access the changes
        let mut shared = self.shared_equalizer.lock().unwrap();
        *shared = self.equalizer.clone();
    }

    /// Draw the Equalizer tab UI.
    /// Now updates in real-time without restarting playback.
    fn draw_equalizer_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Audio Equalizer");

        let mut preset_changed = false;
        
        egui::ComboBox::from_label("Preset")
            .selected_text(format!("{:?}", self.equalizer.preset))
            .show_ui(ui, |ui| {
                preset_changed |= ui.selectable_value(&mut self.equalizer.preset, EqualizerPreset::Flat, "Flat").clicked();
                preset_changed |= ui.selectable_value(&mut self.equalizer.preset, EqualizerPreset::Classical, "Classical").clicked();
                preset_changed |= ui.selectable_value(&mut self.equalizer.preset, EqualizerPreset::HipHop, "Hip Hop").clicked();
                preset_changed |= ui.selectable_value(&mut self.equalizer.preset, EqualizerPreset::Pop, "Pop").clicked();
                preset_changed |= ui.selectable_value(&mut self.equalizer.preset, EqualizerPreset::Rock, "Rock").clicked();
                preset_changed |= ui.selectable_value(&mut self.equalizer.preset, EqualizerPreset::HeavyMetal, "Heavy Metal").clicked();
                preset_changed |= ui.selectable_value(&mut self.equalizer.preset, EqualizerPreset::Folk, "Folk").clicked();
                preset_changed |= ui.selectable_value(&mut self.equalizer.preset, EqualizerPreset::Custom, "Custom").clicked();
            });

        if preset_changed || ui.button("Apply Preset").clicked() {
            self.equalizer.apply_preset();
            self.update_equalizer_settings();
        }

        // For custom settings, update on slider change.
        if self.equalizer.preset == EqualizerPreset::Custom {
            ui.separator();
            ui.label("Custom adjustments:");
            let mut update_needed = false;
            for (i, band) in self.equalizer.bands.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(format!("Band {}:", i + 1));
                    if ui.add(egui::Slider::new(band, -10.0..=10.0).text("dB")).changed() {
                        update_needed = true;
                    }
                });
            }
            if update_needed {
                self.update_equalizer_settings();
            }
        }
    }
}

impl eframe::App for AudioPlayerApp {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        self.check_track_finished();
        self.process_youtube_result();
        self.process_key_commands();

        if !self.is_paused {
            self.current_position += ctx.input(|i| i.unstable_dt);
            if self.current_position >= self.total_duration {
                self.current_position = self.total_duration;
            }
        }

        ctx.set_visuals(if self.show_collections {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        });

        egui::TopBottomPanel::top("tab_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.selectable_label(self.current_tab == AppTab::Player, "Player").clicked() {
                    self.current_tab = AppTab::Player;
                }
                if ui.selectable_label(self.current_tab == AppTab::Equalizer, "Equalizer").clicked() {
                    self.current_tab = AppTab::Equalizer;
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            match self.current_tab {
                AppTab::Player => {
                    ui.add_space(10.0);
                    ui.heading(RichText::new("Rust Audio Player").size(30.0));
                    ui.separator();
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            if ui.button("Open File").clicked() {
                                if let Some(path) = FileDialog::new().pick_file() {
                                    let display_name = clean_title(&path.file_stem().unwrap().to_string_lossy());
                                    self.add_file(MediaItem {
                                        file_path: path,
                                        display_name,
                                        artist: None,
                                    });
                                }
                            }
                            if ui.button("Open Folder").clicked() {
                                if let Some(folder) = FileDialog::new().pick_folder() {
                                    self.add_folder(folder);
                                }
                            }
                        });
                        ui.separator();
                        ui.heading(RichText::new("YouTube Playback").size(20.0));
                        ui.horizontal(|ui| {
                            ui.label("YouTube URL:");
                            let response = ui.text_edit_singleline(&mut self.youtube_url);
                            if ui.button("Add to Collection").clicked() ||
                               (response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))) {
                                self.add_youtube_audio(self.youtube_url.clone());
                                self.youtube_url.clear();
                            }
                        });
                        if !self.download_status.is_empty() {
                            ui.label(&self.download_status);
                        }
                    });
                    ui.add_space(10.0);
                    ui.group(|ui| {
                        ui.heading(RichText::new("Now Playing").underline());
                        if let Some(idx) = self.current_index {
                            if let Some(item) = self.queue.get(idx) {
                                ui.label(format!("{}", item.display_name));
                                let mut progress = self.current_position;
                                if ui.add(egui::Slider::new(&mut progress, 0.0..=self.total_duration)
                                    .text(format!("{:.0} / {:.0} sec", self.current_position, self.total_duration))).changed() {
                                    self.seek_to(progress);
                                }
                            }
                        } else {
                            ui.label("No track playing.");
                        }
                        ui.horizontal(|ui| {
                            if ui.button("Prev").clicked() {
                                self.prev_track();
                            }
                            if self.is_paused {
                                if ui.button("Resume").clicked() {
                                    self.resume();
                                }
                            } else {
                                if ui.button("Pause").clicked() {
                                    self.pause();
                                }
                            }
                            if ui.button("Next").clicked() {
                                self.next_track();
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.label("Volume:");
                            let volume_slider = ui.add(egui::Slider::new(&mut self.volume, 0.0..=1.0));
                            if volume_slider.changed() {
                                self.set_volume(self.volume);
                            }
                        });
                        ui.horizontal(|ui| {
                            if ui.checkbox(&mut self.shuffle, "Shuffle")
                                .on_hover_text("Play tracks in random order")
                                .changed() {
                                // Optionally handle shuffle changes.
                            }
                        });
                    });
                    ui.add_space(10.0);
                    ui.group(|ui| {
                        ui.heading(RichText::new("Queue").underline());
                        egui::ScrollArea::vertical().max_height(150.0).show(ui, |ui| {
                            for i in 0..self.queue.len() {
                                let item = self.queue[i].clone();
                                ui.horizontal(|ui| {
                                    let is_current = Some(i) == self.current_index;
                                    let text = if is_current {
                                        RichText::new(format!("> {}", item.display_name)).strong()
                                    } else {
                                        RichText::new(format!("  {}", item.display_name))
                                    };
                                    ui.label(text);
                                    if ui.interact(ui.min_rect(), egui::Id::new(format!("track_{}", i)), egui::Sense::click()).clicked() {
                                        self.current_index = Some(i);
                                        self.play_current();
                                    }
                                });
                            }
                        });
                    });
                }
                AppTab::Equalizer => {
                    self.draw_equalizer_tab(ui);
                }
            }
        });

        if self.show_collections {
            egui::SidePanel::right("collections_panel")
                .min_width(250.0)
                .default_width(300.0)
                .resizable(true)
                .show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.heading(RichText::new("My Collections").color(egui::Color32::from_rgb(100, 200, 255)));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Search:");
                        ui.text_edit_singleline(&mut self.collections_search);
                        if ui.button("Clear").clicked() {
                            self.collections_search.clear();
                            self.show_youtube_input = false;
                        }
                    });
                    ui.separator();
                    let items = self.load_collections();
                    let filtered_items: Vec<&MediaItem> = if self.collections_search.is_empty() {
                        items.iter().collect()
                    } else {
                        let search_term = self.collections_search.to_lowercase();
                        items.iter()
                            .filter(|item| item.display_name.to_lowercase().contains(&search_term))
                            .collect()
                    };
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 6.0;
                        for item in filtered_items.iter() {
                            ui.horizontal(|ui| {
                                if ui.label(RichText::new(&item.display_name).strong())
                                    .on_hover_text("Click to play now")
                                    .clicked() {
                                    self.queue.insert(0, (*item).clone());
                                    self.current_index = Some(0);
                                    self.play_current();
                                }
                                if ui.button("Add to Queue").clicked() {
                                    self.add_file((*item).clone());
                                }
                            });
                        }
                        if filtered_items.is_empty() && !self.collections_search.is_empty() {
                            ui.add_space(10.0);
                            ui.vertical_centered(|ui| {
                                ui.label(RichText::new(format!("\"{}\" not found", self.collections_search))
                                    .color(egui::Color32::GRAY));
                                if !self.show_youtube_input {
                                    if ui.button("Add from YouTube").clicked() {
                                        self.show_youtube_input = true;
                                        self.youtube_search_url = String::new();
                                    }
                                } else {
                                    ui.add_space(5.0);
                                    ui.label("Enter YouTube URL:");
                                    let response = ui.text_edit_singleline(&mut self.youtube_search_url);
                                    ui.horizontal(|ui| {
                                        if ui.button("Add").clicked() ||
                                           (response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))) {
                                            if !self.youtube_search_url.is_empty() {
                                                self.add_youtube_audio(self.youtube_search_url.clone());
                                                self.youtube_search_url.clear();
                                                self.show_youtube_input = false;
                                            }
                                        }
                                        if ui.button("Cancel").clicked() {
                                            self.show_youtube_input = false;
                                        }
                                    });
                                }
                            });
                        } else if items.is_empty() {
                            ui.vertical_centered(|ui| {
                                ui.add_space(20.0);
                                ui.label(RichText::new("No items in collection")
                                    .color(egui::Color32::GRAY)
                                    .italics());
                                ui.add_space(10.0);
                                ui.label(RichText::new("Download songs via YouTube URL")
                                    .color(egui::Color32::GRAY)
                                    .small());
                            });
                        }
                    });
                });
        }

        ctx.request_repaint();
    }
}

fn main() {
    let options = eframe::NativeOptions {
        viewport: ViewportBuilder::default().with_inner_size([1200.0, 600.0]),
        ..Default::default()
    };
    
    eframe::run_native(
        "Rust Audio Player",
        options,
        Box::new(|cc| {
            // Create the theme and get the context from CreationContext
            let app_theme = theme::Theme::dark();
            app_theme.apply_to_ctx(&cc.egui_ctx); // Use cc.egui_ctx instead of ctx
            
            Ok(Box::new(AudioPlayerApp::new()))
        }),
    );
}
