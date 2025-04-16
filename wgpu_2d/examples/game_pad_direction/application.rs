use std::f32::consts::PI;
use std::sync::Arc;

use crate::input::*;

use gamepad_input::{GamepadMap, GamepadID, XInputGamepad};
use winit::event::{DeviceEvent, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

use crate::shader_manager::*;
use crate::wgpu_context::*;

use crate::math::{Vector2, Vector4};
use crate::rendering::*;
use crate::timer::Timer;

pub struct App {
    title: &'static str,
    inner: Option<AppInner>,
}

impl App {
    pub fn new(title: &'static str) -> Self {
        Self { title, inner: None }
    }
}

struct AppInner {
    window: Arc<Window>,
    render_context: WGPUContext,
    shader_manager: ShaderManager,
    renderer: Renderer2D,
    timer: Timer,
    input: Input,
    scene: (RingRenderer, RectangleRenderer),
    gamepad: Option<XInputGamepad>,
}

impl AppInner {
    pub fn init(window: Window) -> Self {
        let window = Arc::new(window);

        // Create shader_manager
        let shader_manager = ShaderManager::new("");

        // Create input manager
        let input = Input::new();

        // Create WGPU context
        let render_context = WGPUContext::new(
            Arc::clone(&window),
            [window.inner_size().width, window.inner_size().height],
        );

        // Create Timer
        let timer = Timer::new();

        // Create Renderer
        let renderer = Renderer2D::new(&render_context);

        // Create scene
        //  - Ring
        let center = Vector2::new([
            render_context.config().width as f32 / 2.,
            render_context.config().height as f32 / 2.,
        ]);
        const RADIUS: f32 = 200.;
        let rings = vec![Ring {
            color: Vector4::new([1., 1., 1., 1.]),
            position: center,
            outer_radius: RADIUS,
            inner_radius: RADIUS * 0.9,
        }];
        let rings = RingRenderer::new(
            rings,
            renderer.uniform_bind_group_layout(),
            &render_context,
            &shader_manager,
        );

        // - Aim Bar
        const START_ANGLE: f32 = -PI / 2.;
        let rects = vec![CenterRect {
            color: Vector4::new([1., 1., 1., 1.]),
            center: center + Vector2::rotation(START_ANGLE) * RADIUS / 2. * 0.98,
            size: Vector2::new([RADIUS * 0.95, 10.]),
            rotation: START_ANGLE,
        }];
        let rects = RectangleRenderer::new(
            rects,
            renderer.uniform_bind_group_layout(),
            &render_context,
            &shader_manager,
        );

        Self {
            window,
            scene: (rings, rects),
            renderer,
            render_context,
            shader_manager,
            timer,
            input,
            gamepad: None,
        }
    }

    pub fn update_scene(&mut self) {
        // Handle Gamepad state
        match self.gamepad {
            None => self.gamepad = self.input.gamepad_map.current(GamepadID::Id0).copied(),
            Some(ref mut gamepad) => {
                match (
                    self.input.gamepad_map.current(GamepadID::Id0),
                    self.input.gamepad_map.prev(GamepadID::Id0),
                ) {
                    (Some(current), Some(prev)) => {
                        const SENSITIVITY: f32 = 4.0;
                        gamepad.buttons = current.buttons;
                        gamepad.left_trigger = current.left_trigger;
                        gamepad.right_trigger = current.right_trigger;

                        if Vector2::new(current.left_thumb).mag() < 0.8 {
                            gamepad.left_thumb = current.left_thumb;
                        } else {
                            let current = Vector2::new(current.left_thumb);
                            let prev = Vector2::new(prev.left_thumb);
                            let gamepad_vec =
                                Vector2::new(gamepad.left_thumb).normalized() * current.mag();
                            let mut angle_diff = current.angle() - prev.angle();
                            if angle_diff < -PI {
                                angle_diff = -2. * PI - angle_diff;
                            } else if angle_diff > PI {
                                angle_diff = 2. * PI - angle_diff
                            }
                            gamepad.left_thumb =
                                gamepad_vec.rotate(angle_diff / SENSITIVITY).into_inner();
                        }

                        if Vector2::new(current.right_thumb).mag() < 0.8 {
                            gamepad.right_thumb = current.right_thumb;
                        } else {
                            let current = Vector2::new(current.right_thumb);
                            let prev = Vector2::new(prev.right_thumb);
                            let gamepad_vec =
                                Vector2::new(gamepad.right_thumb).normalized() * current.mag();
                            let mut angle_diff = current.angle() - prev.angle();
                            if angle_diff < -PI {
                                angle_diff = -2. * PI - angle_diff;
                            } else if angle_diff > PI {
                                angle_diff = 2. * PI - angle_diff
                            }
                            if angle_diff > 1. {
                                println!("{:?}", angle_diff);
                            }
                            gamepad.right_thumb =
                                gamepad_vec.rotate(angle_diff / SENSITIVITY).into_inner();
                        }
                    }
                    (current, _) => self.gamepad = current.copied(),
                }
            }
        }

        let delta = self.timer.elapsed_reset();
        self.timer.reset();

        let center = Vector2::new([
            self.render_context.config().width as f32 / 2.,
            self.render_context.config().height as f32 / 2.,
        ]);

        let stick_pos = Vector2::new(self.gamepad.map(|x| x.right_thumb).unwrap_or(
			((self.input.mouse_map.mouse_position() - center) / 200. * Vector2::new([1., -1.])).into_inner()
		));
        let len = stick_pos.mag().min(1.) * 200.;
        let angle = stick_pos.angle();

        self.scene.0.rings_mut()[0].position = center;

        self.scene.1.rects_mut()[0].center = center + (Vector2::rotation(-angle) * len) / 2. * 0.98;
        self.scene.1.rects_mut()[0].size[0] = len;
        self.scene.1.rects_mut()[0].rotation = -angle;

        self.scene.0.update_rings(&self.render_context);
        self.scene.1.update_rects(&self.render_context);
    }
}

impl winit::application::ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        match &self.inner {
            None => {
                // Create window
                let window = event_loop
                    .create_window(Window::default_attributes().with_title(self.title.to_owned()))
                    .expect("Could not create window");
                self.inner = Some(AppInner::init(window));
            }
            _ => (),
        }
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
        let inner = self.inner.as_mut().unwrap();
        match event {
            DeviceEvent::MouseMotion { delta } => {
                inner.input.mouse_map.handle_raw_mouse_movement(delta)
            }
            DeviceEvent::MouseWheel { delta } => inner.input.mouse_map.handle_raw_scroll(delta),
            _ => (),
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let inner = self.inner.as_mut().unwrap();
        match event {
            WindowEvent::CloseRequested => {
                println!("The close button was pressed; stopping");
                event_loop.exit();
            }
            WindowEvent::KeyboardInput { event, .. } => match event.logical_key {
                Key::Named(NamedKey::Escape) => event_loop.exit(),
                Key::Named(NamedKey::Space) => inner.shader_manager.reload(),
                x => inner.input.key_map.handle_key(x, event.state),
            },
            WindowEvent::CursorMoved { position, .. } => {
                inner.input.mouse_map.handle_cursor_movement(position);
            }
            WindowEvent::MouseWheel { delta, .. } => {
                inner.input.mouse_map.handle_mouse_scroll(delta);
            }
            WindowEvent::MouseInput { button, state, .. } => {
                inner.input.mouse_map.handle_button(button, state);
            }
            WindowEvent::Resized(new_size) => {
                // inner.render_context.resize(winit::dpi::PhysicalSize::new(8, 8));
                inner
                    .render_context
                    .resize([new_size.width, new_size.height]);
				*inner.renderer.get_uniform().screen_size = [new_size.width as f32, new_size.height as f32];
                inner.renderer.update_uniform(&inner.render_context);
                inner.window.request_redraw();
            }
            WindowEvent::RedrawRequested => {
                inner.input.gamepad_map.update();
                inner.update_scene();
                inner.renderer.render(
                    [
                        &inner.scene.1 as &dyn Render,
                        &inner.scene.0 as &dyn Render,
                    ],
                    &inner.render_context,
                    &inner.shader_manager,
                );
                inner.window.request_redraw();
            }
            _ => (),
        }
    }
}

struct Input {
    key_map: KeyMap,
    mouse_map: MouseMap,
    gamepad_map: GamepadMap,
}

impl Input {
    pub fn new() -> Self {
        Self {
            key_map: KeyMap::new(),
            mouse_map: MouseMap::new(),
            gamepad_map: GamepadMap::new(),
        }
    }
}
