//! Word fork stand-in for `xcap`: the items `craft-platform::screen` calls, and no capture.
//! [`Monitor::all`] always fails, so no [`Monitor`] ever exists.

use std::fmt;

/// Why nothing was captured.
#[derive(Debug)]
pub struct XCapError;

impl fmt::Display for XCapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("screen capture is not included in this build")
    }
}

impl std::error::Error for XCapError {}

/// A result of this crate.
pub type XCapResult<T> = Result<T, XCapError>;

/// A monitor. None exists: the type has no values.
pub enum Monitor {}

/// A captured image. None exists: the type has no values.
pub enum RgbaImage {}

impl Monitor {
    /// Always fails: this build captures no screen.
    pub fn all() -> XCapResult<Vec<Monitor>> {
        Err(XCapError)
    }
    pub fn is_primary(&self) -> XCapResult<bool> {
        match *self {}
    }
    pub fn scale_factor(&self) -> XCapResult<f32> {
        match *self {}
    }
    pub fn x(&self) -> XCapResult<i32> {
        match *self {}
    }
    pub fn y(&self) -> XCapResult<i32> {
        match *self {}
    }
    pub fn name(&self) -> XCapResult<String> {
        match *self {}
    }
    pub fn capture_image(&self) -> XCapResult<RgbaImage> {
        match *self {}
    }
}

impl RgbaImage {
    pub fn width(&self) -> u32 {
        match *self {}
    }
    pub fn height(&self) -> u32 {
        match *self {}
    }
    pub fn into_raw(self) -> Vec<u8> {
        match self {}
    }
}
