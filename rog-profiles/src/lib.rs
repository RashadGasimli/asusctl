pub mod error;
pub mod fan_curve_set;

use error::ProfileError;
use fan_curve_set::CurveData;
use log::debug;
use rog_platform::platform::PlatformPolicy;
use serde_derive::{Deserialize, Serialize};
use typeshare::typeshare;
pub use udev::Device;
#[cfg(feature = "dbus")]
use zbus::zvariant::Type;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn find_fan_curve_node() -> Result<Device, ProfileError> {
    let mut enumerator = udev::Enumerator::new()?;
    enumerator.match_subsystem("hwmon")?;

    for device in enumerator.scan_devices()? {
        if device.parent_with_subsystem("platform")?.is_some() {
            if let Some(name) = device.attribute_value("name") {
                if name == "asus_custom_fan_curve" {
                    return Ok(device);
                }
            }
        }
    }

    Err(ProfileError::NotSupported)
}

#[typeshare]
#[cfg_attr(feature = "dbus", derive(Type), zvariant(signature = "s"))]
#[derive(Deserialize, Serialize, Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum FanCurvePU {
    CPU,
    GPU,
    MID,
}

impl FanCurvePU {
    fn which_fans(device: &Device) -> Vec<Self> {
        let mut fans = Vec::with_capacity(3);
        for fan in [Self::CPU, Self::GPU, Self::MID] {
            let pwm_num: char = fan.into();
            let pwm_enable = format!("pwm{pwm_num}_enable");
            debug!("Looking for {pwm_enable}");
            for attr in device.attributes() {
                let tmp = attr.name().to_string_lossy();
                if tmp.contains(&pwm_enable) {
                    debug!("Found {pwm_enable}");
                    fans.push(fan);
                }
            }
        }
        fans
    }
}

impl From<FanCurvePU> for &str {
    fn from(pu: FanCurvePU) -> &'static str {
        match pu {
            FanCurvePU::CPU => "cpu",
            FanCurvePU::GPU => "gpu",
            FanCurvePU::MID => "mid",
        }
    }
}

impl From<FanCurvePU> for char {
    fn from(pu: FanCurvePU) -> char {
        match pu {
            FanCurvePU::CPU => '1',
            FanCurvePU::GPU => '2',
            FanCurvePU::MID => '3',
        }
    }
}

impl std::str::FromStr for FanCurvePU {
    type Err = ProfileError;

    fn from_str(fan: &str) -> Result<Self, Self::Err> {
        match fan.to_ascii_lowercase().trim() {
            "cpu" => Ok(FanCurvePU::CPU),
            "gpu" => Ok(FanCurvePU::GPU),
            "mid" => Ok(FanCurvePU::MID),
            _ => Err(ProfileError::ParseProfileName),
        }
    }
}

impl Default for FanCurvePU {
    fn default() -> Self {
        Self::CPU
    }
}

/// Main purpose of `FanCurves` is to enable restoring state on system boot
#[typeshare]
#[cfg_attr(feature = "dbus", derive(Type))]
#[derive(Deserialize, Serialize, Debug, Default)]
pub struct FanCurveProfiles {
    pub balanced: Vec<CurveData>,
    pub performance: Vec<CurveData>,
    pub quiet: Vec<CurveData>,
}

impl FanCurveProfiles {
    /// Return an array of `FanCurvePU`. An empty array indicates no support for
    /// Curves.
    pub fn supported_fans() -> Result<Vec<FanCurvePU>, ProfileError> {
        let device = find_fan_curve_node()?;
        Ok(FanCurvePU::which_fans(&device))
    }

    ///
    pub fn read_from_dev_profile(
        &mut self,
        profile: PlatformPolicy,
        device: &Device,
    ) -> Result<(), ProfileError> {
        let fans = Self::supported_fans()?;
        let mut curves = Vec::with_capacity(3);

        for fan in fans {
            let mut curve = CurveData {
                fan,
                ..Default::default()
            };
            debug!("Reading curve for {fan:?}");
            curve.read_from_device(device);
            debug!("Curve: {curve:?}");
            curves.push(curve);
        }

        match profile {
            PlatformPolicy::Balanced => self.balanced = curves,
            PlatformPolicy::Performance => self.performance = curves,
            PlatformPolicy::Quiet => self.quiet = curves,
        }
        Ok(())
    }

    /// Reset the stored (self) and device curve to the defaults of the
    /// platform.
    ///
    /// Each `platform_profile` has a different default and the defualt can be
    /// read only for the currently active profile.
    pub fn set_active_curve_to_defaults(
        &mut self,
        profile: PlatformPolicy,
        device: &mut Device,
    ) -> Result<(), ProfileError> {
        let fans = Self::supported_fans()?;
        // Do reset for all
        for fan in fans {
            let pwm_num: char = fan.into();
            let pwm = format!("pwm{pwm_num}_enable");
            device.set_attribute_value(&pwm, "3")?;
        }
        self.read_from_dev_profile(profile, device)?;
        Ok(())
    }

    /// Write the curves for the selected profile to the device. If the curve is
    /// in the enabled list it will become active. If the curve is zeroed it
    /// will be initialised to a default read from the system.
    // TODO: Make this return an error if curve is zeroed
    pub fn write_profile_curve_to_platform(
        &mut self,
        profile: PlatformPolicy,
        device: &mut Device,
    ) -> Result<(), ProfileError> {
        let fans = match profile {
            PlatformPolicy::Balanced => &mut self.balanced,
            PlatformPolicy::Performance => &mut self.performance,
            PlatformPolicy::Quiet => &mut self.quiet,
        };
        for fan in fans {
            debug!("write_profile_curve_to_platform: writing profile:{profile}, {fan:?}");
            fan.write_to_device(device)?;
        }
        Ok(())
    }

    pub fn set_profile_curves_enabled(&mut self, profile: PlatformPolicy, enabled: bool) {
        match profile {
            PlatformPolicy::Balanced => {
                for curve in self.balanced.iter_mut() {
                    curve.enabled = enabled;
                }
            }
            PlatformPolicy::Performance => {
                for curve in self.performance.iter_mut() {
                    curve.enabled = enabled;
                }
            }
            PlatformPolicy::Quiet => {
                for curve in self.quiet.iter_mut() {
                    curve.enabled = enabled;
                }
            }
        }
    }

    pub fn set_profile_fan_curve_enabled(
        &mut self,
        profile: PlatformPolicy,
        fan: FanCurvePU,
        enabled: bool,
    ) {
        match profile {
            PlatformPolicy::Balanced => {
                for curve in self.balanced.iter_mut() {
                    if curve.fan == fan {
                        curve.enabled = enabled;
                        break;
                    }
                }
            }
            PlatformPolicy::Performance => {
                for curve in self.performance.iter_mut() {
                    if curve.fan == fan {
                        curve.enabled = enabled;
                        break;
                    }
                }
            }
            PlatformPolicy::Quiet => {
                for curve in self.quiet.iter_mut() {
                    if curve.fan == fan {
                        curve.enabled = enabled;
                        break;
                    }
                }
            }
        }
    }

    pub fn get_fan_curves_for(&self, name: PlatformPolicy) -> &[CurveData] {
        match name {
            PlatformPolicy::Balanced => &self.balanced,
            PlatformPolicy::Performance => &self.performance,
            PlatformPolicy::Quiet => &self.quiet,
        }
    }

    pub fn get_fan_curve_for(&self, name: &PlatformPolicy, pu: FanCurvePU) -> Option<&CurveData> {
        match name {
            PlatformPolicy::Balanced => {
                for this_curve in self.balanced.iter() {
                    if this_curve.fan == pu {
                        return Some(this_curve);
                    }
                }
            }
            PlatformPolicy::Performance => {
                for this_curve in self.performance.iter() {
                    if this_curve.fan == pu {
                        return Some(this_curve);
                    }
                }
            }
            PlatformPolicy::Quiet => {
                for this_curve in self.quiet.iter() {
                    if this_curve.fan == pu {
                        return Some(this_curve);
                    }
                }
            }
        }
        None
    }

    pub fn save_fan_curve(
        &mut self,
        curve: CurveData,
        profile: PlatformPolicy,
    ) -> Result<(), ProfileError> {
        match profile {
            PlatformPolicy::Balanced => {
                for this_curve in self.balanced.iter_mut() {
                    if this_curve.fan == curve.fan {
                        *this_curve = curve;
                        break;
                    }
                }
            }
            PlatformPolicy::Performance => {
                for this_curve in self.performance.iter_mut() {
                    if this_curve.fan == curve.fan {
                        *this_curve = curve;
                        break;
                    }
                }
            }
            PlatformPolicy::Quiet => {
                for this_curve in self.quiet.iter_mut() {
                    if this_curve.fan == curve.fan {
                        *this_curve = curve;
                        break;
                    }
                }
            }
        }
        Ok(())
    }
}
