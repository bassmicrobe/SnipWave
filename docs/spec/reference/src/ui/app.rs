use crate::audio::buffer::{AudioRecorderEngine, CHANNELS, SAMPLE_RATE};
use crate::config::{AppConfig, MAX_BUFFER_SECONDS};
use eframe::egui;

pub struct SnipWaveApp {
    config: AppConfig,
    engine: AudioRecorderEngine,
    captured_audio: Vec<f32>,
    trim_start: f32, // 0.0 ~ 1.0
    trim_end: f32,   // 0.0 ~ 1.0
    status_msg: String,
}

impl SnipWaveApp {
    pub fn new(engine: AudioRecorderEngine) -> Self {
        Self {
            config: AppConfig::load(),
            engine,
            captured_audio: Vec::new(),
            trim_start: 0.0,
            trim_end: 1.0,
            status_msg: "Ready".to_string(),
        }
    }

    fn export_wav(&mut self) {
        if self.captured_audio.is_empty() {
            return;
        }
        let total = self.captured_audio.len();
        let start_idx = ((total as f32 * self.trim_start) as usize / 2) * 2;
        let end_idx = ((total as f32 * self.trim_end) as usize / 2) * 2;

        let filename = format!("snip_{}.wav", chrono::Local::now().format("%Y%m%d_%H%M%S"));
        let path = self.config.output_dir.join(filename);

        let spec = hound::WavSpec {
            channels: CHANNELS,
            sample_rate: SAMPLE_RATE,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };

        if let Ok(mut writer) = hound::WavWriter::create(&path, spec) {
            for &sample in &self.captured_audio[start_idx..end_idx.min(total)] {
                let _ = writer.write_sample(sample);
            }
            self.status_msg = format!("Saved: {}", path.display());
        }
    }
}

impl eframe::App for SnipWaveApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("SnipWave");

            // 録音コントロールバー（2パターン）
            ui.horizontal(|ui| {
                // パターンA：遡りキャプチャ
                if ui.button(format!("⏪ Catch Last {}s", self.config.buffer_seconds)).clicked() {
                    self.captured_audio = self.engine.catch_retroactive(self.config.buffer_seconds);
                    self.trim_start = 0.0;
                    self.trim_end = 1.0;
                    self.status_msg = format!("Caught last {}s", self.config.buffer_seconds);
                }

                // パターンB：ライブ録音（開始 / 停止トグル）
                if self.engine.is_recording() {
                    if ui.button("■ Stop Rec").clicked() {
                        self.captured_audio = self.engine.stop_live_recording();
                        self.trim_start = 0.0;
                        self.trim_end = 1.0;
                        self.status_msg = "Live recording captured".to_string();
                    }
                } else if ui.button("● Live Rec").clicked() {
                    self.engine.start_live_recording();
                    self.status_msg = "Recording live...".to_string();
                }

                ui.separator();

                // 遡り秒数設定
                ui.label("遡り秒数:");
                if ui
                    .add(egui::Slider::new(&mut self.config.buffer_seconds, 5..=MAX_BUFFER_SECONDS).suffix(" s"))
                    .changed()
                {
                    self.config.save();
                }
            });

            ui.separator();

            // 波形描画エリア
            let (rect, _response) = ui.allocate_exact_size(
                egui::vec2(ui.available_width(), 150.0),
                egui::Sense::click_and_drag(),
            );
            ui.painter().rect_filled(rect, 4.0, egui::Color32::from_gray(25));

            // トリミング＆書き出し
            ui.horizontal(|ui| {
                ui.add(egui::Slider::new(&mut self.trim_start, 0.0..=self.trim_end).text("Start"));
                ui.add(egui::Slider::new(&mut self.trim_end, self.trim_start..=1.0).text("End"));
                if ui.button("Export WAV").clicked() {
                    self.export_wav();
                }
            });

            ui.label(&self.status_msg);
        });
    }
}
