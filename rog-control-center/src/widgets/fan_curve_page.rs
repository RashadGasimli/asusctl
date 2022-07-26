use crate::{
    page_states::{FanCurvesState, ProfilesState},
    RogApp,
};
use egui::{plot::Points, Ui};
use rog_dbus::RogDbusClientBlocking;
use rog_profiles::{FanCurvePU, Profile};
use rog_supported::SupportedFunctions;

impl<'a> RogApp<'a> {
    pub fn fan_curve_page(&mut self, ctx: &egui::Context) {
        let Self {
            supported,
            states,
            asus_dbus: dbus,
            ..
        } = self;

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Custom fan curves");
            ui.label("A fan curve is only active when the related profile is active and the curve is enabled");
            Self::fan_curve(
                supported,
                &mut states.profiles,
                &mut states.fan_curves,
                dbus, &mut states.error,
                ui,
            );

            Self::fan_graphs(&mut states.profiles, &mut states.fan_curves, dbus, &mut states.error, ui);
        });
    }

    fn fan_curve(
        supported: &SupportedFunctions,
        profiles: &mut ProfilesState,
        curves: &mut FanCurvesState,
        dbus: &RogDbusClientBlocking,
        do_error: &mut Option<String>,
        ui: &mut Ui,
    ) {
        ui.separator();
        ui.label("Enabled fan-curves");

        let mut changed = false;
        ui.horizontal(|ui| {
            let mut item = |p: Profile, _curves: &mut FanCurvesState, mut checked: bool| {
                if ui
                    .add(egui::Checkbox::new(&mut checked, format!("{:?}", p)))
                    .changed()
                {
                    dbus.proxies()
                        .profile()
                        .set_fan_curve_enabled(p, checked)
                        .map_err(|err| {
                            *do_error = Some(err.to_string());
                        })
                        .ok();

                    #[cfg(feature = "mocking")]
                    if !checked {
                        _curves.enabled.remove(&p);
                    } else {
                        _curves.enabled.insert(p);
                    }
                    changed = true;
                }
            };

            for f in profiles.list.iter() {
                item(*f, curves, curves.enabled.contains(f));
            }
        });

        if changed {
            // Need to update app data if change made
            #[cfg(not(feature = "mocking"))]
            {
                let notif = curves.was_notified.clone();
                match FanCurvesState::new(notif, supported, dbus) {
                    Ok(f) => *curves = f,
                    Err(e) => *do_error = Some(e.to_string()),
                }
            }
        }
    }

    fn fan_graphs(
        profiles: &mut ProfilesState,
        curves: &mut FanCurvesState,
        dbus: &RogDbusClientBlocking,
        do_error: &mut Option<String>,
        ui: &mut Ui,
    ) {
        ui.separator();

        let mut item = |p: Profile, ui: &mut Ui| {
            ui.selectable_value(&mut curves.show_curve, p, format!("{p:?}"));
        };

        ui.horizontal_wrapped(|ui| {
            for a in curves.curves.iter() {
                item(*a.0, ui);
            }

            ui.selectable_value(
                &mut curves.show_graph,
                FanCurvePU::CPU,
                format!("{:?}", FanCurvePU::CPU),
            );
            ui.selectable_value(
                &mut curves.show_graph,
                FanCurvePU::GPU,
                format!("{:?}", FanCurvePU::GPU),
            );
        });

        let curve = curves.curves.get_mut(&curves.show_curve).unwrap();

        use egui::plot::{Line, Plot, PlotPoints};

        let data = if curves.show_graph == FanCurvePU::CPU {
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

        Plot::new("my_plot")
            .view_aspect(2.0)
            // .center_x_axis(true)
            // .center_y_axis(true)
            .include_x(0.0)
            .include_x(110.0)
            .include_y(0.0)
            .include_y(110.0)
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

        ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
            if ui.add(egui::Button::new("Apply Fan-curve")).clicked() {
                #[cfg(not(feature = "mocking"))]
                dbus.proxies()
                    .profile()
                    .set_fan_curve(profiles.current, data.clone())
                    .map_err(|err| {
                        *do_error = Some(err.to_string());
                    })
                    .ok();
                #[cfg(feature = "mocking")]
                dbg!("Applied");
            }
        });
    }
}
