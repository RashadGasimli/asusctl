pub const LED_INIT1: [u8; 2] = [0x5d, 0xb9];
pub const LED_INIT2: &str = "]ASUS Tech.Inc."; // ] == 0x5d
pub const LED_INIT3: [u8; 6] = [0x5d, 0x05, 0x20, 0x31, 0, 0x08];
pub const LED_INIT4: &str = "^ASUS Tech.Inc."; // ^ == 0x5e
pub const LED_INIT5: [u8; 6] = [0x5e, 0x05, 0x20, 0x31, 0, 0x08];

use std::fmt::Display;
use std::str::FromStr;

use serde_derive::{Deserialize, Serialize};
use typeshare::typeshare;
#[cfg(feature = "dbus")]
use zbus::zvariant::{OwnedValue, Type, Value};

use crate::error::Error;
use crate::LED_MSG_LEN;

#[typeshare]
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(
    feature = "dbus",
    derive(Type, Value, OwnedValue),
    zvariant(signature = "u")
)]
pub enum LedBrightness {
    Off = 0,
    Low = 1,
    #[default]
    Med = 2,
    High = 3,
}

impl LedBrightness {
    pub const fn next(&self) -> Self {
        match self {
            Self::Off => Self::Low,
            Self::Low => Self::Med,
            Self::Med => Self::High,
            Self::High => Self::Off,
        }
    }

    pub const fn prev(&self) -> Self {
        match self {
            Self::Off => Self::High,
            Self::Low => Self::Off,
            Self::Med => Self::Low,
            Self::High => Self::Med,
        }
    }
}

impl From<u8> for LedBrightness {
    fn from(bright: u8) -> Self {
        match bright {
            0 => LedBrightness::Off,
            1 => LedBrightness::Low,
            3 => LedBrightness::High,
            _ => LedBrightness::Med,
        }
    }
}

impl From<LedBrightness> for u8 {
    fn from(l: LedBrightness) -> Self {
        l as u8
    }
}

#[typeshare]
#[cfg_attr(feature = "dbus", derive(Type, Value, OwnedValue))]
#[derive(Debug, Clone, PartialEq, Eq, Copy, Deserialize, Serialize)]
pub struct Colour {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Default for Colour {
    fn default() -> Self {
        Colour { r: 166, g: 0, b: 0 }
    }
}

impl FromStr for Colour {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() < 6 {
            return Err(Error::ParseColour);
        }
        let r = u8::from_str_radix(&s[0..2], 16).or(Err(Error::ParseColour))?;
        let g = u8::from_str_radix(&s[2..4], 16).or(Err(Error::ParseColour))?;
        let b = u8::from_str_radix(&s[4..6], 16).or(Err(Error::ParseColour))?;
        Ok(Colour { r, g, b })
    }
}

impl From<&[f32; 3]> for Colour {
    fn from(c: &[f32; 3]) -> Self {
        Self {
            r: (255.0 * c[0]) as u8,
            g: (255.0 * c[1]) as u8,
            b: (255.0 * c[2]) as u8,
        }
    }
}

impl From<Colour> for [f32; 3] {
    fn from(c: Colour) -> Self {
        [c.r as f32 / 255.0, c.g as f32 / 255.0, c.b as f32 / 255.0]
    }
}

impl From<&[u8; 3]> for Colour {
    fn from(c: &[u8; 3]) -> Self {
        Self {
            r: c[0],
            g: c[1],
            b: c[2],
        }
    }
}

impl From<Colour> for [u8; 3] {
    fn from(c: Colour) -> Self {
        [c.r, c.g, c.b]
    }
}

#[typeshare]
#[cfg_attr(
    feature = "dbus",
    derive(Type, Value, OwnedValue),
    zvariant(signature = "s")
)]
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub enum Speed {
    Low = 0xe1,
    #[default]
    Med = 0xeb,
    High = 0xf5,
}

impl FromStr for Speed {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.to_lowercase();
        match s.as_str() {
            "low" => Ok(Speed::Low),
            "med" => Ok(Speed::Med),
            "high" => Ok(Speed::High),
            _ => Err(Error::ParseSpeed),
        }
    }
}

impl From<Speed> for u8 {
    fn from(s: Speed) -> u8 {
        match s {
            Speed::Low => 0,
            Speed::Med => 1,
            Speed::High => 2,
        }
    }
}
/// Used for Rainbow mode.
///
/// Enum corresponds to the required integer value
#[typeshare]
#[cfg_attr(
    feature = "dbus",
    derive(Type, Value, OwnedValue),
    zvariant(signature = "s")
)]
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub enum Direction {
    #[default]
    Right = 0,
    Left = 1,
    Up = 2,
    Down = 3,
}

impl FromStr for Direction {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.to_lowercase();
        match s.as_str() {
            "right" => Ok(Direction::Right),
            "up" => Ok(Direction::Up),
            "down" => Ok(Direction::Down),
            "left" => Ok(Direction::Left),
            _ => Err(Error::ParseDirection),
        }
    }
}

/// Enum of modes that convert to the actual number required by a USB HID packet
#[typeshare]
#[cfg_attr(
    feature = "dbus",
    derive(Type, Value, OwnedValue),
    zvariant(signature = "u")
)]
#[derive(
    Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Copy, Deserialize, Serialize,
)]
pub enum AuraModeNum {
    #[default]
    Static = 0,
    Breathe = 1,
    Strobe = 2,
    Rainbow = 3,
    Star = 4,
    Rain = 5,
    Highlight = 6,
    Laser = 7,
    Ripple = 8,
    Pulse = 10,
    Comet = 11,
    Flash = 12,
}

impl Display for AuraModeNum {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", <&str>::from(self))
    }
}

impl From<AuraModeNum> for String {
    fn from(mode: AuraModeNum) -> Self {
        <&str>::from(&mode).to_owned()
    }
}

impl From<&AuraModeNum> for &str {
    fn from(mode: &AuraModeNum) -> Self {
        match mode {
            AuraModeNum::Static => "Static",
            AuraModeNum::Breathe => "Breathe",
            AuraModeNum::Strobe => "Strobe",
            AuraModeNum::Rainbow => "Rainbow",
            AuraModeNum::Star => "Stars",
            AuraModeNum::Rain => "Rain",
            AuraModeNum::Highlight => "Highlight",
            AuraModeNum::Laser => "Laser",
            AuraModeNum::Ripple => "Ripple",
            AuraModeNum::Pulse => "Pulse",
            AuraModeNum::Comet => "Comet",
            AuraModeNum::Flash => "Flash",
        }
    }
}
impl From<&str> for AuraModeNum {
    fn from(mode: &str) -> Self {
        match mode {
            "Breathe" => AuraModeNum::Breathe,
            "Strobe" => AuraModeNum::Strobe,
            "Rainbow" => AuraModeNum::Rainbow,
            "Stars" => AuraModeNum::Star,
            "Rain" => AuraModeNum::Rain,
            "Highlight" => AuraModeNum::Highlight,
            "Laser" => AuraModeNum::Laser,
            "Ripple" => AuraModeNum::Ripple,
            "Pulse" => AuraModeNum::Pulse,
            "Comet" => AuraModeNum::Comet,
            "Flash" => AuraModeNum::Flash,
            _ => AuraModeNum::Static,
        }
    }
}

impl From<u8> for AuraModeNum {
    fn from(mode: u8) -> Self {
        match mode {
            1 => AuraModeNum::Breathe,
            2 => AuraModeNum::Strobe,
            3 => AuraModeNum::Rainbow,
            4 => AuraModeNum::Star,
            5 => AuraModeNum::Rain,
            6 => AuraModeNum::Highlight,
            7 => AuraModeNum::Laser,
            8 => AuraModeNum::Ripple,
            10 => AuraModeNum::Pulse,
            11 => AuraModeNum::Comet,
            12 => AuraModeNum::Flash,
            _ => AuraModeNum::Static,
        }
    }
}

impl From<AuraEffect> for AuraModeNum {
    fn from(value: AuraEffect) -> Self {
        value.mode
    }
}

/// Base effects have no zoning, while multizone is 1-4
#[typeshare]
#[cfg_attr(
    feature = "dbus",
    derive(Type, Value, OwnedValue),
    zvariant(signature = "s")
)]
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub enum AuraZone {
    /// Used if keyboard has no zones, or if setting all
    #[default]
    None = 0,
    /// Leftmost zone
    Key1 = 1,
    /// Zone after leftmost
    Key2 = 2,
    /// Zone second from right
    Key3 = 3,
    /// Rightmost zone
    Key4 = 4,
    /// Logo on the lid (or elsewhere?)
    Logo = 5,
    /// The left part of a lightbar (typically on the front of laptop)
    BarLeft = 6,
    /// The right part of a lightbar
    BarRight = 7,
}

impl FromStr for AuraZone {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.to_lowercase();
        match s.to_ascii_lowercase().as_str() {
            "0" | "none" => Ok(AuraZone::None),
            "1" | "one" => Ok(AuraZone::Key1),
            "2" | "two" => Ok(AuraZone::Key2),
            "3" | "three" => Ok(AuraZone::Key3),
            "4" | "four" => Ok(AuraZone::Key4),
            "5" | "logo" => Ok(AuraZone::Logo),
            "6" | "lightbar-left" => Ok(AuraZone::BarLeft),
            "7" | "lightbar-right" => Ok(AuraZone::BarRight),
            _ => Err(Error::ParseSpeed),
        }
    }
}

/// Default factory modes structure. This easily converts to an USB HID packet
/// with:
/// ```rust
/// // let bytes: [u8; LED_MSG_LEN] = mode.into();
/// ```
#[typeshare]
#[cfg_attr(feature = "dbus", derive(Type, Value, OwnedValue))]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AuraEffect {
    /// The effect type
    pub mode: AuraModeNum,
    /// `AuraZone::None` for no zone or zoneless keyboards
    pub zone: AuraZone,
    /// Primary colour for all modes
    pub colour1: Colour,
    /// Secondary colour in some modes like Breathing or Stars
    pub colour2: Colour,
    /// One of three speeds for modes that support speed (most that animate)
    pub speed: Speed,
    /// Up, down, left, right. Only Rainbow mode seems to use this
    pub direction: Direction,
}

impl AuraEffect {
    pub fn mode(&self) -> &AuraModeNum {
        &self.mode
    }

    pub fn mode_name(&self) -> &str {
        <&str>::from(&self.mode)
    }

    pub fn mode_num(&self) -> u8 {
        self.mode as u8
    }

    pub fn default_with_mode(mode: AuraModeNum) -> Self {
        Self {
            mode,
            ..Default::default()
        }
    }

    pub fn zone(&self) -> AuraZone {
        self.zone
    }
}

impl Default for AuraEffect {
    fn default() -> Self {
        Self {
            mode: AuraModeNum::Static,
            zone: AuraZone::None,
            colour1: Colour { r: 166, g: 0, b: 0 },
            colour2: Colour { r: 0, g: 0, b: 0 },
            speed: Speed::Med,
            direction: Direction::Right,
        }
    }
}

impl Display for AuraEffect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub struct AuraParameters {
    pub zone: bool,
    pub colour1: bool,
    pub colour2: bool,
    pub speed: bool,
    pub direction: bool,
}

#[allow(clippy::fn_params_excessive_bools)]
impl AuraParameters {
    pub const fn new(
        zone: bool,
        colour1: bool,
        colour2: bool,
        speed: bool,
        direction: bool,
    ) -> Self {
        Self {
            zone,
            colour1,
            colour2,
            speed,
            direction,
        }
    }
}

impl AuraEffect {
    /// A helper to provide detail on what effects have which parameters, e.g
    /// the static factory mode accepts only one colour.
    pub const fn allowed_parameters(mode: AuraModeNum) -> AuraParameters {
        match mode {
            AuraModeNum::Static
            | AuraModeNum::Highlight
            | AuraModeNum::Pulse
            | AuraModeNum::Comet
            | AuraModeNum::Flash => AuraParameters::new(true, true, false, false, false),
            AuraModeNum::Breathe => AuraParameters::new(true, true, true, true, false),
            AuraModeNum::Strobe | AuraModeNum::Rain => {
                AuraParameters::new(true, false, false, true, false)
            }
            AuraModeNum::Rainbow => AuraParameters::new(true, false, false, true, true),
            AuraModeNum::Star => AuraParameters::new(true, true, true, true, true),
            AuraModeNum::Laser | AuraModeNum::Ripple => {
                AuraParameters::new(true, true, false, true, false)
            }
        }
    }
}

/// Parses `AuraEffect` in to packet data for writing to the USB interface
///
/// Byte structure where colour is RGB, one byte per R, G, B:
/// ```ignore
/// | 0 | 1 | 2   | 3   | 4, 5, 6 | 7    | 8        | 9 | 10, 11, 12|
/// |---|---|-----|-----|---------|------|----------|---|-----------|
/// |5d |b3 |Zone |Mode |Colour 1 |Speed |Direction |00 |Colour 2   |
/// ```
impl From<&AuraEffect> for [u8; LED_MSG_LEN] {
    fn from(aura: &AuraEffect) -> Self {
        let mut msg = [0u8; LED_MSG_LEN];
        msg[0] = 0x5d;
        msg[1] = 0xb3;
        msg[2] = aura.zone as u8;
        msg[3] = aura.mode as u8;
        msg[4] = aura.colour1.r;
        msg[5] = aura.colour1.g;
        msg[6] = aura.colour1.b;
        msg[7] = aura.speed as u8;
        msg[8] = aura.direction as u8;
        msg[10] = aura.colour2.r;
        msg[11] = aura.colour2.g;
        msg[12] = aura.colour2.b;
        msg
    }
}

impl From<&AuraEffect> for Vec<u8> {
    fn from(aura: &AuraEffect) -> Self {
        let mut msg = vec![0u8; LED_MSG_LEN];
        msg[0] = 0x5d;
        msg[1] = 0xb3;
        msg[2] = aura.zone as u8;
        msg[3] = aura.mode as u8;
        msg[4] = aura.colour1.r;
        msg[5] = aura.colour1.g;
        msg[6] = aura.colour1.b;
        msg[7] = aura.speed as u8;
        msg[8] = aura.direction as u8;
        msg[10] = aura.colour2.r;
        msg[11] = aura.colour2.g;
        msg[12] = aura.colour2.b;
        msg
    }
}

#[cfg(test)]
mod tests {
    use crate::{AuraEffect, AuraModeNum, AuraZone, Colour, Direction, Speed, LED_MSG_LEN};

    #[test]
    fn check_led_static_packet() {
        let st = AuraEffect {
            mode: AuraModeNum::Static,
            zone: AuraZone::None,
            colour1: Colour {
                r: 0xff,
                g: 0x11,
                b: 0xdd,
            },
            colour2: Colour::default(),
            speed: Speed::Med,
            direction: Direction::Right,
        };
        let ar = <[u8; LED_MSG_LEN]>::from(&st);

        println!("{:02x?}", ar);
        let check = [
            0x5d, 0xb3, 0x0, 0x0, 0xff, 0x11, 0xdd, 0xeb, 0x0, 0x0, 0xa6, 0x0, 0x0, 0x0, 0x0, 0x0,
            0x0,
        ];
        assert_eq!(ar, check);
    }

    #[test]
    fn check_led_static_zone_packet() {
        let mut st = AuraEffect {
            mode: AuraModeNum::Static,
            zone: AuraZone::Key1,
            colour1: Colour {
                r: 0xff,
                g: 0,
                b: 0,
            },
            colour2: Colour { r: 0, g: 0, b: 0 },
            speed: Speed::Low,
            direction: Direction::Left,
        };
        let capture = [
            0x5d, 0xb3, 0x01, 0x00, 0xff, 0x00, 0x00, 0xe1, 0x01, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0,
            0x0, 0x0,
        ];
        assert_eq!(<[u8; LED_MSG_LEN]>::from(&st)[..9], capture[..9]);

        st.zone = AuraZone::Key2;
        st.colour1 = Colour {
            r: 0xff,
            g: 0xff,
            b: 0,
        };
        let capture = [
            0x5d, 0xb3, 0x02, 0x00, 0xff, 0xff, 0x00, 0xe1, 0x01, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0,
            0x0, 0x0,
        ];
        assert_eq!(<[u8; LED_MSG_LEN]>::from(&st)[..9], capture[..9]);

        st.zone = AuraZone::Key3;
        st.colour1 = Colour {
            r: 0,
            g: 0xff,
            b: 0xff,
        };
        let capture = [
            0x5d, 0xb3, 0x03, 0x00, 0x00, 0xff, 0xff, 0xe1, 0x01, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0,
            0x0, 0x0,
        ];
        assert_eq!(<[u8; LED_MSG_LEN]>::from(&st)[..9], capture[..9]);

        st.zone = AuraZone::Key4;
        st.colour1 = Colour {
            r: 0xff,
            g: 0x00,
            b: 0xff,
        };
        let capture = [
            0x5d, 0xb3, 0x04, 0x00, 0xff, 0x00, 0xff, 0xe1, 0x01, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0,
            0x0, 0x0,
        ];
        assert_eq!(<[u8; LED_MSG_LEN]>::from(&st)[..9], capture[..9]);

        st.zone = AuraZone::Logo;
        st.colour1 = Colour {
            r: 0x2c,
            g: 0xff,
            b: 0x00,
        };
        let capture = [
            0x5d, 0xb3, 0x05, 0x00, 0x2c, 0xff, 0x00, 0xe1, 0x01, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0,
            0x0, 0x0,
        ];
        assert_eq!(<[u8; LED_MSG_LEN]>::from(&st)[..9], capture[..9]);

        st.zone = AuraZone::BarLeft;
        st.colour1 = Colour {
            r: 0xff,
            g: 0x00,
            b: 0x00,
        };
        let capture = [
            0x5d, 0xb3, 0x06, 0x00, 0xff, 0x00, 0x00, 0xe1, 0x01, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0,
            0x0, 0x0,
        ];
        assert_eq!(<[u8; LED_MSG_LEN]>::from(&st)[..9], capture[..9]);

        st.zone = AuraZone::BarRight;
        st.colour1 = Colour {
            r: 0xff,
            g: 0x00,
            b: 0xcd,
        };
        let capture = [
            0x5d, 0xb3, 0x07, 0x00, 0xff, 0x00, 0xcd, 0xe1, 0x01, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0,
            0x0, 0x0,
        ];
        assert_eq!(<[u8; LED_MSG_LEN]>::from(&st)[..9], capture[..9]);

        st.mode = AuraModeNum::Rainbow;
        let capture = [
            0x5d, 0xb3, 0x07, 0x03, 0xff, 0x00, 0xcd, 0xe1, 0x01, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0,
            0x0, 0x0,
        ];
        assert_eq!(<[u8; LED_MSG_LEN]>::from(&st)[..9], capture[..9]);
    }
}
