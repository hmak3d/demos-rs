//! re: https://exercism.org/tracks/rust/exercises/robot-name/edit

use rand::{Rng, RngExt};
use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

/// A `RobotFactory` is responsible for ensuring that all robots produced by
/// it have a unique name. Robots from different factories can have the same
/// name.
pub struct RobotFactory {
    assigned_names: Rc<RefCell<AssignedNames>>,
}

pub struct Robot {
    assigned_names: Rc<RefCell<AssignedNames>>,
    name: String,
}

struct AssignedNames {
    names: HashSet<String>,
}

impl AssignedNames {
    /// Generate new unique name
    fn generate_name<R: Rng>(&mut self, rng: &mut R) -> String {
        let s = loop {
            let s: String = (0..5)
                .map(|i| match i {
                    0..2 => rng.random_range('A'..='Z'),
                    _ => rng.random_range('0'..='9'),
                })
                .collect();
            if !self.names.contains(&s) {
                break s;
            }
        };
        self.names.insert(s.clone());
        s
    }

    /// Remove/retire a generated name
    fn remove_name(&mut self, old_name: &str) {
        self.names.remove(old_name);
    }

    fn new() -> Self {
        Self {
            names: HashSet::new(),
        }
    }
}

impl RobotFactory {
    #[expect(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            assigned_names: Rc::new(RefCell::new(AssignedNames::new())),
        }
    }

    pub fn new_robot<R: Rng>(&mut self, rng: &mut R) -> Robot {
        Robot {
            assigned_names: Rc::clone(&self.assigned_names),
            name: self.assigned_names.borrow_mut().generate_name(rng),
        }
    }
}

impl Robot {
    pub fn name(&self) -> &str {
        self.name.as_ref()
    }

    pub fn reset<R: Rng>(&mut self, rng: &mut R) {
        let mut assigned_names = self.assigned_names.borrow_mut();
        assigned_names.remove_name(&self.name);
        self.name = assigned_names.generate_name(rng);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng as _;
    use rand::rngs::SmallRng;

    fn deterministic_rng() -> SmallRng {
        SmallRng::seed_from_u64(0)
    }
    fn assert_name_matches_pattern(n: &str) {
        assert!(n.len() == 5, "name is exactly 5 characters long");
        assert!(
            n[0..2].chars().all(|c| c.is_ascii_uppercase()),
            "name starts with 2 uppercase letters"
        );
        assert!(
            n[2..].chars().all(|c| c.is_ascii_digit()),
            "name ends with 3 numbers"
        );
    }
    #[test]
    fn name_should_match_expected_pattern() {
        let mut rng = deterministic_rng();
        let mut factory = RobotFactory::new();
        let robot = factory.new_robot(&mut rng);
        assert_name_matches_pattern(robot.name());
    }
    #[test]
    fn robot_name_depends_on_rng() {
        let mut rng = deterministic_rng();
        let robot_1 = RobotFactory::new().new_robot(&mut rng);
        let robot_2 = RobotFactory::new().new_robot(&mut rng);
        assert_ne!(robot_1.name(), robot_2.name());
    }
    #[test]
    fn robot_name_only_depends_on_rng() {
        let robot_1 = RobotFactory::new().new_robot(&mut deterministic_rng());
        let robot_2 = RobotFactory::new().new_robot(&mut deterministic_rng());
        assert_eq!(robot_1.name(), robot_2.name());
    }
    #[test]
    fn factory_prevents_name_collisions() {
        let mut factory = RobotFactory::new();
        let robot_1 = factory.new_robot(&mut deterministic_rng());
        let robot_2 = factory.new_robot(&mut deterministic_rng());
        assert_ne!(robot_1.name(), robot_2.name());
    }
    #[test]
    fn new_name_should_match_expected_pattern() {
        let mut rng = deterministic_rng();
        let mut factory = RobotFactory::new();
        let mut robot = factory.new_robot(&mut rng);
        assert_name_matches_pattern(robot.name());
        robot.reset(&mut rng);
        assert_name_matches_pattern(robot.name());
    }
    #[test]
    fn new_name_is_different_from_old_name() {
        let mut rng = deterministic_rng();
        let mut factory = RobotFactory::new();
        let mut robot = factory.new_robot(&mut rng);
        let name_1 = robot.name().to_string();
        robot.reset(&mut rng);
        let name_2 = robot.name().to_string();
        assert_ne!(name_1, name_2, "Robot name should change when reset");
    }
    #[test]
    fn factory_prevents_name_collision_despite_reset() {
        let mut factory = RobotFactory::new();
        let mut rng = deterministic_rng();
        let mut robot_1 = factory.new_robot(&mut rng);
        robot_1.reset(&mut rng);
        let mut rng = deterministic_rng();
        let mut robot_2 = factory.new_robot(&mut rng);
        robot_2.reset(&mut rng);
        assert_ne!(robot_1.name(), robot_2.name());
    }
    #[test]
    fn old_name_becomes_available_after_reset() {
        let mut rng = deterministic_rng();
        let mut factory = RobotFactory::new();
        let mut robot = factory.new_robot(&mut rng);
        let first_name = robot.name().to_string();
        robot.reset(&mut rng); // cause first name to become available again
        let mut rng = deterministic_rng(); // reset rng
        let second_robot = factory.new_robot(&mut rng);
        assert_eq!(second_robot.name(), first_name);
    }
}
