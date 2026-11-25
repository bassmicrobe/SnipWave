use crate::audio::engine::AudioEngine;
use crate::config::LoadedConfig;
use crate::hotkeys::HotkeyAction;
use crate::tray::TrayAction;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppAction {
    Hotkey(HotkeyAction),
    Tray(TrayAction),
}

pub struct SnipWaveApp {
    _loaded: LoadedConfig,
    _engine: AudioEngine,
}

impl SnipWaveApp {
    pub fn new(
        _cc: &eframe::CreationContext<'_>,
        loaded: LoadedConfig,
        engine: AudioEngine,
    ) -> Self {
        Self {
            _loaded: loaded,
            _engine: engine,
        }
    }
}

impl eframe::App for SnipWaveApp {
    fn logic(&mut self, _ctx: &egui::Context, _frame: &mut eframe::Frame) {}

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |_ui| {});
    }
}
