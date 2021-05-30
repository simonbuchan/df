use winit::event_loop::EventLoop;

use context::Context;
use renderer::Renderer;

mod context;
mod renderer;

fn main() {
    wgpu_subscriber::initialize_default_subscriber(None);

    let event_loop = EventLoop::new();
    let context = pollster::block_on(Context::new(&event_loop));
    let renderer = Renderer::new(&context);
    context.run(event_loop, renderer);
}
