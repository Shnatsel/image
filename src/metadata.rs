/// Describes the transformations to be applied to the image.
/// Compatible with [Exif orientation](https://web.archive.org/web/20200412005226/https://www.impulseadventure.com/photo/exif-orientation.html).
///
/// Orientation is specified in the Exif metadata, and is often written by cameras.
///
/// You can apply it to an image via [`DynamicImage::apply_orientation`](crate::DynamicImage::apply_orientation).
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum Orientation {
    /// Do not perform any transformations.
    NoTransforms,
    /// Rotate by 90 degrees clockwise.
    Rotate90,
    /// Rotate by 180 degrees. Can be performed in-place.
    Rotate180,
    /// Rotate by 270 degrees clockwise. Equivalent to rotating by 90 degrees counter-clockwise.
    Rotate270,
    /// Flip horizontally. Can be performed in-place.
    FlipHorizontal,
    /// Flip vertically. Can be performed in-place.
    FlipVertical,
    /// Rotate by 90 degrees clockwise and flip horizontally.
    Rotate90FlipH,
    /// Rotate by 270 degrees clockwise and flip horizontally.
    Rotate270FlipH,
}

impl Orientation {
    /// Converts from [Exif orientation](https://web.archive.org/web/20200412005226/https://www.impulseadventure.com/photo/exif-orientation.html)
    pub fn from_exif(exif_orientation: u8) -> Option<Self> {
        match exif_orientation {
            1 => Some(Self::NoTransforms),
            2 => Some(Self::FlipHorizontal),
            3 => Some(Self::Rotate180),
            4 => Some(Self::FlipVertical),
            5 => Some(Self::Rotate90FlipH),
            6 => Some(Self::Rotate90),
            7 => Some(Self::Rotate270FlipH),
            8 => Some(Self::Rotate270),
            0 | 9.. => None,
        }
    }

    /// Converts into [Exif orientation](https://web.archive.org/web/20200412005226/https://www.impulseadventure.com/photo/exif-orientation.html)
    pub fn to_exif(self) -> u8 {
        match self {
            Self::NoTransforms => 1,
            Self::FlipHorizontal => 2,
            Self::Rotate180 => 3,
            Self::FlipVertical => 4,
            Self::Rotate90FlipH => 5,
            Self::Rotate90 => 6,
            Self::Rotate270FlipH => 7,
            Self::Rotate270 => 8,
        }
    }

    /// Returns `true` if the specified orientation can be applied to an image in-place,
    /// without making a copy of the image.
    ///
    /// This is relevant if you need to enforce a memory limit,
    /// since a copy of the image will briefly use additional memory to store the copy.
    /// The required amount of additional memory is the same as the memory used to store the original image.
    pub fn applies_in_place(self) -> bool {
        match self {
            Orientation::NoTransforms => true,
            Orientation::Rotate90 => false,
            Orientation::Rotate180 => true,
            Orientation::Rotate270 => false,
            Orientation::FlipHorizontal => true,
            Orientation::FlipVertical => true,
            Orientation::Rotate90FlipH => false,
            Orientation::Rotate270FlipH => false,
        }
    }
}
