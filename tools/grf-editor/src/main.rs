mod app;
mod file_list;
mod preview;
mod sprite_preview;
mod tree;

use eframe::egui;

fn setup_korean_font(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    const CJK_FONT_PATH: &str = "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc";
    if let Ok(font_data) = std::fs::read(CJK_FONT_PATH) {
        fonts.font_data.insert(
            "noto-sans-kr".to_owned(),
            std::sync::Arc::new(egui::FontData {
                font: std::borrow::Cow::Owned(font_data),
                index: 1,
                tweak: Default::default(),
            }),
        );
        if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
            family.push("noto-sans-kr".to_owned());
        }
        if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Monospace) {
            family.push("noto-sans-kr".to_owned());
        }
    } else {
        eprintln!("Warning: Korean font not found at {CJK_FONT_PATH}");
    }

    ctx.set_fonts(fonts);
}

fn main() -> eframe::Result {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let grf_paths: Vec<String> = std::env::args().skip(1).collect();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1024.0, 700.0])
            .with_min_inner_size([640.0, 400.0]),
        ..Default::default()
    };
    eframe::run_native(
        "GRF Editor",
        options,
        Box::new(|cc| {
            setup_korean_font(&cc.egui_ctx);
            let mut app = app::GrfEditorApp::default();
            for path in &grf_paths {
                app.open_grf(std::path::Path::new(path));
            }
            Ok(Box::new(app))
        }),
    )
}
