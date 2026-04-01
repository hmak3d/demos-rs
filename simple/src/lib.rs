pub mod anon_lifetime;
pub mod box_dyn_is_static;
pub mod generic_implicit_sized;
pub mod to_ub_or_not_ub;
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
}
