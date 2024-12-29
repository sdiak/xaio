// use std::{
//     marker::PhantomData,
//     sync::{
//         atomic::{AtomicBool, AtomicUsize},
//         Arc,
//     },
//     thread::Thread,
// };

// pub struct Scope<'scope, 'env: 'scope> {
//     data: Arc<ScopeData>,
//     scope: PhantomData<&'scope mut &'scope ()>,
//     env: PhantomData<&'env mut &'env ()>,
// }
// pub(super) struct ScopeData {
//     num_running_threads: AtomicUsize,
//     a_thread_panicked: AtomicBool,
//     main_thread: Thread,
// }

// impl<'scope, 'env: 'scope> Scope<'scope, 'env> {}

// pub fn scope<'env, F, T>(f: F) -> T
// where
//     F: for<'scope> FnOnce(&'scope Scope<'scope, 'env>) -> T,
// {
//     // We put the `ScopeData` into an `Arc` so that other threads can finish their
//     // `decrement_num_running_threads` even after this function returns.
//     let scope = Scope {
//         data: Arc::new(ScopeData {
//             num_running_threads: AtomicUsize::new(0),
//             main_thread: current(),
//             a_thread_panicked: AtomicBool::new(false),
//         }),
//         env: PhantomData,
//         scope: PhantomData,
//     };

//     // Run `f`, but catch panics so we can make sure to wait for all the threads to join.
//     let result = catch_unwind(AssertUnwindSafe(|| f(&scope)));

//     // Wait until all the threads are finished.
//     while scope.data.num_running_threads.load(Ordering::Acquire) != 0 {
//         park();
//     }

//     // Throw any panic from `f`, or the return value of `f` if no thread panicked.
//     match result {
//         Err(e) => resume_unwind(e),
//         Ok(_) if scope.data.a_thread_panicked.load(Ordering::Relaxed) => {
//             panic!("a scoped thread panicked")
//         }
//         Ok(result) => result,
//     }
// }
