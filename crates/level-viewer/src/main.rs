use winit::event_loop::EventLoop;

use crate::render_target::SurfaceRenderTarget;
use context::Context;
use renderer::Renderer;

mod context;
mod loader;
mod mesh;
mod render_target;
mod renderer;

fn main() {
    wgpu_subscriber::initialize_default_subscriber(None);

    let mut loader =
        loader::Loader::open(r"C:\Games\Steam\steamapps\common\Dark Forces\Game\").unwrap();

    let event_loop = EventLoop::new();
    let context = pollster::block_on(Context::new(&event_loop));

    let level = loader.load_lev("SECBASE.LEV", &context).unwrap();

    let mut renderer = Renderer::new(&context, level);

    let mut target = SurfaceRenderTarget::new(&context);

    event_loop.run(move |event, _, control_flow| {
        use winit::event::{Event, WindowEvent};
        use winit::event_loop::ControlFlow;
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
            }
            Event::RedrawRequested(_) => {
                target.render(&context, &mut renderer);
            }
            _ => {}
        }
    });
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
