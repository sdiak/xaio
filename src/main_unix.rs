use xaio::{Driver, DriverConfig, DriverIFace, DriverKind, RequestList};

pub fn main() {
    println!("Hello unix");
    let config = DriverConfig::default();
    let mut driver = Driver::new(DriverKind::EPoll, &config).unwrap();
    println!("{driver:?}");
    let mut ready = RequestList::new();
    driver.wake();
    (*driver).wait(-1, &mut ready).unwrap();
}
