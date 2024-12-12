use std::io::{Read, Write};
use xaio::{Driver, DriverConfig, DriverIFace, DriverKind, RequestList};

pub fn main() {
    println!("Hello unix");
    let config = DriverConfig::default();
    let mut driver = Driver::new(DriverKind::EPoll, &config).unwrap();
    println!("{driver:?}");
    let mut ready = RequestList::new();
    driver.wake();
    (*driver).wait(-1, &mut ready).unwrap();

    let (mut a, mut b) = xaio::socketpair(socket2::Type::STREAM, None).unwrap();
    println!("({a:?}, {b:?})");

    let msg = ['a' as u8, 'b' as u8, 'c' as u8, 'd' as u8];
    println!(
        "A wrote {} bytes: {:?}",
        a.write(&msg).unwrap(),
        std::str::from_utf8(&msg).unwrap()
    );

    let mut msg = ['a' as u8, 'a' as u8, 'a' as u8, 'a' as u8];
    println!(
        "B read {} bytes: {:?}",
        b.read(&mut msg).unwrap(),
        std::str::from_utf8(&msg).unwrap()
    );

    let msg = ['0' as u8, '1' as u8, '2' as u8, '3' as u8];
    println!(
        "B wrote {} bytes: {:?}",
        b.write(&msg).unwrap(),
        std::str::from_utf8(&msg).unwrap()
    );

    let mut msg = ['a' as u8, 'a' as u8, 'a' as u8, 'a' as u8];
    println!(
        "A read {} bytes: {:?}",
        a.read(&mut msg).unwrap(),
        std::str::from_utf8(&msg).unwrap()
    );

    // println!(
    //     "a:{:?}, b:{:?}",
    //     a.as_socket().as_raw(),
    //     b.as_socket().as_raw()
    // );
}
