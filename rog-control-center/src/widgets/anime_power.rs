use egui::{RichText, Ui};
use rog_dbus::RogDbusClient;
use rog_platform::supported::SupportedFunctions;

use crate::page_states::PageDataStates;

pub async fn anime_power_group(
    _supported: &SupportedFunctions,
    states: &mut PageDataStates,
    dbus: &mut RogDbusClient<'static>,
    ui: &mut Ui,
) {
    ui.heading("AniMe Matrix Settings");
    ui.label("Options are incomplete. Awake + Boot should work");

    let mut changed = false;
    let mut anime_boot_changed = false;
    let mut anime_awake_changed = false;

    ui.horizontal_wrapped(|ui| {
        ui.vertical(|ui| {
            let h = 16.0;
            ui.set_row_height(22.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("Brightness").size(h));
            });
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("Boot").size(h));
            });
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("Awake").size(h));
            });
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("Sleep").size(h));
            });
        });
        ui.vertical(|ui| {
            ui.set_row_height(22.0);
            ui.horizontal_wrapped(|ui| {
                if ui
                    .add(egui::Slider::new(&mut states.anime.bright, 0..=254))
                    .changed()
                {
                    changed = true;
                }
            });

            ui.horizontal_wrapped(|ui| {
                anime_boot_changed = ui.checkbox(&mut states.anime.boot, "Enable").changed();
            });
            ui.horizontal_wrapped(|ui| {
                anime_awake_changed = ui.checkbox(&mut states.anime.awake, "Enable").changed();
            });
            ui.horizontal_wrapped(|ui| {
                if ui.checkbox(&mut states.anime.sleep, "Enable").changed() {
                    changed = true;
                }
            });
        });
    });

    if anime_boot_changed {
        dbus.proxies()
            .anime()
            .set_boot_on_off(states.anime.boot)
            .await
            .map_err(|err| {
                states.error = Some(err.to_string());
            })
            .ok();
    }
    if anime_awake_changed {
        dbus.proxies()
            .anime()
            .set_on_off(states.anime.awake)
            .await
            .map_err(|err| {
                states.error = Some(err.to_string());
            })
            .ok();
    }
}
