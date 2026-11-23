//! Word fork stand-in for `core-graphics2`: the items `craft-platform::screen` calls on macOS,
//! and no capture. [`display::get_active_display_list`] finds no display, so no display or
//! image value ever exists.

/// A point.
#[derive(Clone, Copy, Debug, Default)]
pub struct CGPoint {
    pub x: f64,
    pub y: f64,
}

/// A size.
#[derive(Clone, Copy, Debug, Default)]
pub struct CGSize {
    pub width: f64,
    pub height: f64,
}

/// A rectangle.
#[derive(Clone, Copy, Debug, Default)]
pub struct CGRect {
    pub origin: CGPoint,
    pub size: CGSize,
}

/// Bytes. None exists: the type has no values.
pub enum Data {}

impl Data {
    pub fn len(&self) -> usize {
        match *self {}
    }
    pub fn is_empty(&self) -> bool {
        match *self {}
    }
    pub fn bytes(&self) -> &[u8] {
        match *self {}
    }
}

pub mod display {
    use super::CGRect;

    /// A display. None exists: the type has no values.
    pub enum CGDisplay {}

    /// A display mode. None exists: the type has no values.
    pub enum CGDisplayMode {}

    /// Always `None`: this build captures no screen.
    pub fn get_active_display_list(_max: usize) -> Option<Vec<CGDisplay>> {
        None
    }

    impl CGDisplay {
        pub fn is_main(&self) -> bool {
            match *self {}
        }
        pub fn copy_display_mode(&self) -> Option<CGDisplayMode> {
            match *self {}
        }
        pub fn bounds(&self) -> CGRect {
            match *self {}
        }
    }

    impl CGDisplayMode {
        pub fn pixel_width(&self) -> usize {
            match *self {}
        }
        pub fn pixel_height(&self) -> usize {
            match *self {}
        }
    }
}

pub mod image {
    use super::Data;

    /// How an image stores alpha.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum CGImageAlphaInfo {
        AlphaNone,
        AlphaPremultipliedLast,
        AlphaPremultipliedFirst,
        AlphaLast,
        AlphaFirst,
        AlphaNoneSkipLast,
        AlphaNoneSkipFirst,
        AlphaOnly,
    }

    /// Bitmap layout flags.
    #[derive(Clone, Copy, Debug)]
    pub struct CGBitmapInfo(u32);

    impl CGBitmapInfo {
        pub fn bits(&self) -> u32 {
            self.0
        }
    }

    /// A colour space. None exists: the type has no values.
    pub enum CGColorSpace {}

    impl CGColorSpace {
        pub fn copy_icc_data(&self) -> Option<Data> {
            match *self {}
        }
    }

    /// An image's pixel source. None exists: the type has no values.
    pub enum CGDataProvider {}

    impl CGDataProvider {
        pub fn copy_data(&self) -> Option<Data> {
            match *self {}
        }
    }

    /// An image. None exists: the type has no values.
    pub enum CGImage {}

    impl CGImage {
        pub fn width(&self) -> usize {
            match *self {}
        }
        pub fn height(&self) -> usize {
            match *self {}
        }
        pub fn color_space(&self) -> Option<CGColorSpace> {
            match *self {}
        }
        pub fn bitmap_info(&self) -> CGBitmapInfo {
            match *self {}
        }
        pub fn bits_per_component(&self) -> usize {
            match *self {}
        }
        pub fn bits_per_pixel(&self) -> usize {
            match *self {}
        }
        pub fn alpha_info(&self) -> CGImageAlphaInfo {
            match *self {}
        }
        pub fn bytes_per_row(&self) -> usize {
            match *self {}
        }
        pub fn data_provider(&self) -> Option<CGDataProvider> {
            match *self {}
        }
    }
}

pub mod window {
    use super::CGRect;
    use super::image::CGImage;

    /// The null window id.
    #[allow(non_upper_case_globals)]
    pub const kCGNullWindowID: u32 = 0;

    /// Which windows a list image takes.
    #[derive(Clone, Copy, Debug)]
    pub enum CGWindowListOption {
        OnScreenOnly,
    }

    /// Image options.
    #[derive(Clone, Copy, Debug)]
    pub struct CGWindowImageOption(u32);

    #[allow(non_upper_case_globals)]
    impl CGWindowImageOption {
        pub const BestResolution: Self = Self(1 << 3);
        pub const ShouldBeOpaque: Self = Self(1 << 1);
    }

    impl std::ops::BitOr for CGWindowImageOption {
        type Output = Self;
        fn bitor(self, rhs: Self) -> Self {
            Self(self.0 | rhs.0)
        }
    }

    /// Permission is never asked: the display list is empty before any capture would need it.
    pub fn preflight_screen_capture_access() -> bool {
        true
    }

    pub fn request_screen_capture_access() -> bool {
        true
    }

    /// Always `None`: this build captures no screen.
    pub fn new_image(_bounds: CGRect, _list: CGWindowListOption, _window: u32, _options: CGWindowImageOption) -> Option<CGImage> {
        None
    }
}
