use xaio::{Driver, DriverConfig, DriverIFace, DriverKind, Request, RequestList};
// use xaio::thread_pool::s

pub fn main() {
    println!("Hello unix");
    println!(
        "sys::current_thread(): {:?}",
        xaio::sys::get_current_thread_id()
    );
    let config = DriverConfig::default();
    let mut driver = Driver::new(DriverKind::EPoll, &config).unwrap();
    println!("{driver:?}");
    let mut ready = RequestList::new();
    driver.wake().unwrap();
    (*driver).wait(&mut ready, -1).unwrap();

    let mut req: Request = Request::default();
    let sub = unsafe { std::ptr::NonNull::new_unchecked(&mut req as *mut Request) };
    let sub2 = sub;
    println!("{sub:?}/{sub2:?}");
    // unsafe { driver.submit(sub).unwrap() };
    // let (mut a, mut b) = xaio::socketpair(socket2::Type::STREAM, None).unwrap();
    // println!("({a:?}, {b:?})");

    // let msg = ['a' as u8, 'b' as u8, 'c' as u8, 'd' as u8];
    // println!(
    //     "A wrote {} bytes: {:?}",
    //     a.write(&msg).unwrap(),
    //     std::str::from_utf8(&msg).unwrap()
    // );

    // let mut msg = ['a' as u8, 'a' as u8, 'a' as u8, 'a' as u8];
    // println!(
    //     "B read {} bytes: {:?}",
    //     b.read(&mut msg).unwrap(),
    //     std::str::from_utf8(&msg).unwrap()
    // );

    // let msg = ['0' as u8, '1' as u8, '2' as u8, '3' as u8];
    // println!(
    //     "B wrote {} bytes: {:?}",
    //     b.write(&msg).unwrap(),
    //     std::str::from_utf8(&msg).unwrap()
    // );

    // let mut msg = ['a' as u8, 'a' as u8, 'a' as u8, 'a' as u8];
    // println!(
    //     "A read {} bytes: {:?}",
    //     a.read(&mut msg).unwrap(),
    //     std::str::from_utf8(&msg).unwrap()
    // );

    // println!(
    //     "a:{:?}, b:{:?}",
    //     a.as_socket().as_raw(),
    //     b.as_socket().as_raw()
    // );

    println!("\nSizeof Request: {}", std::mem::size_of::<Request>());
    println!(
        "\nSizeof io::Error: {}",
        std::mem::size_of::<std::io::Error>()
    );
    println!(
        "\nSizeof Rstd::io::esult<i32>: {}",
        std::mem::size_of::<std::io::Result<i32>>()
    );
    println!(
        "\nSizeof Rstd::io::esult<()>: {}",
        std::mem::size_of::<std::io::Result<()>>()
    );
    println!(
        "sys::current_thread(): {:?}",
        xaio::sys::get_current_thread_id()
    );
    let handles = vec![
        std::thread::spawn(|| {
            println!(
                " - sys::current_thread(): {:?}",
                xaio::sys::get_current_thread_id()
            );
        }),
        std::thread::spawn(|| {
            println!(
                " - sys::current_thread(): {:?}",
                xaio::sys::get_current_thread_id()
            );
        }),
        std::thread::spawn(|| {
            println!(
                " - sys::current_thread(): {:?}",
                xaio::sys::get_current_thread_id()
            );
        }),
        std::thread::spawn(|| {
            let id = xaio::sys::get_current_thread_id();
            println!(
                " - sys::current_thread(): {:?} ({})",
                id,
                id == xaio::sys::get_current_thread_id()
            );
        }),
    ];
    for h in handles {
        h.join().unwrap();
    }
}
