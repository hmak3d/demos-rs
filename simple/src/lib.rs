pub mod anon_lifetime;
pub mod box_dyn_is_static;
pub mod generic_implicit_sized;
pub mod linked_list;
pub mod to_ub_or_not_ub;
pub mod too_many_lists;

#[cfg(test)]
mod tests {
    use std::time::Duration;

    static DELAY: Duration = Duration::from_secs(3);

    #[test]
    fn test_par1() {
        println!("test_par1 start");
        std::thread::sleep(DELAY);
        println!("test_par1 end");
    }

    #[test]
    fn test_par2() {
        println!("test_par2 start");
        std::thread::sleep(DELAY);
        println!("test_par2 end");
    }

    #[test]
    fn test_par3() {
        panic!("bad");
        println!("test_par3 start");
        std::thread::sleep(DELAY);
        println!("test_par3 end");
    }

    #[test]
    fn test_par4() {
        eprintln!("test_par4 start");
        std::thread::sleep(DELAY);
        eprintln!("test_par4 end");
    }
}
