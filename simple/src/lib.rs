pub mod anon_lifetime;
pub mod box_dyn_is_static;
pub mod generic_implicit_sized;
pub mod too_many_lists;

#[cfg(test)]
mod tests {
    use std::time::Duration;

    static DELAY: Duration = Duration::from_secs(3);

    #[test]
    fn test_par1() {
        std::thread::sleep(DELAY);
    }

    #[test]
    fn test_par2() {
        std::thread::sleep(DELAY);
    }

    #[test]
    fn test_par3() {
        std::thread::sleep(DELAY);
    }

    #[test]
    fn test_par4() {
        std::thread::sleep(DELAY);
    }

    #[test]
    fn test_par5_ub() {
        unsafe {
            let mut x = 42;
            let mut1 = &mut x;
            let ptr2 = mut1 as *mut _;
            let mut3 = &mut *ptr2;

            // TODO Uncomment below to have miri flag Undefined Behavior (UB)
            // *ptr2 += 1;

            *mut3 += 1;
            *ptr2 += 1;
            *mut1 += 1;

            dbg!(x);
        }
    }
}
