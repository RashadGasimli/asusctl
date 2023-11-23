use egui::Ui;
use rog_platform::platform::{GpuMode, PlatformPolicy};

use crate::system_state::SystemState;

pub fn platform_profile(states: &mut SystemState, ui: &mut Ui) {
    if let Some(mut throt) = states.bios.throttle {
        ui.heading("Platform profile");

        let mut changed = false;
        let mut item = |p: PlatformPolicy, ui: &mut Ui| {
            if ui
                .selectable_value(&mut throt, p, format!("{p:?}"))
                .clicked()
            {
                changed = true;
            }
        };

        ui.horizontal_wrapped(|ui| {
            for a in PlatformPolicy::list() {
                item(a, ui);
            }
        });

        if changed {
            if let Some(throttle) = states.bios.throttle {
                states
                    .asus_dbus
                    .proxies()
                    .platform()
                    .set_throttle_thermal_policy(throttle)
                    .map_err(|err| {
                        states.error = Some(err.to_string());
                    })
                    .ok();
            }
        };
    }
}

pub fn rog_bios_group(states: &mut SystemState, ui: &mut Ui) {
    ui.heading("Bios options");

    if let Some(mut limit) = states.bios.charge_limit {
        let slider = egui::Slider::new(&mut limit, 20..=100)
            .text("Charging limit")
            .step_by(1.0);
        if ui.add(slider).drag_released() {
            states
                .asus_dbus
                .proxies()
                .platform()
                .set_charge_control_end_threshold(limit)
                .map_err(|err| {
                    states.error = Some(err.to_string());
                })
                .ok();
        }
    }

    if let Some(mut sound) = states.bios.post_sound {
        if ui
            .add(egui::Checkbox::new(&mut sound, "POST sound"))
            .changed()
        {
            states
                .asus_dbus
                .proxies()
                .platform()
                .set_post_animation_sound(sound)
                .map_err(|err| {
                    states.error = Some(err.to_string());
                })
                .ok();
        }
    }

    if let Some(mut overdrive) = states.bios.panel_overdrive {
        if ui
            .add(egui::Checkbox::new(&mut overdrive, "Panel overdrive"))
            .changed()
        {
            states
                .asus_dbus
                .proxies()
                .platform()
                .set_panel_od(overdrive)
                .map_err(|err| {
                    states.error = Some(err.to_string());
                })
                .ok();
        }
    }

    if let Some(mut mini_led_mode) = states.bios.mini_led_mode {
        if ui
            .add(egui::Checkbox::new(&mut mini_led_mode, "MiniLED backlight"))
            .changed()
        {
            states
                .asus_dbus
                .proxies()
                .platform()
                .set_mini_led_mode(mini_led_mode)
                .map_err(|err| {
                    states.error = Some(err.to_string());
                })
                .ok();
        }
    }

    if let Some(mut gpu_mux_mode) = states.bios.gpu_mux_mode {
        let mut changed = false;

        let mut reboot_required = false;
        if let Ok(mode) = states.asus_dbus.proxies().platform().gpu_mux_mode() {
            reboot_required = GpuMode::from(mode) != gpu_mux_mode;
        }

        ui.group(|ui| {
            ui.vertical(|ui| {
                ui.horizontal_wrapped(|ui| ui.label("GPU MUX mode"));
                ui.horizontal_wrapped(|ui| ui.label("NOTE: Value does not change until rebooted"));
                ui.horizontal_wrapped(|ui| {
                    changed = ui
                        .selectable_value(
                            &mut gpu_mux_mode,
                            GpuMode::Discrete,
                            "Dedicated (Ultimate)",
                        )
                        .clicked()
                        || ui
                            .selectable_value(
                                &mut gpu_mux_mode,
                                GpuMode::Optimus,
                                "Optimus (Hybrid)",
                            )
                            .clicked();
                });

                if reboot_required {
                    ui.horizontal_wrapped(|ui| ui.heading("REBOOT REQUIRED"));
                }
            });
        });

        if changed {
            states
                .asus_dbus
                .proxies()
                .platform()
                .set_gpu_mux_mode(gpu_mux_mode)
                .map_err(|err| {
                    states.error = Some(err.to_string());
                })
                .ok();
        }
    }
}
