use cgmath::prelude::*;
use winit::event_loop::EventLoop;

use context::Context;
use renderer::Renderer;

use crate::camera::Camera;
use crate::render_target::SurfaceRenderTarget;

mod camera;
mod context;
mod loader;
mod mesh;
mod render_target;
mod renderer;

fn main() {
    wgpu_subscriber::initialize_default_subscriber(None);

    let steam_path = registry::Hive::CurrentUser.open(r"Software\Valve\Steam", registry::Security::Read)
        .expect("want a Steam installation")
        .value("SteamPath")
        .unwrap();
    let steam_path = match steam_path {
        registry::Data::String(steam_path) => steam_path.to_os_string(),
        _ => panic!("unexpected type: {:?}", steam_path),
    };

    let game_path = std::path::Path::new(&steam_path).join(r"steamapps\common\Dark Forces\Game");

    let mut loader = loader::Loader::open(game_path).unwrap();

    let level_names = loader.level_names();
    dbg!(&level_names);
    let mut level_index = 0;

    let event_loop = EventLoop::new();
    let context = pollster::block_on(Context::new(&event_loop));

    let level = loader
        .load_lev(&level_names[level_index], &context)
        .unwrap();

    let mut renderer = Renderer::new(&context, level);

    let mut target = SurfaceRenderTarget::new(&context);

    let mut camera = Camera::new(cgmath::point3(246.0, 310.0, 8.0));

    let mut input_state = InputState::default();
    let mut grab = false;

    let mut last_update = std::time::Instant::now();

    event_loop.run(move |event, _, control_flow| {
        use winit::event::{DeviceEvent, ElementState, Event, MouseButton, VirtualKeyCode, WindowEvent};
        use winit::event_loop::ControlFlow;

        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => {
                    *control_flow = ControlFlow::Exit;
                }
                WindowEvent::MouseInput {
                    button: MouseButton::Left,
                    ..
                } => {
                    grab = true;
                    let _ = context.window.set_cursor_grab(true);
                    let _ = context.window.set_cursor_visible(false);
                }
                WindowEvent::KeyboardInput { input, .. } => {
                    if input.state == ElementState::Pressed {
                        match input.virtual_keycode {
                            Some(VirtualKeyCode::Escape) => {
                                grab = false;
                                let _ = context.window.set_cursor_grab(false);
                                let _ = context.window.set_cursor_visible(true);
                            }
                            Some(VirtualKeyCode::Tab) => {
                                level_index = (level_index + 1) % level_names.len();
                                let level = loader
                                    .load_lev(&level_names[level_index], &context)
                                    .unwrap();
                                renderer = Renderer::new(&context, level);
                            }
                            _ => {}
                        }
                    }
                    input_state.key(input);
                }
                _ => {}
            },
            Event::DeviceEvent {
                event: DeviceEvent::MouseMotion { delta },
                ..
            } => {
                if grab {
                    const DEGREES_PER_PIXEL: f32 = 1.0;

                    let delta_x = delta.0 as f32;

                    camera.dir += (cgmath::Deg(DEGREES_PER_PIXEL) * delta_x).into();
                }
            }
            Event::MainEventsCleared => {
                let now = std::time::Instant::now();
                let delta_time = now - last_update;
                last_update = now;
                const UNITS_PER_SECOND: f32 = 150.0;
                camera.eye +=
                    input_state.delta(camera.dir) * UNITS_PER_SECOND * delta_time.as_secs_f32();

                camera.render(&context, &mut target, &mut renderer);

                *control_flow = ControlFlow::Poll;
            }
            _ => {}
        }
    });
}

#[derive(Default)]
struct InputState {
    pub forward: bool,
    pub left: bool,
    pub right: bool,
    pub back: bool,
    pub up: bool,
    pub down: bool,
}

impl InputState {
    fn key(&mut self, input: winit::event::KeyboardInput) {
        use winit::event::{ElementState, VirtualKeyCode};

        match input.virtual_keycode {
            Some(VirtualKeyCode::Comma) => {
                self.forward = input.state == ElementState::Pressed;
            }
            Some(VirtualKeyCode::A) => {
                self.left = input.state == ElementState::Pressed;
            }
            Some(VirtualKeyCode::E) => {
                self.right = input.state == ElementState::Pressed;
            }
            Some(VirtualKeyCode::O) => {
                self.back = input.state == ElementState::Pressed;
            }
            Some(VirtualKeyCode::Space) => {
                self.up = input.state == ElementState::Pressed;
            }
            Some(VirtualKeyCode::LControl) => {
                self.down = input.state == ElementState::Pressed;
            }
            _ => {}
        }
    }

    fn delta(&self, dir: impl Angle<Unitless = f32>) -> cgmath::Vector3<f32> {
        let (s, c) = dir.sin_cos();
        let forward = cgmath::vec3(s, c, 0.0);
        let right = cgmath::vec3(c, -s, 0.0);
        let up = cgmath::Vector3::unit_z();
        let mut result = cgmath::Vector3::zero();
        if self.forward {
            result += forward;
        }
        if self.back {
            result -= forward;
        }
        if self.right {
            result += right;
        }
        if self.left {
            result -= right;
        }
        if self.up {
            result += up;
        }
        if self.down {
            result -= up;
        }
        result
    }
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

pub(crate) fn transmute_as_bytes<T: Copy>(src: &T) -> &[u8] {
    let size = std::mem::size_of_val(src);
    // Safety: src and dst contents are Copy
    unsafe { std::slice::from_raw_parts(src as *const T as *const u8, size) }
}
