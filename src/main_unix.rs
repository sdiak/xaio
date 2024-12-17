use xaio::{Driver, DriverConfig, DriverIFace, DriverKind, ReadyList, Request, Ring};
// use xaio::thread_pool::s

extern "C" fn io_work_callback(req: &mut Request) {
    println!("io_work_callback({}: {})", req.opcode_raw(), req.status());
}

pub fn main() {
    println!("Hello unix");
    println!(
        "sys::current_thread(): {:?}",
        xaio::sys::ThreadId::current()
    );
    let config = DriverConfig::default();
    let mut driver = Driver::new(DriverKind::EPoll, &config).unwrap();
    println!("{driver:?}");
    let mut ready = ReadyList::new();
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
        xaio::sys::ThreadId::current()
    );
    let handles = vec![
        std::thread::spawn(|| {
            println!(
                " - sys::current_thread(): {:?}",
                xaio::sys::ThreadId::current()
            );
        }),
        std::thread::spawn(|| {
            println!(
                " - sys::current_thread(): {:?}",
                xaio::sys::ThreadId::current()
            );
        }),
        std::thread::spawn(|| {
            println!(
                " - sys::current_thread(): {:?}",
                xaio::sys::ThreadId::current()
            );
        }),
        std::thread::spawn(|| {
            let id = xaio::sys::ThreadId::current();
            println!(
                " - sys::current_thread(): {:?} ({})",
                id,
                id == xaio::sys::ThreadId::current()
            );
        }),
    ];
    for h in handles {
        h.join().unwrap();
    }

    let config = DriverConfig::default();
    let driver = Driver::new(DriverKind::EPoll, &config).unwrap();
    let ring = Ring::new(driver).unwrap();
    for i in 0..10 {
        ring.submit_io_work(
            move || {
                println!("Hello {i}");
                33 + i
            },
            Some(io_work_callback),
            None,
        )
        .unwrap();
    }
    std::thread::sleep(std::time::Duration::from_millis(1000));
    ring.wait_ms(1);

    println!(
        "\nSizeof Box<dyn>: {}",
        std::mem::size_of::<std::pin::Pin<Box<dyn std::future::Future<Output = i64>>>>()
    );
    println!("Probe: {:?}", &*xaio::sys::PROBE);
    let mut cnf = xaio::capi::xconfig_s::default();
    cnf.submission_queue_depth = 64;
    cnf.completion_queue_depth = 1024;
    let d = xaio::sys::Driver::new(&cnf).unwrap();
    d.init().unwrap();
    println!("Driver: {:?}", d);
    d.wake().unwrap();
    println!("Sizeof Sqe: {:?}", std::mem::size_of::<xaio::sys::Sqe>());
    println!(
        "Sizeof Option<NonNull<uring_sys2::io_uring_sqe>>: {:?}",
        std::mem::size_of::<Option<std::ptr::NonNull<uring_sys2::io_uring_sqe>>>()
    );
    println!(
        "Sizeof Option<xaio::sys::Submission>: {:?}",
        std::mem::size_of::<Option<xaio::sys::Submission>>()
    );
    // println!("-4096isize as usize: {}", -4096isize as usize);
    // println!("{}", 18446744073709547521usize as isize);
}
