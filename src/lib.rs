pub mod allocators;
pub mod assembly;
pub mod assembly2;
pub mod connectivity;
pub mod element;
pub mod error;
pub mod mesh;
pub mod model;
pub mod procedural;
pub mod quadrature;
pub mod reorder;
pub mod space;
pub mod util;
pub mod vtk;

pub(crate) mod workspace;

pub mod geometry {
    pub use fenris_geometry::*;
}

pub mod optimize {
    pub use fenris_optimize::*;
}

#[cfg(feature = "proptest")]
pub mod proptest;

mod mesh_convert;
mod space_impl;

pub extern crate nalgebra;
pub extern crate nested_vec;
pub extern crate vtkio;

use nalgebra::{DimMin, DimName};

/// A small, fixed-size dimension.
///
/// Used as a trait alias for various traits frequently needed by generic `fenris` routines.
pub trait SmallDim: DimName + DimMin<Self, Output = Self> {}

impl<D> SmallDim for D where D: DimName + DimMin<Self, Output = Self> {}
