use std::time::Duration;

use config_traits::{StdConfig, StdConfigLoad2};
use nanoserde::{DeRon, SerRon};
use rog_anime::error::AnimeError;
use rog_anime::usb::Brightness;
use rog_anime::{
    ActionData, ActionLoader, AnimTime, Animations, AnimeType, DeviceState, Fade, MyVec2,
};

const CONFIG_FILE: &str = "anime.ron";

#[derive(DeRon, SerRon)]
pub struct AnimeConfigV460 {
    pub system: Vec<ActionLoader>,
    pub boot: Vec<ActionLoader>,
    pub wake: Vec<ActionLoader>,
    pub sleep: Vec<ActionLoader>,
    pub shutdown: Vec<ActionLoader>,
    pub brightness: f32,
}

impl From<AnimeConfigV460> for AnimeConfig {
    fn from(c: AnimeConfigV460) -> AnimeConfig {
        AnimeConfig {
            system: c.system,
            boot: c.boot,
            wake: c.wake,
            shutdown: c.shutdown,
            ..Default::default()
        }
    }
}

#[derive(DeRon, SerRon, Debug)]
pub struct AnimeConfigV472 {
    pub model_override: Option<AnimeType>,
    pub system: Vec<ActionLoader>,
    pub boot: Vec<ActionLoader>,
    pub wake: Vec<ActionLoader>,
    pub sleep: Vec<ActionLoader>,
    pub shutdown: Vec<ActionLoader>,
    pub brightness: f32,
    pub display_enabled: bool,
    pub display_brightness: Brightness,
    pub builtin_anims_enabled: bool,
    pub builtin_anims: Animations,
}

impl From<AnimeConfigV472> for AnimeConfig {
    fn from(c: AnimeConfigV472) -> AnimeConfig {
        AnimeConfig {
            system: c.system,
            boot: c.boot,
            wake: c.wake,
            shutdown: c.shutdown,
            model_override: c.model_override,
            display_enabled: c.display_enabled,
            display_brightness: c.display_brightness,
            builtin_anims_enabled: c.builtin_anims_enabled,
            builtin_anims: c.builtin_anims,
            ..Default::default()
        }
    }
}

#[derive(DeRon, SerRon, Default)]
pub struct AnimeConfigCached {
    pub system: Vec<ActionData>,
    pub boot: Vec<ActionData>,
    pub wake: Vec<ActionData>,
    pub shutdown: Vec<ActionData>,
}

impl AnimeConfigCached {
    pub fn init_from_config(
        &mut self,
        config: &AnimeConfig,
        anime_type: AnimeType,
    ) -> Result<(), AnimeError> {
        let mut sys = Vec::with_capacity(config.system.len());
        for ani in &config.system {
            sys.push(ActionData::from_anime_action(anime_type, ani)?);
        }
        self.system = sys;

        let mut boot = Vec::with_capacity(config.boot.len());
        for ani in &config.boot {
            boot.push(ActionData::from_anime_action(anime_type, ani)?);
        }
        self.boot = boot;

        let mut wake = Vec::with_capacity(config.wake.len());
        for ani in &config.wake {
            wake.push(ActionData::from_anime_action(anime_type, ani)?);
        }
        self.wake = wake;

        let mut shutdown = Vec::with_capacity(config.shutdown.len());
        for ani in &config.shutdown {
            shutdown.push(ActionData::from_anime_action(anime_type, ani)?);
        }
        self.shutdown = shutdown;
        Ok(())
    }
}

/// Config for base system actions for the anime display
#[derive(DeRon, SerRon, Debug, Clone)]
pub struct AnimeConfig {
    pub model_override: Option<AnimeType>,
    pub system: Vec<ActionLoader>,
    pub boot: Vec<ActionLoader>,
    pub wake: Vec<ActionLoader>,
    pub shutdown: Vec<ActionLoader>,
    // pub brightness: f32,
    pub display_enabled: bool,
    pub display_brightness: Brightness,
    pub builtin_anims_enabled: bool,
    pub off_when_unplugged: bool,
    pub off_when_suspended: bool,
    pub off_when_lid_closed: bool,
    pub brightness_on_battery: Brightness,
    pub builtin_anims: Animations,
}

impl Default for AnimeConfig {
    fn default() -> Self {
        AnimeConfig {
            model_override: None,
            system: Vec::new(),
            boot: Vec::new(),
            wake: Vec::new(),
            shutdown: Vec::new(),
            // brightness: 1.0,
            display_enabled: true,
            display_brightness: Brightness::Med,
            builtin_anims_enabled: true,
            off_when_unplugged: true,
            off_when_suspended: true,
            off_when_lid_closed: true,
            brightness_on_battery: Brightness::Low,
            builtin_anims: Animations::default(),
        }
    }
}

impl StdConfig for AnimeConfig {
    fn new() -> Self {
        Self::create_default()
    }

    fn file_name(&self) -> String {
        CONFIG_FILE.to_owned()
    }

    fn config_dir() -> std::path::PathBuf {
        std::path::PathBuf::from(crate::CONFIG_PATH_BASE)
    }
}

impl StdConfigLoad2<AnimeConfigV460, AnimeConfigV472> for AnimeConfig {}

impl From<&AnimeConfig> for DeviceState {
    fn from(config: &AnimeConfig) -> Self {
        DeviceState {
            display_enabled: config.display_enabled,
            display_brightness: config.display_brightness,
            builtin_anims_enabled: config.builtin_anims_enabled,
            builtin_anims: config.builtin_anims,
            off_when_unplugged: config.off_when_unplugged,
            off_when_suspended: config.off_when_suspended,
            off_when_lid_closed: config.off_when_lid_closed,
            brightness_on_battery: config.brightness_on_battery,
        }
    }
}

impl AnimeConfig {
    // fn clamp_config_brightness(mut config: &mut AnimeConfig) {
    //     if config.brightness < 0.0 || config.brightness > 1.0 {
    //         warn!(
    //             "Clamped brightness to [0.0 ; 1.0], was {}",
    //             config.brightness
    //         );
    //         config.brightness = f32::max(0.0, f32::min(1.0, config.brightness));
    //     }
    // }

    fn create_default() -> Self {
        // create a default config here
        AnimeConfig {
            system: vec![],
            boot: vec![ActionLoader::ImageAnimation {
                file: "/usr/share/asusd/anime/custom/sonic-run.gif".into(),
                scale: 0.9,
                angle: 0.65,
                translation: MyVec2::default(),
                brightness: 1.0,
                time: AnimTime::Fade(Fade::new(
                    Duration::from_secs(2),
                    Some(Duration::from_secs(2)),
                    Duration::from_secs(2),
                )),
            }],
            wake: vec![ActionLoader::ImageAnimation {
                file: "/usr/share/asusd/anime/custom/sonic-run.gif".into(),
                scale: 0.9,
                angle: 0.65,
                translation: MyVec2::default(),
                brightness: 1.0,
                time: AnimTime::Fade(Fade::new(
                    Duration::from_secs(2),
                    Some(Duration::from_secs(2)),
                    Duration::from_secs(2),
                )),
            }],
            shutdown: vec![ActionLoader::ImageAnimation {
                file: "/usr/share/asusd/anime/custom/sonic-wait.gif".into(),
                scale: 0.9,
                angle: 0.0,
                translation: MyVec2 { x: 3.0, y: 2.0 },
                brightness: 1.0,
                time: AnimTime::Infinite,
            }],
            ..Default::default()
        }
    }
}
