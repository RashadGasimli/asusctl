use std::f64::consts::PI;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use egui::{Button, RichText};
use rog_aura::layouts::KeyLayout;
use rog_platform::platform::Properties;

use crate::config::Config;
use crate::error::Result;
use crate::system_state::SystemState;
use crate::{Page, RogDbusClientBlocking};

pub struct RogApp {
    pub page: Page,
    pub states: Arc<Mutex<SystemState>>,
    // TODO: can probably just open and read whenever
    pub config: Config,
    /// Oscillator in percentage
    pub oscillator1: Arc<AtomicU8>,
    pub oscillator2: Arc<AtomicU8>,
    pub oscillator3: Arc<AtomicU8>,
    /// Frequency of oscillation
    pub oscillator_freq: Arc<AtomicU8>,
    /// A toggle that toggles true/false when the oscillator reaches 0
    pub oscillator_toggle: Arc<AtomicBool>,
    pub supported_interfaces: Vec<String>,
    pub supported_properties: Vec<Properties>,
}

impl RogApp {
    /// Called once before the first frame.
    pub fn new(
        config: Config,
        states: Arc<Mutex<SystemState>>,
        _cc: &eframe::CreationContext<'_>,
    ) -> Result<Self> {
        let (dbus, _) = RogDbusClientBlocking::new()?;
        let supported_interfaces = dbus.proxies().platform().supported_interfaces()?;
        let supported_properties = dbus.proxies().platform().supported_properties()?;

        // Set up an oscillator to run on a thread.
        // Helpful for visual effects like colour pulse.
        let oscillator1 = Arc::new(AtomicU8::new(0));
        let oscillator2 = Arc::new(AtomicU8::new(0));
        let oscillator3 = Arc::new(AtomicU8::new(0));

        let oscillator1_1 = oscillator1.clone();
        let oscillator1_2 = oscillator2.clone();
        let oscillator1_3 = oscillator3.clone();

        let oscillator_freq = Arc::new(AtomicU8::new(5));
        let oscillator_freq1 = oscillator_freq.clone();
        let oscillator_toggle = Arc::new(AtomicBool::new(false));
        let oscillator_toggle1 = oscillator_toggle.clone();

        std::thread::spawn(move || {
            let started = Instant::now();
            let mut toggled = false;
            loop {
                let time = started.elapsed();
                // 32 = slow, 16 = med, 8 = fast
                let scale = oscillator_freq1.load(Ordering::SeqCst) as f64;
                let elapsed1 = (time.as_millis() as f64 + 333.0) / 10000.0;
                let elapsed2 = (time.as_millis() as f64 + 666.0) / 10000.0;
                let elapsed3 = (time.as_millis() as f64 + 999.0) / 10000.0;
                let tmp1 = ((scale * elapsed1 * PI).cos()).abs();
                let tmp2 = ((scale * 0.85 * elapsed2 * PI).cos()).abs();
                let tmp3 = ((scale * 0.7 * elapsed3 * PI).cos()).abs();
                if tmp1 <= 0.1 && !toggled {
                    let s = oscillator_toggle1.load(Ordering::SeqCst);
                    oscillator_toggle1.store(!s, Ordering::SeqCst);
                    toggled = true;
                } else if tmp1 > 0.9 {
                    toggled = false;
                }

                let tmp1 = (255.0 * tmp1 * 100.0 / 255.0) as u8;
                let tmp2 = (255.0 * tmp2 * 100.0 / 255.0) as u8;
                let tmp3 = (255.0 * tmp3 * 100.0 / 255.0) as u8;

                oscillator1_1.store(tmp1, Ordering::SeqCst);
                oscillator1_2.store(tmp2, Ordering::SeqCst);
                oscillator1_3.store(tmp3, Ordering::SeqCst);

                std::thread::sleep(Duration::from_millis(33));
            }
        });

        Ok(Self {
            supported_interfaces,
            supported_properties,
            states,
            page: Page::System,
            config,
            oscillator1,
            oscillator2,
            oscillator3,
            oscillator_toggle,
            oscillator_freq,
        })
    }
}

impl eframe::App for RogApp {
    /// Called each time the UI needs repainting, which may be many times per
    /// second. Put your widgets into a `SidePanel`, `TopPanel`,
    /// `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        let states = self.states.clone();

        if let Ok(mut states) = states.try_lock() {
            if states.app_should_update {
                states.app_should_update = false;
                ctx.request_repaint();
            }
        }

        // Shortcut typical display stuff
        if let Ok(mut states) = states.try_lock() {
            let layout_testing = states.aura_creation.layout_testing.clone();
            if let Some(path) = &layout_testing {
                let modified = path.metadata().unwrap().modified().unwrap();
                if states.aura_creation.layout_last_modified < modified {
                    states.aura_creation.layout_last_modified = modified;
                    // time to reload the config
                    states.aura_creation.keyboard_layout = KeyLayout::from_file(path).unwrap();
                }
                self.aura_page(&mut states, ctx);
                return;
            }

            self.top_bar(&mut states, ctx, frame);
            self.side_panel(ctx);
        }
        let page = self.page;

        let mut was_error = false;

        if let Ok(mut states) = states.try_lock() {
            if let Some(err) = states.error.clone() {
                was_error = true;
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.heading(RichText::new("Error!").size(28.0));

                    ui.centered_and_justified(|ui| {
                        ui.label(RichText::new(format!("The error was: {:?}", err)).size(22.0));
                    });
                });
                egui::TopBottomPanel::bottom("error_bar")
                    .default_height(26.0)
                    .show(ctx, |ui| {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                            if ui
                                .add(Button::new(RichText::new("Okay").size(20.0)))
                                .clicked()
                            {
                                states.error = None;
                            }
                        });
                    });
            }
        }

        if !was_error {
            if let Ok(mut states) = states.try_lock() {
                match page {
                    Page::AppSettings => self.app_settings_page(&mut states, ctx),
                    Page::System => self.system_page(&mut states, ctx),
                    Page::AuraEffects => self.aura_page(&mut states, ctx),
                    Page::AnimeMatrix => todo!(),
                    Page::FanCurves => self.fan_curve_page(&mut states, ctx),
                };
            }
        }
    }
}
