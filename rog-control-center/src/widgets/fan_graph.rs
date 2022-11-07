use egui::{plot::Points, Ui};
use rog_dbus::RogDbusClient;
use rog_platform::supported::SupportedFunctions;
use rog_profiles::{FanCurvePU, Profile};

use crate::page_states::{FanCurvesState, PageDataStates};

pub async fn fan_graphs(
    supported: &SupportedFunctions,
    states: &mut PageDataStates,
    dbus: &RogDbusClient<'static>,
    ui: &mut Ui,
) {
    ui.separator();

    let mut item = |p: Profile, ui: &mut Ui| {
        ui.group(|ui| {
            ui.selectable_value(&mut states.fan_curves.show_curve, p, format!("{p:?}"));
            ui.add_enabled_ui(states.fan_curves.show_curve == p, |ui| {
                ui.selectable_value(
                    &mut states.fan_curves.show_graph,
                    FanCurvePU::CPU,
                    format!("{:?}", FanCurvePU::CPU),
                );
                ui.selectable_value(
                    &mut states.fan_curves.show_graph,
                    FanCurvePU::GPU,
                    format!("{:?}", FanCurvePU::GPU),
                );
            });
        });
    };

    ui.horizontal_wrapped(|ui| {
        for a in states.fan_curves.curves.iter() {
            item(*a.0, ui);
        }
    });

    let curve = states
        .fan_curves
        .curves
        .get_mut(&states.fan_curves.show_curve)
        .unwrap();

    use egui::plot::{Line, Plot, PlotPoints};

    let data = if states.fan_curves.show_graph == FanCurvePU::CPU {
        &mut curve.cpu
    } else {
        &mut curve.gpu
    };

    let points = data.temp.iter().enumerate().map(|(idx, x)| {
        let x = *x as f64;
        let y = ((data.pwm[idx] as u32) * 100 / 255) as f64;
        [x, y]
    });

    let line = Line::new(PlotPoints::from_iter(points.clone())).width(2.0);
    let points = Points::new(PlotPoints::from_iter(points)).radius(3.0);

    Plot::new("fan_curves")
        .view_aspect(1.666)
        // .center_x_axis(true)
        // .center_y_axis(true)
        .include_x(0.0)
        .include_x(104.0)
        .include_y(0.0)
        .include_y(106.0)
        .allow_scroll(false)
        .allow_drag(false)
        .allow_boxed_zoom(false)
        .x_axis_formatter(|d, _r| format!("{}", d))
        .y_axis_formatter(|d, _r| format!("{:.*}%", 1, d))
        .label_formatter(|name, value| {
            if !name.is_empty() {
                format!("{}: {:.*}%", name, 1, value.y)
            } else {
                format!("Temp {}c\nFan {:.*}%", value.x as u8, 1, value.y)
            }
        })
        .show(ui, |plot_ui| {
            if plot_ui.plot_hovered() {
                let mut idx = 0;

                if let Some(point) = plot_ui.pointer_coordinate() {
                    let mut x: i32 = 255;
                    for (i, n) in data.temp.iter().enumerate() {
                        let tmp = x.min((point.x as i32 - *n as i32).abs());
                        if tmp < x {
                            x = tmp;
                            idx = i;
                        }
                    }

                    if plot_ui.plot_clicked() {
                        data.temp[idx] = point.x as u8;
                        data.pwm[idx] = (point.y * 255.0 / 100.0) as u8;
                    } else {
                        let drag = plot_ui.pointer_coordinate_drag_delta();
                        if drag.length_sq() != 0.0 {
                            data.temp[idx] = (point.x as f32 + drag.x) as u8;
                            data.pwm[idx] = ((point.y as f32 + drag.y) * 255.0 / 100.0) as u8;
                        }
                    }
                }
            }
            plot_ui.line(line);
            plot_ui.points(points)
        });

    let mut set = false;
    let mut reset = false;
    ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
        set = ui.add(egui::Button::new("Apply Fan-curve")).clicked();
        reset = ui.add(egui::Button::new("Reset Profile")).clicked();
    });

    if set {
        dbus.proxies()
            .profile()
            .set_fan_curve(states.profiles.current, data.clone())
            .await
            .map_err(|err| {
                states.error = Some(err.to_string());
            })
            .ok();
    }

    if reset {
        dbus.proxies()
            .profile()
            .reset_profile_curves(states.profiles.current)
            .await
            .map_err(|err| {
                states.error = Some(err.to_string());
            })
            .ok();

        let notif = states.fan_curves.was_notified.clone();
        match FanCurvesState::new(notif, supported, dbus).await {
            Ok(f) => states.fan_curves = f,
            Err(e) => states.error = Some(e.to_string()),
        }
    }
}
