use self::app::NodedApp;

mod app;
mod node;
mod render;
mod types;
mod widget;

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([400.0, 300.0])
            .with_min_inner_size([300.0, 220.0]),
        ..Default::default()
    };

    eframe::run_native("noded", native_options, Box::new(|cx| Ok(Box::new(NodedApp::new(cx)))))
}
