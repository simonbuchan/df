use winit::event_loop::EventLoop;

use context::Context;
use renderer::Renderer;

mod context;
mod renderer;

mod loader;

fn main() {
    wgpu_subscriber::initialize_default_subscriber(None);

    let mut loader =
        loader::Loader::open(r"C:\Games\Steam\steamapps\common\Dark Forces\Game\").unwrap();

    let pal = loader.load_pal("NARSHADA.PAL").unwrap();

    let event_loop = EventLoop::new();
    let context = pollster::block_on(Context::new(&event_loop));

    let texture = loader.load_bm("NSSIGN04.BM", &pal, &context).unwrap();

    let renderer = Renderer::new(&context, texture);
    context.run(event_loop, renderer);
}
