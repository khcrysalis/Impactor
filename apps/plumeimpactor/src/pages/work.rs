use wxdragon::prelude::*;

#[derive(Clone)]
pub struct WorkPage {
    pub panel: Panel,
    status_text: StaticText,
    status_gauge: Gauge,
    back_button: Button,
}

pub fn create_work_page(frame: &Frame) -> WorkPage {
    let panel = Panel::builder(frame).build();
    let sizer = BoxSizer::builder(Orientation::Vertical).build();

    let warning_text = StaticText::builder(&panel)
        .with_label("Preparing application before installation/or export, this will take a moment. Do not\ndisconnect the device until finished.")
        .build();
    let status_text = StaticText::builder(&panel)
        .with_label("Idle")
        .build();
    let status_gauge = Gauge::builder(&panel)
        .with_range(100)
        .with_size(Size::new(-1, 30))
        .build();
    sizer.add(&warning_text, 0, SizerFlag::AlignLeft | SizerFlag::Expand | SizerFlag::Left | SizerFlag::Right | SizerFlag::Bottom, 16);
    sizer.add(&status_text, 0, SizerFlag::AlignLeft | SizerFlag::Expand | SizerFlag::Top | SizerFlag::Left | SizerFlag::Right, 16);
    sizer.add(&status_gauge, 0, SizerFlag::Expand | SizerFlag::All, 45);

    sizer.add_stretch_spacer(1);

    let button_sizer = BoxSizer::builder(Orientation::Horizontal).build();
    let back_button = Button::builder(&panel)
        .with_label("Back")
        .build();
    back_button.enable(false);
    button_sizer.add(&back_button, 0, SizerFlag::All, 0);
    button_sizer.add_stretch_spacer(1);
    sizer.add_sizer(&button_sizer, 0, SizerFlag::Expand | SizerFlag::Left | SizerFlag::Bottom, 13);

    panel.set_sizer(sizer, true);

    WorkPage { 
        panel,
        status_text,
        status_gauge,
        back_button,
    }
}

impl WorkPage {
    pub fn set_status(&self, text: &str, progress: i32) {
        self.status_text.set_label(text);
        let new_value = std::cmp::min(progress, 100);
        self.status_gauge.set_value(new_value);
    }

    pub fn enable_back_button(&self, enable: bool) {
        self.back_button.enable(enable);
    }

    pub fn set_back_handler(&self, on_back: impl Fn() + 'static) {
        self.back_button.on_click(move |_| {
            on_back();
        });
    }
}
