//! 配色トークン、Visuals、フォント。

use std::collections::BTreeMap;
use std::sync::Arc;

use egui::{
    Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Margin, Shadow, Stroke,
    TextStyle, Theme, Visuals,
};

use crate::ui::text;

/// 埋め込み日本語フォントの登録名。
pub const FONT_NAME_JP: &str = "noto_sans_jp";

const NOTO_SANS_JP_REGULAR: &[u8] = include_bytes!("../../assets/fonts/NotoSansJP-Regular.ttf");

// 余白（4 の倍数）
pub const SPACE_XS: f32 = 4.0;
pub const SPACE_SM: f32 = 8.0;
pub const SPACE_MD: f32 = 12.0;
pub const SPACE_LG: f32 = 16.0;
pub const SPACE_XL: f32 = 24.0;

// 角の丸み（2 種類のみ）
pub const RADIUS_CONTROL: u8 = 6;
pub const RADIUS_SURFACE: u8 = 10;

// 文字の大きさ
pub const FONT_SIZE_ELAPSED: f32 = 32.0;
pub const FONT_SIZE_CATCH: f32 = 16.0;
pub const FONT_SIZE_BODY: f32 = 14.0;
pub const FONT_SIZE_CAPTION: f32 = 12.0;

// レイアウトの定数
pub const HEADER_HEIGHT: f32 = 48.0;
pub const CONTROLS_HEIGHT: f32 = 72.0;
pub const CATCH_BUTTON_HEIGHT: f32 = 44.0;
pub const EDIT_BAR_HEIGHT: f32 = 56.0;
pub const STATUS_BAR_HEIGHT: f32 = 30.0;
pub const SETTINGS_PANEL_WIDTH: f32 = 320.0;

/// 配色トークン（§5.9）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Palette {
    pub bg: Color32,
    pub panel: Color32,
    pub raised: Color32,
    pub border: Color32,
    pub text: Color32,
    pub text_dim: Color32,
    pub text_muted: Color32,
    pub accent: Color32,
    pub accent_strong: Color32,
    pub on_accent: Color32,
    pub wave: Color32,
    pub wave_dim: Color32,
    pub selection_bg: Color32,
    pub rec: Color32,
    pub warn: Color32,
    pub ok: Color32,
}

impl Palette {
    pub const DARK: Self = Self {
        bg: Color32::from_rgb(0x0b, 0x0e, 0x18),
        panel: Color32::from_rgb(0x12, 0x17, 0x26),
        raised: Color32::from_rgb(0x1b, 0x21, 0x33),
        border: Color32::from_rgb(0x2c, 0x33, 0x48),
        text: Color32::from_rgb(0xee, 0xf1, 0xf8),
        text_dim: Color32::from_rgb(0xc6, 0xcb, 0xd8),
        text_muted: Color32::from_rgb(0x87, 0x91, 0xa8),
        accent: Color32::from_rgb(0x3e, 0x95, 0xff),
        accent_strong: Color32::from_rgb(0x21, 0x68, 0xf2),
        on_accent: Color32::from_rgb(0x0b, 0x0e, 0x18),
        wave: Color32::from_rgb(0x3e, 0x95, 0xff),
        wave_dim: Color32::from_rgb(0x2c, 0x33, 0x48),
        // accent (#3e95ff) の 14% 不透明度（255 * 0.14 = 35.7 -> 36）
        selection_bg: Color32::from_rgba_unmultiplied_const(0x3e, 0x95, 0xff, 36),
        rec: Color32::from_rgb(0xff, 0x52, 0x63),
        warn: Color32::from_rgb(0xff, 0xb5, 0x47),
        ok: Color32::from_rgb(0x3e, 0xcf, 0x8e),
    };

    pub const LIGHT: Self = Self {
        bg: Color32::from_rgb(0xee, 0xf1, 0xf8),
        panel: Color32::from_rgb(0xff, 0xff, 0xff),
        raised: Color32::from_rgb(0xef, 0xf1, 0xf6),
        border: Color32::from_rgb(0xc6, 0xcb, 0xd8),
        text: Color32::from_rgb(0x0b, 0x0e, 0x18),
        text_dim: Color32::from_rgb(0x2c, 0x33, 0x48),
        text_muted: Color32::from_rgb(0x5d, 0x66, 0x80),
        accent: Color32::from_rgb(0x21, 0x68, 0xf2),
        accent_strong: Color32::from_rgb(0x1a, 0x52, 0xc4),
        on_accent: Color32::from_rgb(0xff, 0xff, 0xff),
        wave: Color32::from_rgb(0x21, 0x68, 0xf2),
        wave_dim: Color32::from_rgb(0xc6, 0xcb, 0xd8),
        // accent (#2168f2) の 12% 不透明度（255 * 0.12 = 30.6 -> 31）
        selection_bg: Color32::from_rgba_unmultiplied_const(0x21, 0x68, 0xf2, 31),
        rec: Color32::from_rgb(0xd9, 0x2b, 0x3e),
        warn: Color32::from_rgb(0xb3, 0x6b, 0x00),
        ok: Color32::from_rgb(0x14, 0x80, 0x4f),
    };

    pub const fn for_theme(theme: Theme) -> Self {
        match theme {
            Theme::Dark => Self::DARK,
            Theme::Light => Self::LIGHT,
        }
    }

    pub fn from_ctx(ctx: &egui::Context) -> Self {
        Self::for_theme(ctx.theme())
    }

    pub fn from_ui(ui: &egui::Ui) -> Self {
        if ui.visuals().dark_mode {
            Self::DARK
        } else {
            Self::LIGHT
        }
    }
}

/// テーマに応じた `egui::Visuals` を組み立てる（§5.9）。
pub fn visuals(theme: Theme) -> Visuals {
    let p = Palette::for_theme(theme);
    let mut v = match theme {
        Theme::Dark => Visuals::dark(),
        Theme::Light => Visuals::light(),
    };

    let control_radius = CornerRadius::same(RADIUS_CONTROL);
    let surface_radius = CornerRadius::same(RADIUS_SURFACE);
    // accent の 35% 不透明度（255 * 0.35 = 89.25 -> 89）
    let selection_fill =
        Color32::from_rgba_unmultiplied_const(p.accent.r(), p.accent.g(), p.accent.b(), 89);

    v.panel_fill = p.bg;
    v.window_fill = p.bg;
    v.extreme_bg_color = p.panel;
    v.faint_bg_color = p.raised;
    v.window_stroke = Stroke::new(1.0, p.border);
    v.override_text_color = None;
    v.weak_text_color = Some(p.text_muted);

    v.widgets.noninteractive.bg_fill = p.bg;
    v.widgets.noninteractive.weak_bg_fill = p.bg;
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, p.border);
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, p.text_dim);
    v.widgets.noninteractive.corner_radius = control_radius;

    v.widgets.inactive.bg_fill = p.raised;
    v.widgets.inactive.weak_bg_fill = p.raised;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, p.border);
    v.widgets.inactive.fg_stroke = Stroke::new(1.0, p.text);
    v.widgets.inactive.corner_radius = control_radius;

    v.widgets.hovered.bg_fill = p.border;
    v.widgets.hovered.weak_bg_fill = p.border;
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, p.accent);
    v.widgets.hovered.fg_stroke = Stroke::new(1.0, p.text);
    v.widgets.hovered.corner_radius = control_radius;

    v.widgets.active.bg_fill = p.accent_strong;
    v.widgets.active.weak_bg_fill = p.accent_strong;
    v.widgets.active.bg_stroke = Stroke::new(1.0, p.accent);
    v.widgets.active.fg_stroke = Stroke::new(1.0, p.on_accent);
    v.widgets.active.corner_radius = control_radius;

    v.widgets.open.bg_fill = p.raised;
    v.widgets.open.weak_bg_fill = p.raised;
    v.widgets.open.bg_stroke = Stroke::new(1.0, p.border);
    v.widgets.open.fg_stroke = Stroke::new(1.0, p.text);
    v.widgets.open.corner_radius = control_radius;

    v.selection.bg_fill = selection_fill;
    v.selection.stroke = Stroke::new(1.0, p.accent);

    v.hyperlink_color = p.accent;
    v.warn_fg_color = p.warn;
    v.error_fg_color = p.rec;

    v.window_corner_radius = surface_radius;
    v.menu_corner_radius = surface_radius;
    v.window_shadow = Shadow::NONE;
    v.popup_shadow = Shadow::NONE;
    v.slider_trailing_fill = true;
    v.text_cursor.stroke = Stroke::new(2.0, p.accent);

    v
}

/// 設定パネルの左端だけに付ける影（§5.9）。
pub fn settings_panel_shadow(dark_mode: bool) -> Shadow {
    let alpha = if dark_mode { 96 } else { 32 };
    Shadow {
        offset: [-6, 0],
        blur: 16,
        spread: 0,
        color: Color32::from_black_alpha(alpha),
    }
}

/// 日本語フォント（Noto Sans JP Regular）を登録する。
pub fn install_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        FONT_NAME_JP.to_owned(),
        Arc::new(FontData::from_static(NOTO_SANS_JP_REGULAR)),
    );

    // 本文（Proportional）の先頭に入れる
    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(0, FONT_NAME_JP.to_owned());

    // 等幅（Monospace）では英数字は既定の等幅を優先し、日本語のフォールバックとして末尾に置く
    fonts
        .families
        .entry(FontFamily::Monospace)
        .or_default()
        .push(FONT_NAME_JP.to_owned());

    ctx.set_fonts(fonts);
}

fn text_styles() -> BTreeMap<TextStyle, FontId> {
    [
        (
            TextStyle::Small,
            FontId::new(FONT_SIZE_CAPTION, FontFamily::Proportional),
        ),
        (
            TextStyle::Body,
            FontId::new(FONT_SIZE_BODY, FontFamily::Proportional),
        ),
        (
            TextStyle::Button,
            FontId::new(FONT_SIZE_BODY, FontFamily::Proportional),
        ),
        (
            TextStyle::Heading,
            FontId::new(FONT_SIZE_CATCH, FontFamily::Proportional),
        ),
        (
            TextStyle::Monospace,
            FontId::new(FONT_SIZE_BODY, FontFamily::Monospace),
        ),
    ]
    .into()
}

/// フォント・ダーク/ライト双方の Visuals・文字サイズ・余白を `egui::Context` へ適用する。
/// テーマの既定は `ThemePreference::System` のまま OS の設定に従う。
pub fn apply(ctx: &egui::Context) {
    install_fonts(ctx);
    ctx.set_visuals_of(Theme::Dark, visuals(Theme::Dark));
    ctx.set_visuals_of(Theme::Light, visuals(Theme::Light));

    let styles = text_styles();
    ctx.all_styles_mut(|style| {
        style.text_styles = styles.clone();
        style.drag_value_text_style = TextStyle::Monospace;
        style.spacing.item_spacing = egui::vec2(SPACE_SM, SPACE_SM);
        style.spacing.button_padding = egui::vec2(SPACE_MD, SPACE_SM);
        style.spacing.window_margin = Margin::same(16);
    });
}

/// T6 の確認用: 全配色トークンと文字サイズ・日本語文言を一覧表示する。
pub fn debug_swatches(ui: &mut egui::Ui) {
    let p = Palette::from_ui(ui);
    let mode_label = if ui.visuals().dark_mode {
        text::THEME_DARK
    } else {
        text::THEME_LIGHT
    };

    ui.heading(format!(
        "{} 配色・フォント確認（{}）",
        text::APP_TITLE,
        mode_label
    ));
    ui.add_space(SPACE_SM);

    let tokens: [(&str, Color32, &str); 16] = [
        ("bg", p.bg, "背景"),
        ("panel", p.panel, "パネル / 入力欄の溝"),
        ("raised", p.raised, "ボタン・浮いた面"),
        ("border", p.border, "境界線"),
        ("text", p.text, "主文字"),
        ("text_dim", p.text_dim, "副文字"),
        ("text_muted", p.text_muted, "補足文字"),
        ("accent", p.accent, "アクセント / Catch"),
        (
            "accent_strong",
            p.accent_strong,
            "アクセント（押下・ホバー）",
        ),
        ("on_accent", p.on_accent, "アクセント上の文字"),
        ("wave", p.wave, "波形（選択内）"),
        ("wave_dim", p.wave_dim, "波形（選択外）"),
        ("selection_bg", p.selection_bg, "選択範囲の背景"),
        ("rec", p.rec, "録音・エラー"),
        ("warn", p.warn, "注意"),
        ("ok", p.ok, "取得中・成功"),
    ];

    egui::Frame::new()
        .fill(p.panel)
        .stroke(Stroke::new(1.0, p.border))
        .corner_radius(CornerRadius::same(RADIUS_SURFACE))
        .inner_margin(Margin::same(12))
        .show(ui, |ui| {
            egui::Grid::new("debug_swatches_grid")
                .num_columns(4)
                .spacing([SPACE_MD, SPACE_SM])
                .show(ui, |ui| {
                    for (name, color, desc) in tokens {
                        let [r, g, b, a] = color.to_srgba_unmultiplied();
                        let (rect, _) =
                            ui.allocate_exact_size(egui::vec2(48.0, 20.0), egui::Sense::hover());
                        ui.painter()
                            .rect_filled(rect, CornerRadius::same(RADIUS_CONTROL), p.bg);
                        ui.painter()
                            .rect_filled(rect, CornerRadius::same(RADIUS_CONTROL), color);
                        ui.painter().rect_stroke(
                            rect,
                            CornerRadius::same(RADIUS_CONTROL),
                            Stroke::new(1.0, p.border),
                            egui::StrokeKind::Inside,
                        );

                        ui.label(egui::RichText::new(name).monospace().color(p.text));
                        ui.label(
                            egui::RichText::new(format!("#{r:02x}{g:02x}{b:02x} (a={a})"))
                                .monospace()
                                .size(FONT_SIZE_CAPTION)
                                .color(p.text_muted),
                        );
                        ui.label(egui::RichText::new(desc).color(p.text_dim));
                        ui.end_row();
                    }
                });
        });

    ui.add_space(SPACE_LG);

    egui::Frame::new()
        .fill(p.panel)
        .stroke(Stroke::new(1.0, p.border))
        .corner_radius(CornerRadius::same(RADIUS_SURFACE))
        .inner_margin(Margin::same(12))
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new("0:12.3")
                    .font(FontId::new(FONT_SIZE_ELAPSED, FontFamily::Monospace))
                    .color(p.text),
            );
            ui.label(
                egui::RichText::new(format!(
                    "経過時間 32pt（等幅） / {} 30 {}",
                    text::CATCH,
                    text::SECONDS_UNIT
                ))
                .size(FONT_SIZE_CAPTION)
                .color(p.text_muted),
            );
            ui.add_space(SPACE_SM);

            ui.label(
                egui::RichText::new(format!("{} 30 {}（16pt）", text::CATCH, text::SECONDS_UNIT))
                    .size(FONT_SIZE_CATCH)
                    .color(p.text),
            );
            ui.label(
                egui::RichText::new(format!("本文 14pt: {}", text::IDLE_PLACEHOLDER))
                    .size(FONT_SIZE_BODY)
                    .color(p.text),
            );
            ui.label(
                egui::RichText::new(format!(
                    "補足 12pt: {} / {}",
                    text::HOTKEYS_HINT,
                    text::STATUS_RUNNING
                ))
                .size(FONT_SIZE_CAPTION)
                .color(p.text_muted),
            );
            ui.add_space(SPACE_SM);

            ui.horizontal(|ui| {
                let catch_btn = egui::Button::new(
                    egui::RichText::new(format!("{} 30 {}", text::CATCH, text::SECONDS_UNIT))
                        .size(FONT_SIZE_CATCH)
                        .color(p.on_accent),
                )
                .fill(p.accent)
                .min_size(egui::vec2(160.0, CATCH_BUTTON_HEIGHT));
                ui.add(catch_btn);

                let rec_btn = egui::Button::new(
                    egui::RichText::new(text::REC)
                        .size(FONT_SIZE_BODY)
                        .color(p.rec),
                )
                .fill(p.raised)
                .min_size(egui::vec2(88.0, CATCH_BUTTON_HEIGHT));
                ui.add(rec_btn);

                ui.add(egui::Button::new(text::EXPORT_BUTTON));
                ui.add(egui::Button::new(text::SETTINGS_BUTTON));
            });
            ui.add_space(SPACE_SM);

            ui.label(
                egui::RichText::new(text::DISCLAIMER)
                    .size(FONT_SIZE_CAPTION)
                    .color(p.text_dim),
            );
        });
}
