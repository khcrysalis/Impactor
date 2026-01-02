use eframe::egui;
use eframe::epaint::ColorImage;
use futures::StreamExt;
use plume_utils::Device;
use std::thread;

use idevice::usbmuxd::{UsbmuxdConnection, UsbmuxdListenEvent};
use tokio::{runtime::Builder, sync::mpsc};

// -----------------------------------------------------------------------------
// Messages sent FROM async layer TO egui
// -----------------------------------------------------------------------------
enum AppMessage {
    DeviceConnected(Device),
    DeviceDisconnected(u32),
    Error(String),
    PackageSelected(String),
    PackageDeselected,
}

// -----------------------------------------------------------------------------
// egui app
// -----------------------------------------------------------------------------
#[derive(Default)]
struct ImpactorApp {
    devices: Vec<Device>,
    selected_device: Option<u32>,
    receiver: Option<mpsc::UnboundedReceiver<AppMessage>>,

    last_selected_package: Option<String>,

    install_image: Option<egui::TextureHandle>,
}

impl eframe::App for ImpactorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // ---- async message handling ----
        if let Some(mut rx) = self.receiver.take() {
            while let Ok(msg) = rx.try_recv() {
                self.handle_message(msg);
            }
            self.receiver = Some(rx);
        }

        // ---- Load embedded image once ----
        if self.install_image.is_none() {
            if let Ok(image) = load_embedded_install_image() {
                self.install_image =
                    Some(ctx.load_texture("install_png", image, Default::default()));
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            // -----------------------------------------------------------------
            // Top bar
            // -----------------------------------------------------------------
            ui.horizontal(|ui| {
                egui::ComboBox::from_id_salt("device_picker")
                    .selected_text(
                        self.selected_device
                            .and_then(|id| self.devices.iter().find(|d| d.device_id == id))
                            .map(|d| d.to_string())
                            .unwrap_or_else(|| "No device".into()),
                    )
                    .show_ui(ui, |ui| {
                        for dev in &self.devices {
                            ui.selectable_value(
                                &mut self.selected_device,
                                Some(dev.device_id),
                                dev.to_string(),
                            );
                        }
                    });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("⚙ Settings").clicked() {
                        println!("Settings clicked");
                    }
                    if ui.button("Utilities").clicked() {
                        println!("Utilities clicked");
                    }
                });
            });

            // -----------------------------------------------------------------
            // Drag & drop area
            // -----------------------------------------------------------------
            let available = ui.available_size();
            let drag_rect = ui.allocate_exact_size(available, egui::Sense::hover()).0;

            let fixed_size = egui::Vec2::new(128.0, 128.0);
            let spacing = 8.0;
            let text_height =
                ui.fonts(|f| f.row_height(&egui::TextStyle::Heading.resolve(ui.style())));
            let total_height = fixed_size.y + spacing + text_height;

            let top = drag_rect.center().y - total_height / 2.0;
            let image_rect = egui::Rect::from_min_size(
                egui::Pos2::new(drag_rect.center().x - fixed_size.x / 2.0, top),
                fixed_size,
            );

            if let Some(texture) = &self.install_image {
                ui.painter().image(
                    texture.id(),
                    image_rect,
                    egui::Rect::from_min_max(egui::Pos2::ZERO, egui::Pos2::new(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
            }

            let text_pos = egui::Pos2::new(drag_rect.center().x, top + fixed_size.y + spacing);
            ui.painter().text(
                text_pos,
                egui::Align2::CENTER_TOP,
                "Drag & Drop IPA Here",
                egui::TextStyle::Heading.resolve(ui.style()),
                ui.visuals().weak_text_color(),
            );

            // -----------------------------------------------------------------
            // File drop handling
            // -----------------------------------------------------------------
            ctx.input(|i| {
                for file in &i.raw.dropped_files {
                    if let Some(path) = &file.path {
                        if matches!(
                            path.extension().and_then(|e| e.to_str()),
                            Some("ipa" | "tipa")
                        ) {
                            self.handle_message(AppMessage::PackageSelected(
                                path.display().to_string(),
                            ));
                        }
                    }
                }
            });
        });

        ctx.request_repaint();
    }
}

// -----------------------------------------------------------------------------
// Message handling (egui-side)
// -----------------------------------------------------------------------------
impl ImpactorApp {
    fn handle_message(&mut self, msg: AppMessage) {
        match msg {
            AppMessage::DeviceConnected(device) => {
                if !self.devices.iter().any(|d| d.device_id == device.device_id) {
                    if self.selected_device.is_none() {
                        self.selected_device = Some(device.device_id);
                    }
                    self.devices.push(device);
                }
            }

            AppMessage::DeviceDisconnected(device_id) => {
                self.devices.retain(|d| d.device_id != device_id);
                if self.selected_device == Some(device_id) {
                    self.selected_device = self.devices.first().map(|d| d.device_id);
                }
            }

            AppMessage::PackageSelected(path) => {
                println!("Selected package: {}", path);
                self.last_selected_package = Some(path);
            }
            AppMessage::PackageDeselected => {
                self.last_selected_package = None;
            }

            AppMessage::Error(err) => {
                eprintln!("Error: {}", err);
            }
        }
    }
}

// -----------------------------------------------------------------------------
// usbmuxd listener
// -----------------------------------------------------------------------------
fn spawn_usbmuxd_listener(sender: mpsc::UnboundedSender<AppMessage>) {
    thread::spawn(move || {
        let rt = Builder::new_current_thread().enable_io().build().unwrap();

        rt.block_on(async move {
            let Ok(mut muxer) = UsbmuxdConnection::default().await else {
                return;
            };

            if let Ok(devices) = muxer.get_devices().await {
                for dev in devices {
                    let _ = sender.send(AppMessage::DeviceConnected(Device::new(dev).await));
                }
            }

            let Ok(mut stream) = muxer.listen().await else {
                let _ = sender.send(AppMessage::Error("Failed to listen".to_string()));
                return;
            };

            while let Some(event) = stream.next().await {
                let msg = match event {
                    Ok(UsbmuxdListenEvent::Connected(dev)) => {
                        AppMessage::DeviceConnected(Device::new(dev).await)
                    }
                    Ok(UsbmuxdListenEvent::Disconnected(id)) => AppMessage::DeviceDisconnected(id),
                    Err(e) => AppMessage::Error(e.to_string()),
                };

                if sender.send(msg).is_err() {
                    break;
                }
            }
        });
    });
}

// -----------------------------------------------------------------------------
// Load embedded image as ColorImage
// -----------------------------------------------------------------------------
fn load_embedded_install_image() -> Result<ColorImage, String> {
    const INSTALL_PNG: &[u8] = include_bytes!("./install.png");
    let image = image::load_from_memory(INSTALL_PNG).map_err(|e| e.to_string())?;
    let size = [image.width() as usize, image.height() as usize];
    let image = image.to_rgba8();
    Ok(ColorImage::from_rgba_unmultiplied(size, &image))
}

// -----------------------------------------------------------------------------
// Entry point
// -----------------------------------------------------------------------------
pub const APP_NAME: &str = concat!("Impactor – Version ", env!("CARGO_PKG_VERSION"));

fn main() -> eframe::Result<()> {
    env_logger::init();

    let (tx, rx) = mpsc::unbounded_channel();
    spawn_usbmuxd_listener(tx);

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([540.0, 400.0])
            .with_resizable(false),
        ..Default::default()
    };

    eframe::run_native(
        APP_NAME,
        options,
        Box::new(|_cc| {
            Ok(Box::new(ImpactorApp {
                receiver: Some(rx),
                ..Default::default()
            }))
        }),
    )
}
