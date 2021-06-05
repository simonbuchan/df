use winit::event_loop::EventLoop;

use context::Context;
use renderer::Renderer;

mod context;
mod loader;
mod mesh;
mod renderer;

fn main() {
    wgpu_subscriber::initialize_default_subscriber(None);

    let mut loader =
        loader::Loader::open(r"C:\Games\Steam\steamapps\common\Dark Forces\Game\").unwrap();

    let event_loop = EventLoop::new();
    let context = pollster::block_on(Context::new(&event_loop));

    let level = loader.load_lev("SECBASE.LEV", &context).unwrap();

    let renderer = Renderer::new(&context, level);

    context.run(event_loop, renderer);
}

pub(crate) fn poll_device<F: std::future::Future>(context: &Context, mut fut: F) -> F::Output {
    use std::sync::Arc;
    use std::task::{Context, Poll, Wake, Waker};

    struct NullWake;
    impl Wake for NullWake {
        fn wake(self: Arc<Self>) {}
    }

    let waker = Waker::from(Arc::new(NullWake));
    let mut cx = Context::from_waker(&waker);

    loop {
        let fut = unsafe { std::pin::Pin::new_unchecked(&mut fut) };

        match fut.poll(&mut cx) {
            Poll::Pending => {
                context.device.poll(wgpu::Maintain::Wait);
            }
            Poll::Ready(value) => break value,
        }
    }
}

pub(crate) fn transmute_slice<T: Copy, U: Copy>(src: &[T]) -> &[U] {
    let size = std::mem::size_of_val(src);
    let rem = size % std::mem::size_of::<U>();
    assert_eq!(rem, 0);
    let len = size / std::mem::size_of::<U>();
    let ptr = src.as_ptr();
    // Safety: src and dst contents are Copy, we have checked sizes above
    unsafe { std::slice::from_raw_parts(ptr.cast(), len) }
}
