//! https://exercism.org/tracks/rust/exercises/triangle/edit

use std::ops::Add;

pub struct Triangle<T>([T; 3]);

impl<T> Triangle<T>
where
    T: PartialOrd + Add<Output = T> + Copy + PartialEq,
{
    pub fn build(mut sides: [T; 3]) -> Option<Triangle<T>> {
        if sides
            .iter()
            .enumerate()
            .all(|(i, side)| *side < (sides[(i + 1) % sides.len()] + sides[(i + 2) % sides.len()]))
        {
            // Sort sides to speed up is_*() calls.
            // Can't use normal sort() due to f64 note being full Ord.
            sides.sort_unstable_by(|a, b| a.partial_cmp(b).expect("NaN cannot be a side"));
            Some(Triangle(sides))
        } else {
            None
        }
    }

    pub fn is_equilateral(&self) -> bool {
        // Every side is equal to all other sides.
        // Since sides are sorted, only first/last needs to be compared.
        self.0[0] == self.0[2]
    }

    pub fn is_scalene(&self) -> bool {
        self.0
            .iter()
            .enumerate()
            .skip(1)
            // Every side is diff from all other sides
            .all(|(i, side)| *side != self.0[i - 1])
    }

    pub fn is_isosceles(&self) -> bool {
        // At least 2 sides are equal.
        // Equilateral is superset of isosceles.
        !self.is_scalene()
    }
}

#[cfg(test)]
mod tests {
    mod equilateral {
        use super::super::Triangle;
        #[test]
        fn all_sides_are_equal() {
            let input = [2, 2, 2];
            let output = Triangle::build(input).unwrap();
            assert!(output.is_equilateral());
        }
        #[test]
        fn any_side_is_unequal() {
            let input = [2, 3, 2];
            let output = Triangle::build(input).unwrap();
            assert!(!output.is_equilateral());
        }
        #[test]
        fn no_sides_are_equal() {
            let input = [5, 4, 6];
            let output = Triangle::build(input).unwrap();
            assert!(!output.is_equilateral());
        }
        #[test]
        fn sides_may_be_floats() {
            let input = [0.5, 0.5, 0.5];
            let output = Triangle::build(input).unwrap();
            assert!(output.is_equilateral());
        }
    }
    mod isosceles {
        use super::super::Triangle;
        #[test]
        fn last_two_sides_are_equal() {
            let input = [3, 4, 4];
            let output = Triangle::build(input).unwrap();
            assert!(output.is_isosceles());
        }
        #[test]
        fn first_two_sides_are_equal() {
            let input = [4, 4, 3];
            let output = Triangle::build(input).unwrap();
            assert!(output.is_isosceles());
        }
        #[test]
        fn first_and_last_sides_are_equal() {
            let input = [4, 3, 4];
            let output = Triangle::build(input).unwrap();
            assert!(output.is_isosceles());
        }
        #[test]
        fn equilateral_triangles_are_also_isosceles() {
            let input = [4, 4, 4];
            let output = Triangle::build(input).unwrap();
            assert!(output.is_isosceles());
        }
        #[test]
        fn no_sides_are_equal() {
            let input = [2, 3, 4];
            let output = Triangle::build(input).unwrap();
            assert!(!output.is_isosceles());
        }
        #[test]
        fn sides_may_be_floats() {
            let input = [0.5, 0.4, 0.5];
            let output = Triangle::build(input).unwrap();
            assert!(output.is_isosceles());
        }
    }
    mod scalene {
        use super::super::Triangle;
        #[test]
        fn no_sides_are_equal() {
            let input = [5, 4, 6];
            let output = Triangle::build(input).unwrap();
            assert!(output.is_scalene());
        }
        #[test]
        fn all_sides_are_equal() {
            let input = [4, 4, 4];
            let output = Triangle::build(input).unwrap();
            assert!(!output.is_scalene());
        }
        #[test]
        fn first_and_second_sides_are_equal() {
            let input = [4, 4, 3];
            let output = Triangle::build(input).unwrap();
            assert!(!output.is_scalene());
        }
        #[test]
        fn first_and_third_sides_are_equal() {
            let input = [3, 4, 3];
            let output = Triangle::build(input).unwrap();
            assert!(!output.is_scalene());
        }
        #[test]
        fn second_and_third_sides_are_equal() {
            let input = [4, 3, 3];
            let output = Triangle::build(input).unwrap();
            assert!(!output.is_scalene());
        }
        #[test]
        fn sides_may_be_floats() {
            let input = [0.5, 0.4, 0.6];
            let output = Triangle::build(input).unwrap();
            assert!(output.is_scalene());
        }
    }

    mod tests2 {
        use super::super::Triangle;

        #[test]
        fn all_zero_sides_is_not_a_triangle() {
            let input = [0, 0, 0];
            let output = Triangle::build(input);
            assert!(output.is_none());
        }
        #[test]
        fn first_triangle_inequality_violation() {
            let input = [1, 1, 3];
            let output = Triangle::build(input);
            assert!(output.is_none());
        }
        #[test]
        fn second_triangle_inequality_violation() {
            let input = [1, 3, 1];
            let output = Triangle::build(input);
            assert!(output.is_none());
        }
        #[test]
        fn third_triangle_inequality_violation() {
            let input = [3, 1, 1];
            let output = Triangle::build(input);
            assert!(output.is_none());
        }
        #[test]
        fn may_not_violate_triangle_inequality() {
            let input = [7, 3, 2];
            let output = Triangle::build(input);
            assert!(output.is_none());
        }
    }
}
