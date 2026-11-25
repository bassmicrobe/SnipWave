use std::process::ExitCode;
use std::sync::Arc;

use snipwave::audio::engine::AudioEngine;
use snipwave::config::AppConfig;
use snipwave::ui::app::SnipWaveApp;
use snipwave::ui::{text, theme};

// T6 の一時確認用ラッパー（T9 で SnipWaveApp を直接返す形に戻す）。
struct ThemePreviewApp {
    app: SnipWaveApp,
    preference: egui::ThemePreference,
}

impl ThemePreviewApp {
    fn new(app: SnipWaveApp, preference: egui::ThemePreference) -> Self {
        Self { app, preference }
    }
}

impl eframe::App for ThemePreviewApp {
    fn logic(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.app.logic(ctx, frame);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label("テーマ確認切替:");
                for (pref, label) in [
                    (egui::ThemePreference::System, text::THEME_SYSTEM),
                    (egui::ThemePreference::Dark, text::THEME_DARK),
                    (egui::ThemePreference::Light, text::THEME_LIGHT),
                ] {
                    if ui
                        .selectable_value(&mut self.preference, pref, label)
                        .clicked()
                    {
                        ui.ctx().set_theme(pref);
                    }
                }
            });
            ui.separator();
            egui::ScrollArea::vertical().show(ui, |ui| {
                theme::debug_swatches(ui);
            });
        });
    }
}

fn initial_theme_preference() -> egui::ThemePreference {
    match std::env::var("SNIPWAVE_THEME") {
        Ok(val) if val.eq_ignore_ascii_case("dark") => egui::ThemePreference::Dark,
        Ok(val) if val.eq_ignore_ascii_case("light") => egui::ThemePreference::Light,
        _ => egui::ThemePreference::System,
    }
}

fn main() -> ExitCode {
    env_logger::init();

    let loaded = match AppConfig::load() {
        Ok(loaded) => loaded,
        Err(err) => {
            log::error!("設定の読み込みに失敗しました: {err}");
            eprintln!("エラー: {err}");
            return ExitCode::FAILURE;
        }
    };

    let icon =
        match eframe::icon_data::from_png_bytes(include_bytes!("../assets/icons/app-256.png")) {
            Ok(icon) => icon,
            Err(err) => {
                log::error!("ウィンドウアイコンの読み込みに失敗しました: {err}");
                eprintln!("エラー: ウィンドウアイコンの読み込みに失敗しました: {err}");
                return ExitCode::FAILURE;
            }
        };

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(text::APP_TITLE)
            .with_inner_size([960.0, 560.0])
            .with_min_inner_size([760.0, 440.0])
            .with_icon(icon),
        ..Default::default()
    };

    let initial_pref = initial_theme_preference();

    let result = eframe::run_native(
        text::APP_TITLE,
        options,
        Box::new(move |cc| {
            theme::apply(&cc.egui_ctx);
            cc.egui_ctx.set_theme(initial_pref);

            let ctx = cc.egui_ctx.clone();
            let engine = AudioEngine::start(Arc::new(move || {
                ctx.request_repaint();
            }))?;
            let app = SnipWaveApp::new(cc, loaded, engine);
            Ok(Box::new(ThemePreviewApp::new(app, initial_pref)))
        }),
    );

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            log::error!("アプリケーションの起動に失敗しました: {err}");
            eprintln!("エラー: {err}");
            ExitCode::FAILURE
        }
    }
}
