mod capi;
mod driver;
mod driver_none;
mod selector;
mod socket;
mod sub;
mod sub_list;
pub use driver::*;
pub use driver_none::*;
pub use socket::RawSocketFd;
pub use sub::*;
pub use sub_list::*;

#[cfg(target_os = "linux")]
mod driver_epoll;

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        // println!("Hello\n");
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
