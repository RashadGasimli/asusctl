use crate::{page_states::PageDataStates};
use egui::Ui;
use rog_dbus::RogDbusClient;
use rog_platform::{platform::GpuMode, supported::SupportedFunctions};
use rog_profiles::Profile;

pub async fn platform_profile(states: &mut PageDataStates, dbus: &RogDbusClient<'static>, ui: &mut Ui) {
    ui.heading("Platform profile");

    let mut changed = false;
    let mut item = |p: Profile, ui: &mut Ui| {
        if ui
            .selectable_value(&mut states.profiles.current, p, format!("{p:?}"))
            .clicked()
        {
            changed = true;
        }
    };

    ui.horizontal_wrapped(|ui| {
        for a in states.profiles.list.iter() {
            item(*a, ui);
        }
    });

    if changed {
        dbus.proxies()
            .profile()
            .set_active_profile(states.profiles.current).await
            .map_err(|err| {
                states.error = Some(err.to_string());
            })
            .ok();
    };
}

pub async fn rog_bios_group(
    supported: &SupportedFunctions,
    states: &mut PageDataStates,
    dbus: &mut RogDbusClient<'static>,
    ui: &mut Ui,
) {
    ui.heading("Bios options");

    let slider = egui::Slider::new(&mut states.charge_limit, 20..=100)
        .text("Charging limit")
        .step_by(1.0);
    if ui.add(slider).drag_released() {
        dbus.proxies()
            .charge()
            .set_charge_control_end_threshold(states.charge_limit as u8).await
            .map_err(|err| {
                states.error = Some(err.to_string());
            })
            .ok();
    }

    if supported.rog_bios_ctrl.post_sound
        && ui
            .add(egui::Checkbox::new(
                &mut states.bios.post_sound,
                "POST sound",
            ))
            .changed()
    {
        dbus.proxies()
            .rog_bios()
            .set_post_boot_sound(states.bios.post_sound).await
            .map_err(|err| {
                states.error = Some(err.to_string());
            })
            .ok();
    }

    if supported.rog_bios_ctrl.post_sound
        && ui
            .add(egui::Checkbox::new(
                &mut states.bios.panel_overdrive,
                "Panel overdrive",
            ))
            .changed()
    {
        dbus.proxies()
            .rog_bios()
            .set_panel_od(states.bios.panel_overdrive).await
            .map_err(|err| {
                states.error = Some(err.to_string());
            })
            .ok();
    }

    if supported.rog_bios_ctrl.gpu_mux {
        let mut changed = false;
        ui.group(|ui| {
            ui.vertical(|ui| {
                ui.horizontal_wrapped(|ui| ui.label("GPU MUX mode"));
                ui.horizontal_wrapped(|ui| ui.label("NOTE: Value does not change until rebooted"));
                ui.horizontal_wrapped(|ui| {
                    changed = ui
                        .selectable_value(
                            &mut states.bios.dedicated_gfx,
                            GpuMode::Discrete,
                            "Dedicated (Ultimate)",
                        )
                        .clicked()
                        || ui
                            .selectable_value(
                                &mut states.bios.dedicated_gfx,
                                GpuMode::Optimus,
                                "Optimus (Hybrid)",
                            )
                            .clicked();
                });
            });
        });

        if changed {
            dbus.proxies()
                .rog_bios()
                .set_gpu_mux_mode(states.bios.dedicated_gfx).await
                .map_err(|err| {
                    states.error = Some(err.to_string());
                })
                .ok();
        }
    }
}
