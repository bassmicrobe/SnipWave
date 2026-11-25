use std::process::ExitCode;
use std::sync::Arc;

use snipwave::audio::engine::AudioEngine;
use snipwave::config::AppConfig;
use snipwave::ui::app::SnipWaveApp;

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

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("SnipWave")
            .with_inner_size([960.0, 560.0])
            .with_min_inner_size([760.0, 440.0]),
        // T6 で assets/icons/app-256.png を追加したあとに with_icon を有効にする。
        ..Default::default()
    };

    let result = eframe::run_native(
        "SnipWave",
        options,
        Box::new(move |cc| {
            let ctx = cc.egui_ctx.clone();
            let engine = AudioEngine::start(Arc::new(move || {
                ctx.request_repaint();
            }))?;
            Ok(Box::new(SnipWaveApp::new(cc, loaded, engine)))
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
