use crate::ImageFileDirectory;

/// Affine transformation values.
#[derive(Debug)]
pub struct AffineTransform(f64, f64, f64, f64, f64, f64);

impl AffineTransform {
    pub fn new(a: f64, b: f64, xoff: f64, d: f64, e: f64, yoff: f64) -> Self {
        Self(a, b, xoff, d, e, yoff)
    }

    pub fn a(&self) -> f64 {
        self.0
    }

    pub fn b(&self) -> f64 {
        self.1
    }

    pub fn c(&self) -> f64 {
        self.2
    }

    pub fn d(&self) -> f64 {
        self.3
    }

    pub fn e(&self) -> f64 {
        self.4
    }

    pub fn f(&self) -> f64 {
        self.5
    }

    /// Construct a new Affine Transform from the IFD
    pub fn from_ifd(ifd: &ImageFileDirectory) -> Option<Self> {
        if let (Some(model_pixel_scale), Some(model_tiepoint)) =
            (&ifd.model_pixel_scale, &ifd.model_tiepoint)
        {
            Some(Self::new(
                model_pixel_scale[0],
                0.0,
                model_tiepoint[3],
                0.0,
                -model_pixel_scale[1],
                model_tiepoint[4],
            ))
        } else {
            None
        }
    }
}
