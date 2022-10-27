//! A composable library for Finite Element computations.
//!
//! **Although featureful, the library API is completely unstable, the functionality is not
//! sufficiently well tested and documentation is sparse. Production usage strongly discouraged
//! at this point.**
//!
//! Please see the [repository README](https://github.com/InteractiveComputerGraphics/fenris) for more information.
use nalgebra::{Const, DimMin, DimName, U1};

pub mod allocators;
pub mod assembly;
pub mod connectivity;
pub mod element;
pub mod error;
pub mod integrate;
pub mod interpolate;
pub mod io;
pub mod mesh;
pub mod model;
pub mod quadrature;
pub mod space;
pub mod util;

pub mod geometry {
    pub use fenris_geometry::*;
}

#[cfg(feature = "proptest")]
pub mod proptest;

mod mesh_convert;
mod space_impl;

pub extern crate eyre;
pub extern crate nalgebra;
pub extern crate nalgebra_sparse;
pub extern crate vtkio;

pub use fenris_traits::Real;

/// A small, fixed-size dimension.
///
/// Used as a trait alias for various traits frequently needed by generic `fenris` routines.
pub trait SmallDim: DimName + DimMin<Self, Output = Self> {}

impl<D> SmallDim for D where D: DimName + DimMin<Self, Output = Self> {}

mod internal {
    pub trait Sealed {}
}

/// A small fixed-size dimension that is guaranteed to be implemented only for `Const` dimensions.
///
/// This means that an instance of `PhysicalDim`.
pub unsafe trait PhysicalDim: SmallDim + internal::Sealed {}

// pub fn physical_to_const<const D: usize>() -> Const<D> {
//
// }

macro_rules! impl_physical_dim {
    ($($dim:expr),*) => {
        $(
        impl internal::Sealed for Const<$dim> {}
        unsafe impl PhysicalDim for Const<$dim> {}
        )*
    }
}

impl_physical_dim!(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12);

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum Symmetry {
    NonSymmetric,
    Symmetric,
}
