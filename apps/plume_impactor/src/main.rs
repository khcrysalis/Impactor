use eframe::egui;

#[derive(Default)]
struct ImpactorApp {
    counter: i32,
    input: String,
    status: String,
}

impl eframe::App for ImpactorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading(APP_NAME);

            ui.separator();
        });
    }
}

enum ImpactorAppMessage {}

impl ImpactorApp {
    fn handle_message(&mut self, msg: ImpactorAppMessage) {
        match msg {}
    }
}

pub const APP_NAME: &str = concat!("Impactor – Version ", env!("CARGO_PKG_VERSION"));

fn main() -> eframe::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([540.0, 400.0])
            .with_resizable(false),
        ..Default::default()
    };
    eframe::run_native(
        APP_NAME,
        options,
        Box::new(|_cc| Ok(Box::new(ImpactorApp::default()))),
    )
}
