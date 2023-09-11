use core::{
    cell::Cell,
    fmt,
    future::Future,
    marker::PhantomData,
    mem::ManuallyDrop,
    ops::Deref,
    sync::atomic::{AtomicBool, Ordering},
    task::{Context, Poll, RawWaker, RawWakerVTable, Waker},
};
use std::{
    sync::Arc,
    thread::{self, Thread},
};

#[macro_export]
macro_rules! pin_mut {
    ($($x:ident),* $(,)?) => { $(
        let mut $x = $x;
        #[allow(unused_mut)]
        let mut $x = unsafe {
            core::pin::Pin::new_unchecked(&mut $x)
        };
    )* }
}

trait ArcWake: Send + Sync {
    fn wake(self: Arc<Self>) {
        Self::wake_by_ref(&self)
    }
    fn wake_by_ref(arc_self: &Arc<Self>);
}

unsafe fn increase_refcount<T: ArcWake>(data: *const ()) {
    let arc = core::mem::ManuallyDrop::new(Arc::<T>::from_raw(data.cast::<T>()));
    let _arc_clone: core::mem::ManuallyDrop<_> = arc.clone();
}

unsafe fn clone_arc_raw<T: ArcWake>(data: *const ()) -> RawWaker {
    increase_refcount::<T>(data);
    RawWaker::new(data, waker_vtable::<T>())
}

unsafe fn wake_arc_raw<T: ArcWake>(data: *const ()) {
    let arc: Arc<T> = Arc::from_raw(data.cast::<T>());
    ArcWake::wake(arc);
}

unsafe fn wake_by_ref_arc_raw<T: ArcWake>(data: *const ()) {
    let arc = core::mem::ManuallyDrop::new(Arc::<T>::from_raw(data.cast::<T>()));
    ArcWake::wake_by_ref(&arc);
}

unsafe fn drop_arc_raw<T: ArcWake>(data: *const ()) {
    drop(Arc::<T>::from_raw(data.cast::<T>()))
}

fn waker_vtable<W: ArcWake>() -> &'static RawWakerVTable {
    &RawWakerVTable::new(
        clone_arc_raw::<W>,
        wake_arc_raw::<W>,
        wake_by_ref_arc_raw::<W>,
        drop_arc_raw::<W>,
    )
}

struct ThreadNotify {
    thread: Thread,
    unparked: AtomicBool,
}

impl ArcWake for ThreadNotify {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        let unparked = arc_self.unparked.swap(true, Ordering::Release);
        if !unparked {
            arc_self.thread.unpark();
        }
    }
}

thread_local! {
    static CURRENT_THREAD_NOTIFY: Arc<ThreadNotify> = Arc::new(ThreadNotify {
        thread: thread::current(),
        unparked: AtomicBool::new(false),
    });
}

thread_local!(static ENTERED: Cell<bool> = Cell::new(false));

struct Enter {
    _priv: (),
}
struct EnterError {
    _priv: (),
}

impl fmt::Debug for EnterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EnterError").finish()
    }
}

impl fmt::Display for EnterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "an execution scope has already been entered")
    }
}

impl std::error::Error for EnterError {}

fn enter() -> Result<Enter, EnterError> {
    ENTERED.with(|c| {
        if c.get() {
            Err(EnterError { _priv: () })
        } else {
            c.set(true);

            Ok(Enter { _priv: () })
        }
    })
}

fn run_executor<T, F: FnMut(&mut Context<'_>) -> Poll<T>>(mut f: F) -> T {
    let _enter = enter().expect(
        "cannot execute `LocalPool` executor from within \
         another executor",
    );

    CURRENT_THREAD_NOTIFY.with(|thread_notify| {
        let waker = waker_ref(thread_notify);
        let mut cx = Context::from_waker(&waker);
        loop {
            if let Poll::Ready(t) = f(&mut cx) {
                return t;
            }
            while !thread_notify.unparked.swap(false, Ordering::Acquire) {
                thread::park();
            }
        }
    })
}
#[derive(Debug)]
struct WakerRef<'a> {
    waker: ManuallyDrop<Waker>,
    _marker: PhantomData<&'a ()>,
}
impl<'a> WakerRef<'a> {
    #[inline]
    fn new_unowned(waker: ManuallyDrop<Waker>) -> Self {
        Self {
            waker,
            _marker: PhantomData,
        }
    }
}
impl Deref for WakerRef<'_> {
    type Target = Waker;

    #[inline]
    fn deref(&self) -> &Waker {
        &self.waker
    }
}
fn waker_ref<W>(wake: &Arc<W>) -> WakerRef<'_>
where
    W: ArcWake,
{
    let ptr = Arc::as_ptr(wake).cast::<()>();

    let waker =
        ManuallyDrop::new(unsafe { Waker::from_raw(RawWaker::new(ptr, waker_vtable::<W>())) });
    WakerRef::new_unowned(waker)
}

pub(crate) fn block_on<F: Future>(f: F) -> F::Output {
    pin_mut!(f);
    run_executor(|cx| f.as_mut().poll(cx))
}
