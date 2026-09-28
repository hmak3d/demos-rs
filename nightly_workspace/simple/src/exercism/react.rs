//! re: <https://exercism.org/tracks/rust/exercises/react/edit>
//!
//! Implement reactor pattern where updating one Cell can cascade refresh other Cell's that depend on it.
//!
//! # There are 2 approaches:
//!
//! We've implemented Approach B
//!
//! ## Approach A: Use interior mutabity on the cells. Pass non-mut &Reactor everywhere
//!
//! Pros:
//! * Easier to implement
//!
//! ## Approach B: Avoid interior mutability. Use &mut Reactor everyone. This means "flattening" the function call stack
//!   ... splitting up (1) find-the-cells to update from (2) updating the cells.
//!
//! Pros:
//! * More robust final implementation. Do more compile time checks. Eliminate the possiblity of RefCell::borrow/borrow_mut() panicking
//! * Make ping-pong code calls between Reactor vs Cell less likely. The borrow checker will complain more often.

use std::borrow::Borrow;
use std::collections::{HashMap, HashSet};
use std::iter;
#[cfg(not(feature = "react_dyn_get_dependencies"))]
use std::iter::{Copied, Empty};

//////////////////////////////////////////////////////////////////////////////// IDs

/// Define `struct $type_name(usize)` and other ceremonies to enable it to be in [HashMap], etc
///
/// e.g.,
/// ```ignore
/// define_id_type! {
///     /// Some doc comment
///     name = MyId,
///     counter = NEXT_MY_ID_COUNTER,
/// }
/// ```
/// will output
/// ```ignore
/// # use std::sync::atomic::{AtomicUsize, Ordering};
/// /// Counter for generating [MyId]
/// static NEXT_MY_ID_COUNTER: AtomicUsize = AtomicUsize::new(0);
///
/// /// Some doc comment
/// #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
/// pub struct MyId(usize);
///
/// impl MyId {
///     fn new() -> Self {
///         Self(NEXT_MY_ID_COUNTER.fetch_add(1, Ordering::Relaxed))
///     }
/// }
/// ```
macro_rules! define_id_type {
    ($(#[$attrs:meta])* name = $type_name:ident, counter = $counter_name:ident $(,)*) => {
        #[doc = concat!(r"Counter for generating [", stringify!($type_name), "]")]
        static $counter_name: ::std::sync::atomic::AtomicUsize = ::std::sync::atomic::AtomicUsize::new(0);

        $(#[$attrs])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub struct $type_name(usize);

        impl $type_name {
            fn new() -> Self {
                Self($counter_name.fetch_add(1, ::std::sync::atomic::Ordering::Relaxed))
            }
        }
    };
}

define_id_type! {
    /// `InputCellId` is a unique identifier for an input cell
    name = InputCellId,
    counter = NEXT_INPUT_CELL_COUNTER,
}

define_id_type! {
    /// `ComputeCellId` is a unique identifier for a compute cell.
    /// Values of type `InputCellId` and `ComputeCellId` should not be mutually assignable,
    /// demonstrated by the following tests:
    ///
    /// ```compile_fail
    /// let mut r = react::Reactor::new();
    /// let input: react::ComputeCellId = r.create_input(111);
    /// ```
    ///
    /// ```compile_fail
    /// let mut r = react::Reactor::new();
    /// let input = r.create_input(111);
    /// let compute: react::InputCellId = r.create_compute(&[react::CellId::Input(input)], |_| 222).unwrap();
    /// ```
    name = ComputeCellId,
    counter = NEXT_COMPUTE_CELL_COUNTER,
}

define_id_type! {
    /// Assigned to added callbacks
    name = CallbackId,
    counter = NEXT_CALLBACK_COUNTER,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CellId {
    Input(InputCellId),
    Compute(ComputeCellId),
}

#[derive(Debug, PartialEq, Eq)]
pub enum RemoveCallbackError {
    NonexistentCell,
    NonexistentCallback,
}

//////////////////////////////////////////////////////////////////////////////// Cell

enum Cell<'cb, T> {
    Input(InputContent<T>),
    Compute(ComputeContent<'cb, T>),
}

impl<'cb, T: Copy> Cell<'cb, T> {
    /// Return cell value
    fn value(&self) -> T {
        match self {
            Cell::Input(content) => content.value,
            Cell::Compute(content) => content.value.unwrap(),
        }
    }

    /// Add a depender (cells that depend on this cell)
    fn add_depended_by(&mut self, cell_id: CellId) {
        match self {
            Cell::Input(content) => &mut content.depended_by,
            Cell::Compute(content) => &mut content.depended_by,
        }
        .insert(cell_id);
    }

    /// Get dependers
    fn get_depended_by(&self) -> impl Iterator<Item = CellId> {
        match self {
            Cell::Input(content) => &content.depended_by,
            Cell::Compute(content) => &content.depended_by,
        }
        .iter()
        .copied()
    }

    #[cfg(not(feature = "react_dyn_get_dependencies"))]
    /// Get dependencies (cells we depend on)
    fn get_dependencies(&self) -> impl Iterator<Item = CellId> + '_
    where
        T: Copy + PartialEq,
    {
        // RPIT can only have one hidden type, but we want to branch on two.
        // Work around this by using static dispatch via enum.
        // Pros (vs trait objects):
        // - Is more performant
        //   - Avoid runtime overhead from trait object vtable lookup
        //   - Avoid heap allocation
        match self {
            Cell::Input(_) => EitherIterator::Empty(iter::empty()),
            Cell::Compute(content) => EitherIterator::Copied(content.get_dependencies()),
        }
    }

    #[cfg(feature = "react_dyn_get_dependencies")]
    /// Get dependencies (cells we depend on)
    fn get_dependencies(&self) -> Box<dyn Iterator<Item = CellId> + '_>
    where
        T: Copy + PartialEq,
    {
        // RPIT can only have one hidden type, but we want to branch on two.
        // Work around this by using trait objects.
        // Pros:
        // - Is simpler to implement (vs static dispatch via enum)
        // - Code has better encapsulation ... can hide more implementation details in lower layers.
        //   We can refactor get_dependencies() impl to be different without
        //   affecting caller signatures.
        match self {
            Cell::Input(_) => Box::new(iter::empty()),
            Cell::Compute(content) => Box::new(content.get_dependencies()),
        }
    }
}

#[cfg(not(feature = "react_dyn_get_dependencies"))]
enum EitherIterator<'a, T> {
    Empty(Empty<T>),
    Copied(Copied<std::slice::Iter<'a, T>>),
}

#[cfg(not(feature = "react_dyn_get_dependencies"))]
impl<'a, T> Iterator for EitherIterator<'a, T>
where
    T: Copy,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            EitherIterator::Empty(wrapped) => wrapped.next(),
            EitherIterator::Copied(wrapped) => wrapped.next(),
        }
    }
}

//////////////////////////////////////////////////////////////////////////////// InputContent

struct InputContent<T> {
    value: T,

    /// IDs of cells that depend on us
    depended_by: HashSet<CellId>,
}

impl<T: Copy + PartialEq> InputContent<T> {
    fn new(value: T) -> Self
    where
        T: Copy + PartialEq,
    {
        Self {
            value,
            depended_by: HashSet::new(),
        }
    }

    fn set_value(&mut self, value: T) -> bool {
        let changed = self.value != value;
        if changed {
            self.value = value;
        }
        changed
    }
}

//////////////////////////////////////////////////////////////////////////////// ComputeContent

struct ComputeContent<'cb, T> {
    value: Option<T>,

    /// IDs of cells that depend on us
    depended_by: HashSet<CellId>,

    /// IDs of cells that we depend on
    dependencies: Vec<CellId>,

    /// Callbacks to notify when value is changed
    callbacks: HashMap<CallbackId, Box<dyn FnMut(T) + 'cb>>,

    /// Formula to apply to generate value from dependencies
    #[expect(clippy::type_complexity)]
    formula: Box<dyn Fn(&[T]) -> T + 'cb>,
}

impl<'cb, T: Copy + PartialEq> ComputeContent<'cb, T> {
    fn new(dependencies: &[CellId], formula: impl Fn(&[T]) -> T + 'cb) -> Self {
        Self {
            value: None, // none is a transient placeholder, recalculate_value() will overwrite this
            depended_by: HashSet::new(),
            callbacks: HashMap::new(),
            dependencies: dependencies.to_vec(),
            formula: Box::new(formula),
        }
    }

    #[cfg(not(feature = "react_dyn_get_dependencies"))]
    fn get_dependencies(&self) -> Copied<std::slice::Iter<'_, CellId>> {
        // NB: Can *not* use RPIT [unlike trait object version below] because
        // the caller needs to name the type [in order to use EitherIterator]
        self.dependencies.iter().copied()
    }

    #[cfg(feature = "react_dyn_get_dependencies")]
    fn get_dependencies(&self) -> impl Iterator<Item = CellId> {
        self.dependencies.iter().copied()
    }

    /// Recalculate value using formula on dependencies
    ///
    /// Returns whether or not refreshed value was changed
    fn recalculate_value(&mut self, dep_values: &[T]) -> bool {
        // Apply formula to recalculate value
        let value = (self.formula)(dep_values);

        let changed = if let Some(old_value) = self.value {
            old_value != value
        } else {
            true
        };

        if changed {
            self.value = Some(value);

            // Notify callbacks of the change
            for callback in self.callbacks.values_mut() {
                (callback)(value);
            }
        }
        changed
    }

    fn add_callback<F>(&mut self, callback: F) -> CallbackId
    where
        F: FnMut(T) + 'cb,
    {
        let callback_id = CallbackId::new();
        self.callbacks.insert(callback_id, Box::new(callback));
        callback_id
    }

    fn remove_callback(&mut self, callback_id: CallbackId) -> bool {
        self.callbacks.remove(&callback_id).is_some()
    }
}

//////////////////////////////////////////////////////////////////////////////// Reactor

#[derive(Default)]
pub struct Reactor<'cb, T> {
    cells: HashMap<CellId, Cell<'cb, T>>,
}

// You are guaranteed that Reactor will only be tested against types that are Copy + PartialEq.
impl<'cb, T> Reactor<'cb, T>
where
    T: Copy + PartialEq,
{
    pub fn new() -> Self {
        Self {
            cells: HashMap::new(),
        }
    }

    // Creates an input cell with the specified initial value, returning its ID.
    pub fn create_input(&mut self, initial: T) -> InputCellId {
        let input_cell_id = InputCellId::new();
        let cell_id = CellId::Input(input_cell_id);

        self.cells
            .insert(cell_id, Cell::Input(InputContent::new(initial)));

        input_cell_id
    }

    // Creates a compute cell with the specified dependencies and compute function.
    // The compute function is expected to take in its arguments in the same order as specified in
    // `dependencies`.
    // You do not need to reject compute functions that expect more arguments than there are
    // dependencies (how would you check for this, anyway?).
    //
    // If any dependency doesn't exist, returns an Err with that nonexistent dependency.
    // (If multiple dependencies do not exist, exactly which one is returned is not defined and
    // will not be tested)
    //
    // Notice that there is no way to *remove* a cell.
    // This means that you may assume, without checking, that if the dependencies exist at creation
    // time they will continue to exist as long as the Reactor exists.
    pub fn create_compute<F>(
        &mut self,
        dependencies: &[CellId],
        formula: F,
    ) -> Result<ComputeCellId, CellId>
    where
        F: Fn(&[T]) -> T + 'cb,
    {
        let compute_cell_id = ComputeCellId::new();
        let new_cell_id = CellId::Compute(compute_cell_id);

        // Reject bad dependencies
        if let Some(bad_cell_id) = dependencies
            .iter()
            .find(|&cell_id| !self.cells.contains_key(cell_id))
        {
            return Err(*bad_cell_id);
        }

        // Make dependencies know we depend on them
        for dependency_id in dependencies {
            self.cells
                .get_mut(dependency_id)
                .unwrap() // We just checked that all dependencies exist, so this cannot fail
                .add_depended_by(new_cell_id);
        }

        let mut content = ComputeContent::new(dependencies, formula);
        content.recalculate_value(&self.get_cell_values(dependencies).collect::<Vec<_>>());

        self.cells.insert(new_cell_id, Cell::Compute(content));

        Ok(compute_cell_id)
    }

    // Retrieves the current value of the cell, or None if the cell does not exist.
    //
    // You may wonder whether it is possible to implement `get(&self, cell_id: CellId) -> Option<&Cell>`
    // and have a `value(&self)` method on `Cell`.
    //
    // It turns out this introduces a significant amount of extra complexity to this exercise.
    // We chose not to cover this here, since this exercise is probably enough work as-is.
    pub fn value(&self, cell_id: CellId) -> Option<T> {
        self.cells.get(&cell_id).map(Cell::value)
    }

    // Sets the value of the specified input cell.
    //
    // Returns false if the cell does not exist.
    //
    // Similarly, you may wonder about `get_mut(&mut self, cell_id: CellId) -> Option<&mut Cell>`, with
    // a `set_value(&mut self, new_value: T)` method on `Cell`.
    //
    // As before, that turned out to add too much extra complexity.
    pub fn set_value(&mut self, input_cell_id: InputCellId, new_value: T) -> bool {
        let cell_id = CellId::Input(input_cell_id);
        let cell = self.cells.get_mut(&cell_id);
        let Some(cell) = cell else {
            return false;
        };
        let Cell::Input(content) = cell else {
            unreachable!()
        };
        if content.set_value(new_value) {
            let mut topo_sorted_ids = self.topo_sort(cell_id);

            // Burn through the starting cell (it's the only one that is not a compute cell but instead an input cell)
            assert_eq!(topo_sorted_ids.next(), Some(cell_id));

            for cell_id in topo_sorted_ids {
                // NB: Get dependency value _before_ current &mut Cell so we don't overlap &self
                // vs &mut self lifetimes ... avoid compiler error
                let dep_values = self.get_dependency_values_for_cell(cell_id);
                let Cell::Compute(content) = self.cells.get_mut(&cell_id).unwrap() else {
                    unreachable!()
                };
                content.recalculate_value(&dep_values);
            }
        }
        true
    }

    fn get_dependency_values_for_cell(&self, cell_id: CellId) -> Vec<T> {
        let cell = self
            .cells
            .get(&cell_id)
            // FIXME Don't panic if cell IDs are invalid
            .unwrap_or_else(|| panic!("cannot get dependencies for non-exist cell {cell_id:?}"));

        // We return collection instead of returning iterator straight up.
        // This is so we can release the &self hold, allowing caller to caller
        // to acquire &mut self for other activity (i.e., improve this method's
        // ergonomics/utility by tightening method to _not_ hold &self reference
        // longer than it has to).
        self.get_cell_values(cell.get_dependencies()).collect()
    }

    fn get_cell_values(
        &self,
        cell_ids: impl IntoIterator<Item: Borrow<CellId>>,
    ) -> impl Iterator<Item = T> {
        cell_ids
            .into_iter()
            // FIXME Don't panic if cell IDs are invalid
            .map(|cell_id| self.cells.get(cell_id.borrow()).unwrap().value())
    }

    /// Return all cells reachable from a cell
    /// flattened + sorted such that all edges go into same direction.
    /// The starting cell is included in the results (as first element.
    ///
    /// Earlier cells will always "points" to later cells.
    /// * aka treat each edge as node must-come-before
    /// * aka reading results from left-to-right, all edges go from left to right
    /// * aka if edge is (depended_on -> depender) => depended_on's will come first
    ///
    /// NB: `use<T>` so that impl Iterator does _not_ unncessary capture
    /// lifetime of &self as would be the default (aka
    /// `use<'self_receiver_lifetime, 'cb, T>`)
    ///
    /// **ATTN**: This method will panic if cell given to method does not exist
    fn topo_sort(&self, cell_id: CellId) -> impl Iterator<Item = CellId> + use<T> {
        // precise capture use<> needed to avoid ret value having 'self lifetime bound
        let mut out = Vec::new();
        let mut visited = HashSet::new();
        self.topo_sort_helper(cell_id, &mut out, &mut visited);
        out.into_iter().rev()
    }

    fn topo_sort_helper(
        &self,
        cell_id: CellId,
        out: &mut Vec<CellId>,
        visited: &mut HashSet<CellId>,
    ) {
        if !visited.insert(cell_id) {
            return;
        }
        let cell = self.cells.get(&cell_id).unwrap();
        for depender_id in cell.get_depended_by() {
            self.topo_sort_helper(depender_id, out, visited);
        }
        // For topo sort, the starting node is accumulated "last"
        visited.insert(cell_id);
        out.push(cell_id);
    }

    // Adds a callback to the specified compute cell.
    //
    // Returns the ID of the just-added callback, or None if the cell doesn't exist.
    //
    // Callbacks on input cells will not be tested.
    //
    // The semantics of callbacks (as will be tested):
    // For a single set_value call, each compute cell's callbacks should each be called:
    // * Zero times if the compute cell's value did not change as a result of the set_value call.
    // * Exactly once if the compute cell's value changed as a result of the set_value call.
    //   The value passed to the callback should be the final value of the compute cell after the
    //   set_value call.
    pub fn add_callback<F>(
        &mut self,
        compute_cell_id: ComputeCellId,
        callback: F,
    ) -> Option<CallbackId>
    where
        F: FnMut(T) + 'cb,
    {
        self.cells
            .get_mut(&CellId::Compute(compute_cell_id))
            .map(|cell| {
                let Cell::Compute(content) = cell else {
                    // ComputeCellId only maps to Cell::Compute
                    unreachable!()
                };
                content.add_callback(callback)
            })
    }

    // Removes the specified callback, using an ID returned from add_callback.
    //
    // Returns an Err if either the cell or callback does not exist.
    //
    // A removed callback should no longer be called.
    pub fn remove_callback(
        &mut self,
        cell_id: ComputeCellId,
        callback_id: CallbackId,
    ) -> Result<(), RemoveCallbackError> {
        let cell = self
            .cells
            .get_mut(&CellId::Compute(cell_id))
            .ok_or(RemoveCallbackError::NonexistentCell)?;

        let Cell::Compute(content) = cell else {
            unreachable!()
        };

        content
            .remove_callback(callback_id)
            .then_some(())
            .ok_or(RemoveCallbackError::NonexistentCallback)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_cells_have_a_value() {
        let mut reactor = Reactor::new();
        let input = reactor.create_input(10);
        assert_eq!(reactor.value(CellId::Input(input)), Some(10));
    }
    #[test]
    fn an_input_cells_value_can_be_set() {
        let mut reactor = Reactor::new();
        let input = reactor.create_input(4);
        assert!(reactor.set_value(input, 20));
        assert_eq!(reactor.value(CellId::Input(input)), Some(20));
    }
    #[test]
    fn error_setting_a_nonexistent_input_cell() {
        let mut dummy_reactor = Reactor::new();
        let input = dummy_reactor.create_input(1);
        assert!(!Reactor::new().set_value(input, 0));
    }
    #[test]
    fn compute_cells_calculate_initial_value() {
        let mut reactor = Reactor::new();
        let input = reactor.create_input(1);
        let output = reactor
            .create_compute(&[CellId::Input(input)], |v| v[0] + 1)
            .unwrap();
        assert_eq!(reactor.value(CellId::Compute(output)), Some(2));
    }
    #[test]
    fn compute_cells_take_inputs_in_the_right_order() {
        let mut reactor = Reactor::new();
        let one = reactor.create_input(1);
        let two = reactor.create_input(2);
        let output = reactor
            .create_compute(&[CellId::Input(one), CellId::Input(two)], |v| {
                v[0] + v[1] * 10
            })
            .unwrap();
        assert_eq!(reactor.value(CellId::Compute(output)), Some(21));
    }
    #[test]
    fn error_creating_compute_cell_if_input_doesnt_exist() {
        let mut dummy_reactor = Reactor::new();
        let input = dummy_reactor.create_input(1);
        assert_eq!(
            Reactor::new().create_compute(&[CellId::Input(input)], |_| 0),
            Err(CellId::Input(input))
        );
    }
    #[test]
    fn do_not_break_cell_if_creating_compute_cell_with_valid_and_invalid_input() {
        let mut dummy_reactor = Reactor::new();
        let _ = dummy_reactor.create_input(1);
        let dummy_cell = dummy_reactor.create_input(2);
        let mut reactor = Reactor::new();
        let input = reactor.create_input(1);
        assert_eq!(
            reactor.create_compute(&[CellId::Input(input), CellId::Input(dummy_cell)], |_| 0),
            Err(CellId::Input(dummy_cell))
        );
        assert!(reactor.set_value(input, 5));
        assert_eq!(reactor.value(CellId::Input(input)), Some(5));
    }
    #[test]
    fn compute_cells_update_value_when_dependencies_are_changed() {
        let mut reactor = Reactor::new();
        let input = reactor.create_input(1);
        let output = reactor
            .create_compute(&[CellId::Input(input)], |v| v[0] + 1)
            .unwrap();
        assert_eq!(reactor.value(CellId::Compute(output)), Some(2));
        assert!(reactor.set_value(input, 3));
        assert_eq!(reactor.value(CellId::Compute(output)), Some(4));
    }
    #[test]
    fn compute_cells_can_depend_on_other_compute_cells() {
        let mut reactor = Reactor::new();
        let input = reactor.create_input(1);
        let times_two = reactor
            .create_compute(&[CellId::Input(input)], |v| v[0] * 2)
            .unwrap();
        let times_thirty = reactor
            .create_compute(&[CellId::Input(input)], |v| v[0] * 30)
            .unwrap();
        let output = reactor
            .create_compute(
                &[CellId::Compute(times_two), CellId::Compute(times_thirty)],
                |v| v[0] + v[1],
            )
            .unwrap();
        assert_eq!(reactor.value(CellId::Compute(output)), Some(32));
        assert!(reactor.set_value(input, 3));
        assert_eq!(reactor.value(CellId::Compute(output)), Some(96));
    }
    /// A CallbackRecorder helps tests whether callbacks get called correctly.
    /// You'll see it used in tests that deal with callbacks.
    /// The names should be descriptive enough so that the tests make sense,
    /// so it's not necessary to fully understand the implementation,
    /// though you are welcome to.
    struct CallbackRecorder {
        // Note that this `Cell` is https://doc.rust-lang.org/std/cell/
        // a mechanism to allow internal mutability,
        // distinct from the cells (input cells, compute cells) in the reactor
        value: std::cell::Cell<Option<i32>>,
    }
    impl CallbackRecorder {
        fn new() -> Self {
            CallbackRecorder {
                value: std::cell::Cell::new(None),
            }
        }
        fn expect_to_have_been_called_with(&self, v: i32) {
            assert_ne!(
                self.value.get(),
                None,
                "Callback was not called, but should have been"
            );
            assert_eq!(
                self.value.replace(None),
                Some(v),
                "Callback was called with incorrect value"
            );
        }
        fn expect_not_to_have_been_called(&self) {
            assert_eq!(
                self.value.get(),
                None,
                "Callback was called, but should not have been"
            );
        }
        fn callback_called(&self, v: i32) {
            assert_eq!(
                self.value.replace(Some(v)),
                None,
                "Callback was called too many times; can't be called with {v}"
            );
        }
    }
    #[test]
    fn compute_cells_fire_callbacks() {
        let cb = CallbackRecorder::new();
        let mut reactor = Reactor::new();
        let input = reactor.create_input(1);
        let output = reactor
            .create_compute(&[CellId::Input(input)], |v| v[0] + 1)
            .unwrap();
        assert!(
            reactor
                .add_callback(output, |v| cb.callback_called(v))
                .is_some()
        );
        assert!(reactor.set_value(input, 3));
        cb.expect_to_have_been_called_with(4);
    }
    #[test]
    fn error_adding_callback_to_nonexistent_cell() {
        let mut dummy_reactor = Reactor::new();
        let input = dummy_reactor.create_input(1);
        let output = dummy_reactor
            .create_compute(&[CellId::Input(input)], |_| 0)
            .unwrap();
        assert_eq!(
            Reactor::new().add_callback(output, |_: u32| println!("hi")),
            None
        );
    }
    #[test]
    fn error_removing_callback_from_nonexisting_cell() {
        let mut dummy_reactor = Reactor::new();
        let dummy_input = dummy_reactor.create_input(1);
        let _ = dummy_reactor
            .create_compute(&[CellId::Input(dummy_input)], |_| 0)
            .unwrap();
        let dummy_output = dummy_reactor
            .create_compute(&[CellId::Input(dummy_input)], |_| 0)
            .unwrap();
        let mut reactor = Reactor::new();
        let input = reactor.create_input(1);
        let output = reactor
            .create_compute(&[CellId::Input(input)], |_| 0)
            .unwrap();
        let callback = reactor.add_callback(output, |_| ()).unwrap();
        assert_eq!(
            reactor.remove_callback(dummy_output, callback),
            Err(RemoveCallbackError::NonexistentCell)
        );
    }
    #[test]
    fn callbacks_only_fire_on_change() {
        let cb = CallbackRecorder::new();
        let mut reactor = Reactor::new();
        let input = reactor.create_input(1);
        let output = reactor
            .create_compute(
                &[CellId::Input(input)],
                |v| if v[0] < 3 { 111 } else { 222 },
            )
            .unwrap();
        assert!(
            reactor
                .add_callback(output, |v| cb.callback_called(v))
                .is_some()
        );
        assert!(reactor.set_value(input, 2));
        cb.expect_not_to_have_been_called();
        assert!(reactor.set_value(input, 4));
        cb.expect_to_have_been_called_with(222);
    }
    #[test]
    fn callbacks_can_be_called_multiple_times() {
        let cb = CallbackRecorder::new();
        let mut reactor = Reactor::new();
        let input = reactor.create_input(1);
        let output = reactor
            .create_compute(&[CellId::Input(input)], |v| v[0] + 1)
            .unwrap();
        assert!(
            reactor
                .add_callback(output, |v| cb.callback_called(v))
                .is_some()
        );
        assert!(reactor.set_value(input, 2));
        cb.expect_to_have_been_called_with(3);
        assert!(reactor.set_value(input, 3));
        cb.expect_to_have_been_called_with(4);
    }
    #[test]
    fn callbacks_can_be_called_from_multiple_cells() {
        let cb1 = CallbackRecorder::new();
        let cb2 = CallbackRecorder::new();
        let mut reactor = Reactor::new();
        let input = reactor.create_input(1);
        let plus_one = reactor
            .create_compute(&[CellId::Input(input)], |v| v[0] + 1)
            .unwrap();
        let minus_one = reactor
            .create_compute(&[CellId::Input(input)], |v| v[0] - 1)
            .unwrap();
        assert!(
            reactor
                .add_callback(plus_one, |v| cb1.callback_called(v))
                .is_some()
        );
        assert!(
            reactor
                .add_callback(minus_one, |v| cb2.callback_called(v))
                .is_some()
        );
        assert!(reactor.set_value(input, 10));
        cb1.expect_to_have_been_called_with(11);
        cb2.expect_to_have_been_called_with(9);
    }
    #[test]
    fn callbacks_can_be_added_and_removed() {
        let cb1 = CallbackRecorder::new();
        let cb2 = CallbackRecorder::new();
        let cb3 = CallbackRecorder::new();
        let mut reactor = Reactor::new();
        let input = reactor.create_input(11);
        let output = reactor
            .create_compute(&[CellId::Input(input)], |v| v[0] + 1)
            .unwrap();
        let callback = reactor
            .add_callback(output, |v| cb1.callback_called(v))
            .unwrap();
        assert!(
            reactor
                .add_callback(output, |v| cb2.callback_called(v))
                .is_some()
        );
        assert!(reactor.set_value(input, 31));
        cb1.expect_to_have_been_called_with(32);
        cb2.expect_to_have_been_called_with(32);
        assert!(reactor.remove_callback(output, callback).is_ok());
        assert!(
            reactor
                .add_callback(output, |v| cb3.callback_called(v))
                .is_some()
        );
        assert!(reactor.set_value(input, 41));
        cb1.expect_not_to_have_been_called();
        cb2.expect_to_have_been_called_with(42);
        cb3.expect_to_have_been_called_with(42);
    }
    #[test]
    fn removing_a_callback_multiple_times_doesnt_interfere_with_other_callbacks() {
        let cb1 = CallbackRecorder::new();
        let cb2 = CallbackRecorder::new();
        let mut reactor = Reactor::new();
        let input = reactor.create_input(1);
        let output = reactor
            .create_compute(&[CellId::Input(input)], |v| v[0] + 1)
            .unwrap();
        let callback = reactor
            .add_callback(output, |v| cb1.callback_called(v))
            .unwrap();
        assert!(
            reactor
                .add_callback(output, |v| cb2.callback_called(v))
                .is_some()
        );
        // We want the first remove to be Ok, but the others should be errors.
        assert!(reactor.remove_callback(output, callback).is_ok());
        for _ in 1..5 {
            assert_eq!(
                reactor.remove_callback(output, callback),
                Err(RemoveCallbackError::NonexistentCallback)
            );
        }
        assert!(reactor.set_value(input, 2));
        cb1.expect_not_to_have_been_called();
        cb2.expect_to_have_been_called_with(3);
    }
    #[test]
    fn callbacks_should_only_be_called_once_even_if_multiple_dependencies_change() {
        let cb = CallbackRecorder::new();
        let mut reactor = Reactor::new();
        let input = reactor.create_input(1);
        let plus_one = reactor
            .create_compute(&[CellId::Input(input)], |v| v[0] + 1)
            .unwrap();
        let minus_one1 = reactor
            .create_compute(&[CellId::Input(input)], |v| v[0] - 1)
            .unwrap();
        let minus_one2 = reactor
            .create_compute(&[CellId::Compute(minus_one1)], |v| v[0] - 1)
            .unwrap();
        let output = reactor
            .create_compute(
                &[CellId::Compute(plus_one), CellId::Compute(minus_one2)],
                |v| v[0] * v[1],
            )
            .unwrap();
        assert!(
            reactor
                .add_callback(output, |v| cb.callback_called(v))
                .is_some()
        );
        assert!(reactor.set_value(input, 4));
        cb.expect_to_have_been_called_with(10);
    }
    #[test]
    fn callbacks_should_not_be_called_if_dependencies_change_but_output_value_doesnt_change() {
        let cb = CallbackRecorder::new();
        let mut reactor = Reactor::new();
        let input = reactor.create_input(1);
        let plus_one = reactor
            .create_compute(&[CellId::Input(input)], |v| v[0] + 1)
            .unwrap();
        let minus_one = reactor
            .create_compute(&[CellId::Input(input)], |v| v[0] - 1)
            .unwrap();
        let always_two = reactor
            .create_compute(
                &[CellId::Compute(plus_one), CellId::Compute(minus_one)],
                |v| v[0] - v[1],
            )
            .unwrap();
        assert!(
            reactor
                .add_callback(always_two, |v| cb.callback_called(v))
                .is_some()
        );
        for i in 2..5 {
            assert!(reactor.set_value(input, i));
            cb.expect_not_to_have_been_called();
        }
    }
    #[test]
    fn adder_with_boolean_values() {
        // This is a digital logic circuit called an adder:
        // https://en.wikipedia.org/wiki/Adder_(electronics)
        let mut reactor = Reactor::new();
        let a = reactor.create_input(false);
        let b = reactor.create_input(false);
        let carry_in = reactor.create_input(false);
        let a_xor_b = reactor
            .create_compute(&[CellId::Input(a), CellId::Input(b)], |v| v[0] ^ v[1])
            .unwrap();
        let sum = reactor
            .create_compute(&[CellId::Compute(a_xor_b), CellId::Input(carry_in)], |v| {
                v[0] ^ v[1]
            })
            .unwrap();
        let a_xor_b_and_cin = reactor
            .create_compute(&[CellId::Compute(a_xor_b), CellId::Input(carry_in)], |v| {
                v[0] && v[1]
            })
            .unwrap();
        let a_and_b = reactor
            .create_compute(&[CellId::Input(a), CellId::Input(b)], |v| v[0] && v[1])
            .unwrap();
        let carry_out = reactor
            .create_compute(
                &[CellId::Compute(a_xor_b_and_cin), CellId::Compute(a_and_b)],
                |v| v[0] || v[1],
            )
            .unwrap();
        let tests = &[
            (false, false, false, false, false),
            (false, false, true, false, true),
            (false, true, false, false, true),
            (false, true, true, true, false),
            (true, false, false, false, true),
            (true, false, true, true, false),
            (true, true, false, true, false),
            (true, true, true, true, true),
        ];
        for &(aval, bval, cinval, expected_cout, expected_sum) in tests {
            assert!(reactor.set_value(a, aval));
            assert!(reactor.set_value(b, bval));
            assert!(reactor.set_value(carry_in, cinval));
            assert_eq!(reactor.value(CellId::Compute(sum)), Some(expected_sum));
            assert_eq!(
                reactor.value(CellId::Compute(carry_out)),
                Some(expected_cout)
            );
        }
    }
}
