use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll, RawWaker, RawWakerVTable, Waker},
};

fn do_nothing(_: *const ()) {}

fn clone_rawmaker(_: *const ()) -> RawWaker {
    RawWaker::new(core::ptr::null(), &WAKER_TABLE)
}

// Create a static RawWakerVTable with functions for waker operations.
static WAKER_TABLE: RawWakerVTable =
    RawWakerVTable::new(clone_rawmaker, do_nothing, do_nothing, do_nothing);

// A function to block on a Future and return its output.
pub(crate) fn block_on<F: Future>(mut func: F) -> F::Output {
    // Create a Pin from the provided Future.
    let mut func = unsafe { core::pin::Pin::new_unchecked(&mut func) };

    // Create a RawWaker for wake-up notifications.
    let raw_waker = RawWaker::new(core::ptr::null(), &WAKER_TABLE);

    // Create a Waker from the RawWaker.
    let waker = unsafe { Waker::from_raw(raw_waker) };

    // Create a Context for polling the Future with the given Waker.
    let mut ctx = Context::from_waker(&waker);

    // Poll the Future using the Context.
    let poll = Pin::new(&mut func).poll(&mut ctx);

    // Continuously poll the Future until it's ready.
    let result = loop {
        match poll {
            Poll::Pending => continue,
            Poll::Ready(result) => break result,
        };
    };
    result // Return the result of the Future once it's ready.
}
