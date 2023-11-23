use std::path::PathBuf;

use log::{info, warn};
use serde::{Deserialize, Serialize};
use typeshare::typeshare;
use zbus::zvariant::{OwnedValue, Type, Value};

use crate::error::{PlatformError, Result};
use crate::platform::PlatformPolicy;
use crate::{read_attr_string, to_device};

const ATTR_AVAILABLE_GOVERNORS: &str = "cpufreq/scaling_available_governors";
const ATTR_GOVERNOR: &str = "cpufreq/scaling_governor";
const ATTR_AVAILABLE_EPP: &str = "cpufreq/energy_performance_available_preferences";
const ATTR_EPP: &str = "cpufreq/energy_performance_preference";

/// Both modern AMD and Intel have cpufreq control if using `powersave`
/// governor. What interests us the most here is `energy_performance_preference`
/// which can drastically alter CPU performance.
#[derive(Debug, PartialEq, Eq, PartialOrd, Clone)]
pub struct CPUControl {
    paths: Vec<PathBuf>,
}

impl CPUControl {
    pub fn new() -> Result<Self> {
        let mut enumerator = udev::Enumerator::new().map_err(|err| {
            warn!("{}", err);
            PlatformError::Udev("enumerator failed".into(), err)
        })?;
        enumerator.match_subsystem("cpu").map_err(|err| {
            warn!("{}", err);
            PlatformError::Udev("match_subsystem failed".into(), err)
        })?;

        let mut supported = false;
        let mut cpu = CPUControl { paths: Vec::new() };
        for device in enumerator.scan_devices().map_err(|err| {
            warn!("{}", err);
            PlatformError::Udev("CPU: scan_devices failed".into(), err)
        })? {
            if !supported {
                info!(
                    "Found CPU support at {:?}, checking supported items",
                    device.sysname()
                );

                match device.attribute_value(ATTR_AVAILABLE_GOVERNORS) {
                    Some(g) => info!("{ATTR_AVAILABLE_GOVERNORS}: {g:?}"),
                    None => {
                        return Err(PlatformError::CPU(format!(
                            "{ATTR_AVAILABLE_GOVERNORS} not found"
                        )))
                    }
                }
                match device.attribute_value(ATTR_GOVERNOR) {
                    Some(g) => info!("{ATTR_GOVERNOR}: {g:?}"),
                    None => return Err(PlatformError::CPU(format!("{ATTR_GOVERNOR} not found"))),
                }
                match device.attribute_value(ATTR_AVAILABLE_EPP) {
                    Some(g) => info!("{ATTR_AVAILABLE_EPP}: {g:?}"),
                    None => {
                        return Err(PlatformError::CPU(format!(
                            "{ATTR_AVAILABLE_EPP} not found"
                        )))
                    }
                }
                match device.attribute_value(ATTR_EPP) {
                    Some(g) => info!("{ATTR_EPP}: {g:?}"),
                    None => return Err(PlatformError::CPU(format!("{ATTR_EPP} not found"))),
                }
                supported = true;
            }
            if supported {
                info!("Adding: {:?}", device.syspath());
                cpu.paths.push(device.syspath().to_owned());
            }
        }
        if cpu.paths.is_empty() {
            return Err(PlatformError::MissingFunction(
                "asus-nb-wmi not found".into(),
            ));
        }
        Ok(cpu)
    }

    pub fn get_governor(&self) -> Result<CPUGovernor> {
        if let Some(path) = self.paths.first() {
            let s = read_attr_string(&to_device(path)?, ATTR_GOVERNOR)?;
            Ok(s.as_str().into())
            // TODO: check cpu are sync
        } else {
            Err(PlatformError::CPU("No CPU's?".to_string()))
        }
    }

    pub fn get_available_governors(&self) -> Result<Vec<CPUGovernor>> {
        if let Some(path) = self.paths.first() {
            read_attr_string(&to_device(path)?, ATTR_AVAILABLE_GOVERNORS)
                .map(|s| s.split_whitespace().map(|s| s.into()).collect())
            // TODO: check cpu are sync
        } else {
            Err(PlatformError::CPU("No CPU's?".to_string()))
        }
    }

    pub fn set_governor(&self, gov: CPUGovernor) -> Result<()> {
        if !self.get_available_governors()?.contains(&gov) {
            return Err(PlatformError::CPU(format!("{gov:?} is not available")));
        }
        for path in &self.paths {
            let mut dev = to_device(path)?;
            dev.set_attribute_value(ATTR_AVAILABLE_GOVERNORS, String::from(gov))?;
        }
        Ok(())
    }

    pub fn get_epp(&self) -> Result<CPUEPP> {
        if let Some(path) = self.paths.first() {
            let s = read_attr_string(&to_device(path)?, ATTR_EPP)?;
            Ok(s.as_str().into())
            // TODO: check cpu are sync
        } else {
            Err(PlatformError::CPU("No CPU's?".to_string()))
        }
    }

    pub fn get_available_epp(&self) -> Result<Vec<CPUEPP>> {
        if let Some(path) = self.paths.first() {
            read_attr_string(&to_device(path)?, ATTR_AVAILABLE_EPP)
                .map(|s| s.split_whitespace().map(|s| s.into()).collect())
            // TODO: check cpu are sync
        } else {
            Err(PlatformError::CPU("No CPU's?".to_string()))
        }
    }

    pub fn set_epp(&self, epp: CPUEPP) -> Result<()> {
        if !self.get_available_epp()?.contains(&epp) {
            return Err(PlatformError::CPU(format!("{epp:?} is not available")));
        }
        for path in &self.paths {
            let mut dev = to_device(path)?;
            dev.set_attribute_value(ATTR_EPP, String::from(epp))?;
        }
        Ok(())
    }
}

#[typeshare]
#[repr(u8)]
#[derive(
    Deserialize, Serialize, Type, Value, OwnedValue, Debug, PartialEq, PartialOrd, Clone, Copy,
)]
#[zvariant(signature = "s")]
pub enum CPUGovernor {
    Performance = 0,
    Powersave = 1,
    BadValue = 2,
}

impl From<&str> for CPUGovernor {
    fn from(s: &str) -> Self {
        match s {
            "performance" => Self::Performance,
            "powersave" => Self::Powersave,
            _ => Self::BadValue,
        }
    }
}

impl From<CPUGovernor> for String {
    fn from(g: CPUGovernor) -> Self {
        match g {
            CPUGovernor::Performance => "performance".to_string(),
            CPUGovernor::Powersave => "powersave".to_string(),
            CPUGovernor::BadValue => "bad_value".to_string(),
        }
    }
}

#[typeshare]
#[repr(u8)]
#[derive(
    Deserialize, Serialize, Type, Value, OwnedValue, Debug, PartialEq, PartialOrd, Clone, Copy,
)]
#[zvariant(signature = "s")]
pub enum CPUEPP {
    Default = 0,
    Performance = 1,
    BalancePerformance = 2,
    BalancePower = 3,
    Power = 4,
}

impl From<PlatformPolicy> for CPUEPP {
    fn from(value: PlatformPolicy) -> Self {
        match value {
            PlatformPolicy::Balanced => CPUEPP::BalancePerformance,
            PlatformPolicy::Performance => CPUEPP::Performance,
            PlatformPolicy::Quiet => CPUEPP::Power,
        }
    }
}

impl From<&str> for CPUEPP {
    fn from(s: &str) -> Self {
        match s {
            "default" => Self::Default,
            "performance" => Self::Performance,
            "balance_performance" => Self::BalancePerformance,
            "balance_power" => Self::BalancePower,
            "power" => Self::Power,
            _ => Self::Default,
        }
    }
}

impl From<CPUEPP> for String {
    fn from(g: CPUEPP) -> Self {
        match g {
            CPUEPP::Default => "default".to_string(),
            CPUEPP::Performance => "performance".to_string(),
            CPUEPP::BalancePerformance => "balance_performance".to_string(),
            CPUEPP::BalancePower => "balance_power".to_string(),
            CPUEPP::Power => "power".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::CPUControl;
    use crate::cpu::{CPUGovernor, CPUEPP};

    #[test]
    #[ignore = "Can't run this in a docker image"]
    fn check_cpu() {
        let cpu = CPUControl::new().unwrap();
        assert_eq!(cpu.get_governor().unwrap(), CPUGovernor::Powersave);
        assert_eq!(
            cpu.get_available_governors().unwrap(),
            vec![CPUGovernor::Performance, CPUGovernor::Powersave]
        );

        assert_eq!(cpu.get_epp().unwrap(), CPUEPP::BalancePower);
        assert_eq!(
            cpu.get_available_epp().unwrap(),
            vec![
                CPUEPP::Default,
                CPUEPP::Performance,
                CPUEPP::BalancePerformance,
                CPUEPP::BalancePower,
                CPUEPP::Power,
            ]
        );
    }
}
