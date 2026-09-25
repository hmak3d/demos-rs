//! re: <https://exercism.org/tracks/rust/exercises/react/edit>

use std::cell::RefCell;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_CELL_ID: AtomicUsize = AtomicUsize::new(0);

/// `InputCellId` is a unique identifier for an input cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct InputCellId(usize);

impl InputCellId {
    fn new() -> Self {
        Self(NEXT_CELL_ID.fetch_add(1, Ordering::Relaxed))
    }
}

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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ComputeCellId(usize);

impl ComputeCellId {
    fn new() -> Self {
        Self(NEXT_CELL_ID.fetch_add(1, Ordering::Relaxed))
    }
}

static NEXT_CALLBACK_ID: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CallbackId(usize);

impl CallbackId {
    fn new() -> Self {
        Self(NEXT_CALLBACK_ID.fetch_add(1, Ordering::Relaxed))
    }
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
    fn value(&self) -> T {
        match self {
            Cell::Input(content) => content.value.get(),
            Cell::Compute(content) => content.value.get().unwrap(),
        }
    }

    fn add_depended_by(&mut self, cell_id: CellId) {
        match self {
            Cell::Input(content) => &mut content.depended_by,
            Cell::Compute(content) => &mut content.depended_by,
        }
        .insert(cell_id);
    }

    fn get_depended_by(&self) -> impl Iterator<Item = CellId> {
        match self {
            Cell::Input(content) => &content.depended_by,
            Cell::Compute(content) => &content.depended_by,
        }
        .iter()
        .copied()
    }
}

//////////////////////////////////////////////////////////////////////////////// InputContent

struct InputContent<T> {
    value: std::cell::Cell<T>,
    depended_by: HashSet<CellId>,
}

impl<T: Copy + PartialEq> InputContent<T> {
    fn new(value: T) -> Self
    where
        T: Copy + PartialEq,
    {
        Self {
            value: std::cell::Cell::new(value),
            depended_by: HashSet::new(),
        }
    }

    fn set_value(&self, value: T) -> bool {
        let changed = self.value.get() != value;
        if changed {
            self.value.set(value);
        }
        changed
    }
}

//////////////////////////////////////////////////////////////////////////////// ComputeContent

struct ComputeContent<'cb, T> {
    id: ComputeCellId,
    value: std::cell::Cell<Option<T>>,
    depended_by: HashSet<CellId>,
    callbacks: RefCell<HashMap<CallbackId, Box<dyn FnMut(T) + 'cb>>>,
    dependencies: RefCell<Vec<CellId>>,
    #[expect(clippy::type_complexity)]
    formula: Box<dyn Fn(&[T]) -> T>,
}

impl<'cb, T: Copy + PartialEq> ComputeContent<'cb, T> {
    fn new(
        id: ComputeCellId,
        dependencies: &[CellId],
        formula: impl Fn(&[T]) -> T + 'static,
    ) -> Self {
        Self {
            id,
            value: std::cell::Cell::new(None),
            depended_by: HashSet::new(),
            callbacks: RefCell::new(HashMap::new()),
            dependencies: RefCell::new(dependencies.to_vec()),
            formula: Box::new(formula),
        }
    }

    /// Returns whether or not refreshed value was changed
    fn refresh(&self, reactor: &Reactor<T>, visited_cell_ids: &mut HashSet<CellId>) -> bool {
        let my_id = CellId::Compute(self.id);

        if !visited_cell_ids.insert(my_id) {
            // already visited => so nothing new was changed
            return false;
        }

        // Get dependency values
        let dep_values = self
            .dependencies
            .borrow()
            .iter()
            .map(|id| {
                reactor
                    .cells
                    .get(id)
                    .map(|cell| cell.value())
                    // Dependencies are checked when we create cells.
                    // We never delete cells.
                    // => The dependencies should always be valid
                    .expect("invalid dependency")
            })
            .collect::<Vec<T>>();

        // Use formula to recalculate value
        let value = (self.formula)(&dep_values);

        let changed = if let Some(old_value) = self.value.get() {
            old_value != value
        } else {
            true
        };

        if changed {
            self.value.set(Some(value));

            // Notify callbacks of the change
            for callback in self.callbacks.borrow_mut().values_mut() {
                (callback)(value);
            }
        }
        changed
    }

    fn add_callback<F>(&self, callback: F) -> CallbackId
    where
        F: FnMut(T) + 'cb,
    {
        let callback_id = CallbackId::new();
        self.callbacks
            .borrow_mut()
            .insert(callback_id, Box::new(callback));
        callback_id
    }

    fn remove_callback(&self, callback_id: CallbackId) -> bool {
        self.callbacks.borrow_mut().remove(&callback_id).is_some()
    }
}

//////////////////////////////////////////////////////////////////////////////// ComputeContent

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
        F: Fn(&[T]) -> T + 'static,
    {
        let compute_cell_id = ComputeCellId::new();
        let cell_id = CellId::Compute(compute_cell_id);

        for dependency_id in dependencies {
            let cell = self.cells.get_mut(dependency_id).ok_or(*dependency_id)?;
            cell.add_depended_by(cell_id);
        }

        let content = ComputeContent::new(compute_cell_id, dependencies, formula);
        content.refresh(self, &mut HashSet::new());

        self.cells.insert(cell_id, Cell::Compute(content));

        Ok(compute_cell_id)
    }

    // Retrieves the current value of the cell, or None if the cell does not exist.
    //
    // You may wonder whether it is possible to implement `get(&self, id: CellId) -> Option<&Cell>`
    // and have a `value(&self)` method on `Cell`.
    //
    // It turns out this introduces a significant amount of extra complexity to this exercise.
    // We chose not to cover this here, since this exercise is probably enough work as-is.
    pub fn value(&self, id: CellId) -> Option<T> {
        self.cells.get(&id).map(|cell| cell.value())
    }

    // Sets the value of the specified input cell.
    //
    // Returns false if the cell does not exist.
    //
    // Similarly, you may wonder about `get_mut(&mut self, id: CellId) -> Option<&mut Cell>`, with
    // a `set_value(&mut self, new_value: T)` method on `Cell`.
    //
    // As before, that turned out to add too much extra complexity.
    pub fn set_value(&mut self, id: InputCellId, new_value: T) -> bool {
        let mut depender_cell_ids: VecDeque<CellId> = VecDeque::new();

        if let Some(cell) = self.cells.get(&CellId::Input(id)) {
            // Set cell value
            if let Cell::Input(content) = cell {
                if content.set_value(new_value) {
                    depender_cell_ids.extend(cell.get_depended_by());
                }
            } else {
                return false;
            }
        } else {
            return false;
        };

        // Do BFS on tree where cells point to their dependers (i.e., cells
        // whose formula depend on them).
        // Do *not* do DFS as we want to update "furthest" cells last. i.e.,
        // (d) after (b) + (c)
        // a -> b -> d
        // |         ^
        // \--> c ---/
        // where the arrows are "reversed" (i.e., point from dependency to depender)
        let mut visited_ids = HashSet::new();
        while let Some(id) = depender_cell_ids.pop_front() {
            // Dependency cell ID always valid because we never delete cells
            let cell = self.cells.get(&id).unwrap();
            match cell {
                Cell::Input(_) => unreachable!(),
                Cell::Compute(content) => {
                    if content.refresh(self, &mut visited_ids) {
                        // recursively propagate changes to dependers
                        depender_cell_ids.extend(cell.get_depended_by());
                    }
                    // else don't propagate changes if current value hasn't changed
                }
            }
        }

        true
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
    pub fn add_callback<F>(&mut self, id: ComputeCellId, callback: F) -> Option<CallbackId>
    where
        F: FnMut(T) + 'cb,
    {
        self.cells.get_mut(&CellId::Compute(id)).map(|cell| {
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
