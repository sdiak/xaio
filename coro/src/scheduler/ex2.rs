use crate::collection::SList;

use super::Coroutine;
pub struct Ex2 {
    local_tasks: SList<Coroutine>,
}
impl Ex2 {
    fn new() -> Self {
        Self {
            local_tasks: SList::new(),
        }
    }
}
// crate::sync::parking_spot::Context

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let mut x = generator::Gn::<()>::new_scoped_opt_local(1048576, move |mut scope| {
            println!(" - coro: 0");
            scope.yield_with(());
            // scope.yield_(());
            println!(" - coro: 1");
        });
        println!("\nEx: start ({})", std::mem::size_of_val(&x));
        while !x.is_done() {
            println!("Ex: resume");
            x.send(());
        }
        println!("Ex: done");
    }
}
