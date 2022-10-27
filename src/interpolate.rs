use crate::allocators::{BiDimAllocator, DimAllocator};
use crate::space::{FiniteElementConnectivity, FiniteElementSpace, GeometricFiniteElementSpace};
use crate::{Real, SmallDim};
use nalgebra::{Const, DefaultAllocator, DimName, Dynamic, MatrixSliceMut, OMatrix, OPoint, OVector, Scalar};
use std::array;
use std::cell::RefCell;
use std::marker::PhantomData;
use std::mem::transmute;
use davenport::Workspace;
use rstar::{RTree, RTreeObject};
use rstar::primitives::{GeomWithData, Rectangle};
use fenris_geometry::{AxisAlignedBoundingBox, BoundedGeometry, DistanceQuery, GeometryCollection};
use crate::util::{try_transmute_ref, try_transmute_slice};

pub trait InterpolateFiniteElementSpace<T>: FiniteElementSpace<T>
where
    // TODO: Ideally we should be able to use Scalar as a bound, but Scalar doesn't have
    // Default, and unfortunately e.g. OPoint<T, D> require Zero for their default
    // instead of T: Default. Should send a PR to nalgebra ...
    T: Real,
    DefaultAllocator: BiDimAllocator<T, Self::GeometryDim, Self::ReferenceDim>,
{
    // fn interpolate(&self, point: &OPoint<T, Self::GeometryDim>, weights: DVectorSlice<T>) -> OVector<>{
    //     let (element, coords) = self.find_closest_element_and_reference_coords(point);
    //     self.populate_element_basis(element, &mut [])
    // }
    //
    // fn interpolate_gradient(&self, point: &OPoint<T, Self::GeometryDim>, weights: DVectorSlice<T>)

    /// Find the closest point on the mesh to the given point, represented as the
    /// index of the closest element and the coordinates in the reference element.
    fn find_closest_element_and_reference_coords(
        &self,
        point: &OPoint<T, Self::GeometryDim>,
    ) -> (usize, OPoint<T, Self::ReferenceDim>) {
        let mut result = [(usize::MAX, OPoint::default()); 1];
        self.populate_closest_element_and_reference_coords(array::from_ref(point), &mut result);
        let [result] = result;
        result
    }

    /// Same as [`find_closest_element_and_reference_coords`], but applied to several
    /// points at the same time.
    ///
    /// # Panics
    ///
    /// The method should panic if the input point slice and the output slice
    /// do not have the same length.
    fn populate_closest_element_and_reference_coords(
        &self,
        points: &[OPoint<T, Self::GeometryDim>],
        result: &mut [(usize, OPoint<T, Self::ReferenceDim>)],
    );
}

struct RTreeAccelerationStructure<const D: usize>
where
    [f64; D]: rstar::Point
{
    tree: RTree<GeomWithData<Rectangle<[f64; D]>, usize>>,
}

impl<const D: usize> RTreeAccelerationStructure<D>
where
    [f64; D]: rstar::Point,
{
    fn from_bounding_boxes<T: Real, D2: SmallDim>(boxes: &[AxisAlignedBoundingBox<T, D2>]) -> Self
    where
        DefaultAllocator: DimAllocator<T, D2>
    {
        if let Some(boxes) = try_transmute_slice(boxes) {
            let boxes: &[AxisAlignedBoundingBox<T, Const<D>>] = boxes;
            let geometries = boxes.iter()
                .enumerate()
                .map(|(i, bounding_box)| {
                    let box_min: [f64; D] = bounding_box.min().map(|x| x.to_subset().unwrap()).into();
                    let box_max: [f64; D] = bounding_box.max().map(|x| x.to_subset().unwrap()).into();
                    GeomWithData::new(Rectangle::from_corners(box_min, box_max), i)
                }).collect();
            let tree = RTree::bulk_load(geometries);
            Self { tree }
        } else {
            panic!("Mismatched dimensions");
        }
    }
}


pub struct Interpolator<T, Space>
where
    T: Scalar,
    Space: FiniteElementSpace<T>,
    DefaultAllocator: BiDimAllocator<T, Space::GeometryDim, Space::ReferenceDim>,
{
    space: Space,
    // tree: RTree<Rectangle<[f64; Space::GeometryDim::USIZE]>>
    workspace: RefCell<Workspace>,
    marker: PhantomData<T>,
}

impl<T, Space> Interpolator<T, Space>
where
    T: Real,
    for<'a> Space: GeometricFiniteElementSpace<'a, T>,
    for<'a> <Space as GeometryCollection<'a>>::Geometry: BoundedGeometry<T, Dimension=Space::GeometryDim>,
    DefaultAllocator: BiDimAllocator<T, Space::GeometryDim, Space::ReferenceDim>,
{
    pub fn from_space(space: Space) -> Self {
        let bounding_boxes: Vec<_> = (0 .. space.num_geometries())
            .map(|i| space.get_geometry(i).unwrap().bounding_box())
            .collect();

        let mut workspace = Workspace::default();
        match Space::GeometryDim::dim() {
            // TODO: Support dimension 1, probably need to send a PR to rstar for this
            2 => {
                // TODO: Implement a try_insert method on davenport::Workspace?
                workspace.get_or_insert_with(|| RTreeAccelerationStructure::<2>::from_bounding_boxes(&bounding_boxes));
            },
            3 => {
                workspace.get_or_insert_with(|| RTreeAccelerationStructure::<3>::from_bounding_boxes(&bounding_boxes));
            },
            _ => panic!("Unsupported dimension. Currently we only support dimension 2 and 3")
        }

        Self {
            space,
            workspace: RefCell::new(workspace),
            marker: Default::default()
        }
    }
}

impl<T, Space> FiniteElementConnectivity for Interpolator<T, Space>
where
    T: Scalar,
    Space: FiniteElementSpace<T>,
    DefaultAllocator: BiDimAllocator<T, Space::GeometryDim, Space::ReferenceDim>,
{
    fn num_elements(&self) -> usize {
        self.space.num_elements()
    }

    fn num_nodes(&self) -> usize {
        self.space.num_nodes()
    }

    fn element_node_count(&self, element_index: usize) -> usize {
        self.space.element_node_count(element_index)
    }

    fn populate_element_nodes(&self, nodes: &mut [usize], element_index: usize) {
        self.space.populate_element_nodes(nodes, element_index)
    }
}

impl<T, Space> FiniteElementSpace<T> for Interpolator<T, Space>
where
    T: Scalar,
    Space: FiniteElementSpace<T>,
    DefaultAllocator: BiDimAllocator<T, Space::GeometryDim, Space::ReferenceDim>,
{
    type GeometryDim = Space::GeometryDim;
    type ReferenceDim = Space::ReferenceDim;

    fn populate_element_basis(&self, element_index: usize, basis_values: &mut [T], reference_coords: &OPoint<T, Self::ReferenceDim>) {
        self.space.populate_element_basis(element_index, basis_values, reference_coords)
    }

    fn populate_element_gradients(&self, element_index: usize, gradients: MatrixSliceMut<T, Self::ReferenceDim, Dynamic>, reference_coords: &OPoint<T, Self::ReferenceDim>) {
        self.space.populate_element_gradients(element_index, gradients, reference_coords)
    }

    fn element_reference_jacobian(&self, element_index: usize, reference_coords: &OPoint<T, Self::ReferenceDim>) -> OMatrix<T, Self::GeometryDim, Self::ReferenceDim> {
        self.space.element_reference_jacobian(element_index, reference_coords)
    }

    fn map_element_reference_coords(&self, element_index: usize, reference_coords: &OPoint<T, Self::ReferenceDim>) -> OPoint<T, Self::GeometryDim> {
        self.space.map_element_reference_coords(element_index, reference_coords)
    }

    fn diameter(&self, element_index: usize) -> T {
        self.space.diameter(element_index)
    }
}

impl<T, Space> InterpolateFiniteElementSpace<T> for Interpolator<T, Space>
where
    T: Real,
    Space: FiniteElementSpace<T>,
    DefaultAllocator: BiDimAllocator<T, Space::GeometryDim, Space::ReferenceDim>,
{
    fn populate_closest_element_and_reference_coords(&self,
                                                     points: &[OPoint<T, Self::GeometryDim>],
                                                     result: &mut [(usize, OPoint<T, Self::ReferenceDim>)]
    ) {
        let mut workspace = self.workspace.borrow_mut();
        match Space::GeometryDim::dim() {
            1 => {
                let rtree: &RTree<Rectangle<[f64; 2]>> = workspace.get_or_default();
            },
            _ => {}
        }

        todo!()
    }
}
