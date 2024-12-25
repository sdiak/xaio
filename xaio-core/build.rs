use std::env;

struct Context {
    target_os: String,
}
impl Context {
    fn new() -> Self {
        Self {
            target_os: env::var("CARGO_CFG_TARGET_OS").unwrap(),
        }
    }
}
fn has_kqueue(cx: &Context) -> bool {
    println!("cargo::rustc-check-cfg=cfg(has_kqueue)");
    println!("cargo::rustc-check-cfg=cfg(has_evfilt_user)");
    if ["freebsd", "macos", "ios", "tvos", "watchos", "visionos"].contains(&cx.target_os.as_str()) {
        println!("cargo::rustc-cfg=has_kqueue");
        println!("cargo::rustc-cfg=has_evfilt_user");
        true
    } else if ["openbsd", "netbsd", "dragonfly"].contains(&cx.target_os.as_str()) {
        println!("cargo::rustc-cfg=has_kqueue");
        true
    } else {
        false
    }
}

fn autocfg() {
    let cx = Context::new();
    has_kqueue(&cx);
}

#[allow(clippy::field_reassign_with_default)]
fn main() {
    autocfg();
}
