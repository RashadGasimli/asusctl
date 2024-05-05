use std::convert::TryFrom;
use std::path::Path;
use std::process::Command;
use std::thread::sleep;

use anime_cli::{AnimeActions, AnimeCommand};
use argh::FromArgs;
use asusd::ctrl_fancurves::FAN_CURVE_ZBUS_NAME;
use aura_cli::{LedPowerCommand1, LedPowerCommand2};
use dmi_id::DMIID;
use fan_curve_cli::FanCurveCommand;
use rog_anime::usb::get_anime_type;
use rog_anime::{AnimTime, AnimeDataBuffer, AnimeDiagonal, AnimeGif, AnimeImage, AnimeType, Vec2};
use rog_aura::keyboard::{AuraPowerState, LaptopAuraPower};
use rog_aura::{self, AuraEffect, PowerZones};
use rog_dbus::zbus_anime::AnimeProxyBlocking;
use rog_dbus::zbus_aura::AuraProxyBlocking;
use rog_dbus::zbus_fan_curves::FanCurvesProxyBlocking;
use rog_dbus::zbus_platform::PlatformProxyBlocking;
use rog_dbus::zbus_slash::SlashProxyBlocking;
use rog_platform::platform::{GpuMode, Properties, ThrottlePolicy};
use rog_profiles::error::ProfileError;
use rog_slash::SlashMode;
use zbus::blocking::Connection;

use crate::aura_cli::AuraPowerStates;
use crate::cli_opts::*;
use crate::slash_cli::SlashCommand;

mod anime_cli;
mod aura_cli;
mod cli_opts;
mod fan_curve_cli;
mod slash_cli;

fn main() {
    let parsed: CliStart = argh::from_env();

    let conn = Connection::system().unwrap();
    if let Ok(platform_proxy) = PlatformProxyBlocking::new(&conn).map_err(|e| {
        check_service("asusd");
        println!("\nError: {e}\n");
        print_info();
    }) {
        let self_version = env!("CARGO_PKG_VERSION");
        let asusd_version = platform_proxy.version().unwrap();
        if asusd_version != self_version {
            println!("Version mismatch: asusctl = {self_version}, asusd = {asusd_version}");
            return;
        }

        let supported_properties = platform_proxy.supported_properties().unwrap();
        let supported_interfaces = platform_proxy.supported_interfaces().unwrap();

        if parsed.version {
            println!("asusctl v{}", env!("CARGO_PKG_VERSION"));
            println!();
            print_info();
        }

        if let Err(err) = do_parsed(&parsed, &supported_interfaces, &supported_properties, conn) {
            print_error_help(&*err, &supported_interfaces, &supported_properties);
        }
    }
}

fn print_error_help(
    err: &dyn std::error::Error,
    supported_interfaces: &[String],
    supported_properties: &[Properties],
) {
    check_service("asusd");
    println!("\nError: {}\n", err);
    print_info();
    println!();
    println!("Supported interfaces:\n\n{:#?}\n", supported_interfaces);
    println!("Supported properties:\n\n{:#?}\n", supported_properties);
}

fn print_info() {
    let dmi = DMIID::new().unwrap_or_default();
    let board_name = dmi.board_name;
    let prod_family = dmi.product_family;
    println!("asusctl version: {}", env!("CARGO_PKG_VERSION"));
    println!(" Product family: {}", prod_family.trim());
    println!("     Board name: {}", board_name.trim());
}

fn check_service(name: &str) -> bool {
    if name != "asusd" && !check_systemd_unit_enabled(name) {
        println!(
            "\n\x1b[0;31m{} is not enabled, enable it with `systemctl enable {}\x1b[0m",
            name, name
        );
        return true;
    } else if !check_systemd_unit_active(name) {
        println!(
            "\n\x1b[0;31m{} is not running, start it with `systemctl start {}\x1b[0m",
            name, name
        );
        return true;
    }
    false
}

fn find_aura_iface() -> Result<Vec<AuraProxyBlocking<'static>>, Box<dyn std::error::Error>> {
    let conn = zbus::blocking::Connection::system().unwrap();
    let f = zbus::blocking::fdo::ObjectManagerProxy::new(&conn, "org.asuslinux.Daemon", "/org")
        .unwrap();
    let interfaces = f.get_managed_objects().unwrap();
    let mut aura_paths = Vec::new();
    for v in interfaces.iter() {
        // let o: Vec<zbus::names::OwnedInterfaceName> = v.1.keys().map(|e|
        // e.to_owned()).collect(); println!("{}, {:?}", v.0, o);
        for k in v.1.keys() {
            if k.as_str() == "org.asuslinux.Aura" {
                println!("Found aura device at {}, {}", v.0, k);
                aura_paths.push(v.0.clone());
            }
        }
    }
    if aura_paths.len() > 1 {
        println!("Multiple aura devices found: {aura_paths:?}");
        println!("TODO: enable selection");
    }
    if !aura_paths.is_empty() {
        let mut ctrl = Vec::new();
        for path in aura_paths {
            ctrl.push(
                AuraProxyBlocking::builder(&conn)
                    .path(path.clone())?
                    .destination("org.asuslinux.Daemon")?
                    .build()?,
            );
        }
        return Ok(ctrl);
    }

    Err("No Aura interface".into())
}

fn do_parsed(
    parsed: &CliStart,
    supported_interfaces: &[String],
    supported_properties: &[Properties],
    conn: Connection,
) -> Result<(), Box<dyn std::error::Error>> {
    match &parsed.command {
        Some(CliCommand::LedMode(mode)) => handle_led_mode(&find_aura_iface()?, mode)?,
        Some(CliCommand::LedPow1(pow)) => handle_led_power1(&find_aura_iface()?, pow)?,
        Some(CliCommand::LedPow2(pow)) => handle_led_power2(&find_aura_iface()?, pow)?,
        Some(CliCommand::Profile(cmd)) => {
            handle_throttle_profile(&conn, supported_properties, cmd)?
        }
        Some(CliCommand::FanCurve(cmd)) => {
            handle_fan_curve(&conn, supported_interfaces, cmd)?;
        }
        Some(CliCommand::Graphics(_)) => do_gfx(),
        Some(CliCommand::Anime(cmd)) => handle_anime(&conn, cmd)?,
        Some(CliCommand::Slash(cmd)) => handle_slash(&conn, cmd)?,
        Some(CliCommand::Bios(cmd)) => {
            handle_platform_properties(&conn, supported_properties, cmd)?
        }
        None => {
            if !parsed.show_supported
                && parsed.kbd_bright.is_none()
                && parsed.chg_limit.is_none()
                && !parsed.next_kbd_bright
                && !parsed.prev_kbd_bright
            {
                println!("\nExtra help can be requested on any command or subcommand:");
                println!(" asusctl led-mode --help");
                println!(" asusctl led-mode static --help");
            }
        }
    }

    if let Some(brightness) = &parsed.kbd_bright {
        if let Ok(aura) = find_aura_iface() {
            for aura in aura.iter() {
                match brightness.level() {
                    None => {
                        let level = aura.brightness()?;
                        println!("Current keyboard led brightness: {level:?}");
                    }
                    Some(level) => aura.set_brightness(rog_aura::LedBrightness::from(level))?,
                }
            }
        } else {
            println!("No aura interface found");
        }
    }

    if parsed.next_kbd_bright {
        if let Ok(aura) = find_aura_iface() {
            for aura in aura.iter() {
                let brightness = aura.brightness()?;
                aura.set_brightness(brightness.next())?;
            }
        } else {
            println!("No aura interface found");
        }
    }

    if parsed.prev_kbd_bright {
        if let Ok(aura) = find_aura_iface() {
            for aura in aura.iter() {
                let brightness = aura.brightness()?;
                aura.set_brightness(brightness.prev())?;
            }
        } else {
            println!("No aura interface found");
        }
    }

    if parsed.show_supported {
        println!("Supported Core Functions:\n{:#?}", supported_interfaces);
        println!(
            "Supported Platform Properties:\n{:#?}",
            supported_properties
        );
        if let Ok(aura) = find_aura_iface() {
            // TODO: multiple RGB check
            let bright = aura.first().unwrap().supported_brightness()?;
            let modes = aura.first().unwrap().supported_basic_modes()?;
            let zones = aura.first().unwrap().supported_basic_zones()?;
            let power = aura.first().unwrap().supported_power_zones()?;
            println!("Supported Keyboard Brightness:\n{:#?}", bright);
            println!("Supported Aura Modes:\n{:#?}", modes);
            println!("Supported Aura Zones:\n{:#?}", zones);
            println!("Supported Aura Power Zones:\n{:#?}", power);
        } else {
            println!("No aura interface found");
        }
    }

    if let Some(chg_limit) = parsed.chg_limit {
        let proxy = PlatformProxyBlocking::new(&conn)?;
        proxy.set_charge_control_end_threshold(chg_limit)?;
    }

    Ok(())
}

fn do_gfx() {
    println!(
        "Please use supergfxctl for graphics switching. supergfxctl is the result of making \
         asusctl graphics switching generic so all laptops can use it"
    );
    println!("This command will be removed in future");
}

fn handle_anime(conn: &Connection, cmd: &AnimeCommand) -> Result<(), Box<dyn std::error::Error>> {
    let proxy = AnimeProxyBlocking::new(conn)?;
    if let Some(enable) = cmd.enable_display {
        proxy.set_enable_display(enable)?;
    }
    if let Some(enable) = cmd.enable_powersave_anim {
        proxy.set_builtins_enabled(enable)?;
    }
    if let Some(bright) = cmd.brightness {
        proxy.set_brightness(bright)?;
    }
    if let Some(enable) = cmd.off_when_lid_closed {
        proxy.set_off_when_lid_closed(enable)?;
    }
    if let Some(enable) = cmd.off_when_suspended {
        proxy.set_off_when_suspended(enable)?;
    }
    if let Some(enable) = cmd.off_when_unplugged {
        proxy.set_off_when_unplugged(enable)?;
    }
    if cmd.off_with_his_head.is_some() {
        println!("Did Alice _really_ make it back from Wonderland?");
    }

    let mut anime_type = get_anime_type()?;
    if let AnimeType::Unknown = anime_type {
        if let Some(model) = cmd.override_type {
            anime_type = model;
        }
    }

    if cmd.clear {
        let data = vec![255u8; anime_type.data_length()];
        let tmp = AnimeDataBuffer::from_vec(anime_type, data)?;
        proxy.write(tmp)?;
    }

    if let Some(action) = cmd.command.as_ref() {
        match action {
            AnimeActions::Image(image) => {
                verify_brightness(image.bright);

                let matrix = AnimeImage::from_png(
                    Path::new(&image.path),
                    image.scale,
                    image.angle,
                    Vec2::new(image.x_pos, image.y_pos),
                    image.bright,
                    anime_type,
                )?;

                proxy.write(<AnimeDataBuffer>::try_from(&matrix)?)?;
            }
            AnimeActions::PixelImage(image) => {
                verify_brightness(image.bright);

                let matrix = AnimeDiagonal::from_png(
                    Path::new(&image.path),
                    None,
                    image.bright,
                    anime_type,
                )?;

                proxy.write(matrix.into_data_buffer(anime_type)?)?;
            }
            AnimeActions::Gif(gif) => {
                verify_brightness(gif.bright);

                let matrix = AnimeGif::from_gif(
                    Path::new(&gif.path),
                    gif.scale,
                    gif.angle,
                    Vec2::new(gif.x_pos, gif.y_pos),
                    AnimTime::Count(1),
                    gif.bright,
                    anime_type,
                )?;

                let mut loops = gif.loops as i32;
                loop {
                    for frame in matrix.frames() {
                        proxy.write(frame.frame().clone())?;
                        sleep(frame.delay());
                    }
                    if loops >= 0 {
                        loops -= 1;
                    }
                    if loops == 0 {
                        break;
                    }
                }
            }
            AnimeActions::PixelGif(gif) => {
                verify_brightness(gif.bright);

                let matrix = AnimeGif::from_diagonal_gif(
                    Path::new(&gif.path),
                    AnimTime::Count(1),
                    gif.bright,
                    anime_type,
                )?;

                let mut loops = gif.loops as i32;
                loop {
                    for frame in matrix.frames() {
                        proxy.write(frame.frame().clone())?;
                        sleep(frame.delay());
                    }
                    if loops >= 0 {
                        loops -= 1;
                    }
                    if loops == 0 {
                        break;
                    }
                }
            }
            AnimeActions::SetBuiltins(builtins) => {
                proxy.set_builtin_animations(rog_anime::Animations {
                    boot: builtins.boot,
                    awake: builtins.awake,
                    sleep: builtins.sleep,
                    shutdown: builtins.shutdown,
                })?;
            }
        }
    }
    Ok(())
}

fn verify_brightness(brightness: f32) {
    if !(0.0..=1.0).contains(&brightness) {
        println!(
            "Image and global brightness must be between 0.0 and 1.0 (inclusive), was {}",
            brightness
        );
    }
}

fn handle_slash(conn: &Connection, cmd: &SlashCommand) -> Result<(), Box<dyn std::error::Error>> {
    let proxy = SlashProxyBlocking::new(conn)?;
    if cmd.enable {
        proxy.set_enabled(true)?;
    }
    if cmd.disable {
        proxy.set_enabled(false)?;
    }
    if let Some(brightness) = cmd.brightness {
        proxy.set_brightness(brightness)?;
    }
    if let Some(interval) = cmd.interval {
        proxy.set_interval(interval)?;
    }
    if let Some(slash_mode) = cmd.slash_mode {
        proxy.set_slash_mode(slash_mode)?;
    }
    if cmd.list {
        let res = SlashMode::list();
        for p in &res {
            println!("{:?}", p);
        }
    }

    Ok(())
}

fn handle_led_mode(
    aura: &[AuraProxyBlocking],
    mode: &LedModeCommand,
) -> Result<(), Box<dyn std::error::Error>> {
    if mode.next_mode && mode.prev_mode {
        println!("Please specify either next or previous");
        return Ok(());
    }
    if mode.next_mode {
        for aura in aura {
            let mode = aura.led_mode()?;
            let modes = aura.supported_basic_modes()?;
            let mut pos = modes.iter().position(|m| *m == mode).unwrap() + 1;
            if pos >= modes.len() {
                pos = 0;
            }
            aura.set_led_mode(modes[pos])?;
        }
    } else if mode.prev_mode {
        for aura in aura {
            let mode = aura.led_mode()?;
            let modes = aura.supported_basic_modes()?;
            let mut pos = modes.iter().position(|m| *m == mode).unwrap();
            if pos == 0 {
                pos = modes.len() - 1;
            } else {
                pos -= 1;
            }
            aura.set_led_mode(modes[pos])?;
        }
    } else if let Some(mode) = mode.command.as_ref() {
        for aura in aura {
            aura.set_led_mode_data(<AuraEffect>::from(mode))?;
        }
    }

    Ok(())
}

fn handle_led_power1(
    aura: &[AuraProxyBlocking],
    power: &LedPowerCommand1,
) -> Result<(), Box<dyn std::error::Error>> {
    for aura in aura {
        let dev_type = aura.device_type()?;
        if !dev_type.is_old_laptop() && !dev_type.is_tuf_laptop() {
            println!("This option applies only to keyboards 2021+");
        }

        if dev_type.is_old_laptop() || dev_type.is_tuf_laptop() {
            handle_led_power_1_do_1866(aura, power)?;
            return Ok(());
        }
    }

    println!("These options are for keyboards of product ID 0x1866 or TUF only");
    Ok(())
}

fn handle_led_power_1_do_1866(
    aura: &AuraProxyBlocking,
    power: &LedPowerCommand1,
) -> Result<(), Box<dyn std::error::Error>> {
    let zone = if power.keyboard && power.lightbar {
        PowerZones::KeyboardAndLightbar
    } else if power.lightbar {
        PowerZones::Lightbar
    } else {
        PowerZones::Keyboard
    };
    let states = LaptopAuraPower {
        states: vec![AuraPowerState {
            zone,
            boot: power.boot.unwrap_or_default(),
            awake: power.awake.unwrap_or_default(),
            sleep: power.sleep.unwrap_or_default(),
            shutdown: false,
        }],
    };

    aura.set_led_power(states)?;

    Ok(())
}

fn handle_led_power2(
    aura: &[AuraProxyBlocking],
    power: &LedPowerCommand2,
) -> Result<(), Box<dyn std::error::Error>> {
    for aura in aura {
        let dev_type = aura.device_type()?;
        if !dev_type.is_new_laptop() {
            println!("This option applies only to keyboards 2021+");
            continue;
        }
        if let Some(pow) = power.command.as_ref() {
            let mut states = aura.led_power()?;
            let mut set = |zone: PowerZones, set_to: &AuraPowerStates| {
                for state in states.states.iter_mut() {
                    if state.zone == zone {
                        state.boot = set_to.boot;
                        state.awake = set_to.awake;
                        state.sleep = set_to.sleep;
                        state.shutdown = set_to.shutdown;
                        break;
                    }
                }
            };

            if let Some(cmd) = &power.command {
                match cmd {
                    aura_cli::SetAuraZoneEnabled::Keyboard(k) => set(PowerZones::Keyboard, k),
                    aura_cli::SetAuraZoneEnabled::Logo(l) => set(PowerZones::Logo, l),
                    aura_cli::SetAuraZoneEnabled::Lightbar(l) => set(PowerZones::Lightbar, l),
                    aura_cli::SetAuraZoneEnabled::Lid(l) => set(PowerZones::Lid, l),
                    aura_cli::SetAuraZoneEnabled::RearGlow(r) => set(PowerZones::RearGlow, r),
                }
            }

            aura.set_led_power(states)?;
        }
    }

    Ok(())
}

fn handle_throttle_profile(
    conn: &Connection,
    supported: &[Properties],
    cmd: &ProfileCommand,
) -> Result<(), Box<dyn std::error::Error>> {
    if !supported.contains(&Properties::ThrottlePolicy) {
        println!("Profiles not supported by either this kernel or by the laptop.");
        return Err(ProfileError::NotSupported.into());
    }

    let proxy = PlatformProxyBlocking::new(conn)?;
    let current = proxy.throttle_thermal_policy()?;

    if cmd.next {
        proxy.set_throttle_thermal_policy(current.next())?;
    } else if let Some(profile) = cmd.profile_set {
        proxy.set_throttle_thermal_policy(profile)?;
    }

    if cmd.list {
        let res = ThrottlePolicy::list();
        for p in &res {
            println!("{:?}", p);
        }
    }

    if cmd.profile_get {
        println!("Active profile is {current:?}");
    }

    Ok(())
}

fn handle_fan_curve(
    conn: &Connection,
    supported: &[String],
    cmd: &FanCurveCommand,
) -> Result<(), Box<dyn std::error::Error>> {
    if !supported.contains(&FAN_CURVE_ZBUS_NAME.to_string()) {
        println!("Fan-curves not supported by either this kernel or by the laptop.");
        return Err(ProfileError::NotSupported.into());
    }

    if (cmd.enable_fan_curves.is_some() || cmd.fan.is_some() || cmd.data.is_some())
        && cmd.mod_profile.is_none()
    {
        println!(
            "--enable-fan-curves, --enable-fan-curve, --fan, and --data options require \
             --mod-profile"
        );
        return Ok(());
    }

    let plat_proxy = PlatformProxyBlocking::new(conn)?;
    let fan_proxy = FanCurvesProxyBlocking::new(conn)?;
    if cmd.get_enabled {
        let profile = plat_proxy.throttle_thermal_policy()?;
        let curves = fan_proxy.fan_curve_data(profile)?;
        for curve in curves.iter() {
            println!("{}", String::from(curve));
        }
    }

    if cmd.default {
        let active = plat_proxy.throttle_thermal_policy()?;
        fan_proxy.set_curves_to_defaults(active)?;
    }

    if let Some(profile) = cmd.mod_profile {
        if cmd.enable_fan_curves.is_none() && cmd.data.is_none() {
            let data = fan_proxy.fan_curve_data(profile)?;
            let data = toml::to_string(&data)?;
            println!("\nFan curves for {:?}\n\n{}", profile, data);
        }

        if let Some(enabled) = cmd.enable_fan_curves {
            fan_proxy.set_fan_curves_enabled(profile, enabled)?;
        }

        if let Some(enabled) = cmd.enable_fan_curve {
            if let Some(fan) = cmd.fan {
                fan_proxy.set_profile_fan_curve_enabled(profile, fan, enabled)?;
            } else {
                println!(
                    "--enable-fan-curves, --enable-fan-curve, --fan, and --data options require \
                     --mod-profile"
                );
            }
        }

        if let Some(mut curve) = cmd.data.clone() {
            let fan = cmd.fan.unwrap_or_default();
            curve.set_fan(fan);
            fan_proxy.set_fan_curve(profile, curve)?;
        }
    }

    Ok(())
}

fn handle_platform_properties(
    conn: &Connection,
    supported: &[Properties],
    cmd: &SysCommand,
) -> Result<(), Box<dyn std::error::Error>> {
    {
        let proxy = PlatformProxyBlocking::new(conn)?;

        if let Some(opt) = cmd.post_sound_set {
            proxy.set_boot_sound(opt)?;
        }
        if cmd.post_sound_get {
            let res = proxy.boot_sound()?;
            println!("Bios POST sound on: {}", res);
        }

        if let Some(opt) = cmd.gpu_mux_mode_set {
            println!("Rebuilding initrd to include drivers");
            proxy.set_gpu_mux_mode(GpuMode::from_mux(opt))?;
            println!(
                "The mode change is not active until you reboot, on boot the bios will make the \
                 required change"
            );
        }
        if cmd.gpu_mux_mode_get {
            let res = proxy.gpu_mux_mode()?;
            println!("Bios GPU MUX: {:?}", res);
        }

        if let Some(opt) = cmd.panel_overdrive_set {
            proxy.set_panel_od(opt)?;
        }
        if cmd.panel_overdrive_get {
            let res = proxy.panel_od()?;
            println!("Panel overdrive on: {}", res);
        }
    }
    Ok(())
}

fn check_systemd_unit_active(name: &str) -> bool {
    if let Ok(out) = Command::new("systemctl")
        .arg("is-active")
        .arg(name)
        .output()
    {
        let buf = String::from_utf8_lossy(&out.stdout);
        return !buf.contains("inactive") && !buf.contains("failed");
    }
    false
}

fn check_systemd_unit_enabled(name: &str) -> bool {
    if let Ok(out) = Command::new("systemctl")
        .arg("is-enabled")
        .arg(name)
        .output()
    {
        let buf = String::from_utf8_lossy(&out.stdout);
        return buf.contains("enabled");
    }
    false
}
