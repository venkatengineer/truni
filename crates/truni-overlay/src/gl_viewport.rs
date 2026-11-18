use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{EventControllerScroll, EventControllerScrollFlags, GestureClick, GestureDrag, GLArea};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use truni_core::events::TelemetrySnapshot;
use truni_spaceship::{SpaceshipRenderer, SpaceshipScene};

#[link(name = "EGL")]
#[link(name = "dl")]
extern "C" {
    fn eglGetProcAddress(name: *const std::os::raw::c_char) -> *const std::os::raw::c_void;
    fn dlsym(handle: *mut std::os::raw::c_void, symbol: *const std::os::raw::c_char) -> *const std::os::raw::c_void;
}

pub struct SpaceshipViewport {
    pub gl_area: GLArea,
    pub scene: Rc<RefCell<SpaceshipScene>>,
}

impl SpaceshipViewport {
    pub fn new() -> Self {
        let gl_area = GLArea::builder()
            .has_depth_buffer(true)
            .has_stencil_buffer(false)
            .auto_render(true)
            .hexpand(true)
            .vexpand(true)
            .build();

        let scene = Rc::new(RefCell::new(SpaceshipScene::new()));
        let renderer_storage: Rc<RefCell<Option<SpaceshipRenderer>>> = Rc::new(RefCell::new(None));

        // Realize signal
        {
            let renderer_storage = renderer_storage.clone();
            gl_area.connect_realize(move |area| {
                area.make_current();
                let gl = Arc::new(unsafe {
                    glow::Context::from_loader_function(|s| {
                        let c_str = std::ffi::CString::new(s).unwrap();
                        let ptr = eglGetProcAddress(c_str.as_ptr());
                        if !ptr.is_null() {
                            ptr
                        } else {
                            dlsym(std::ptr::null_mut(), c_str.as_ptr())
                        }
                    })
                });

                match unsafe { SpaceshipRenderer::new(gl) } {
                    Ok(renderer) => {
                        *renderer_storage.borrow_mut() = Some(renderer);
                    }
                    Err(e) => {
                        eprintln!("Failed to initialize SpaceshipRenderer: {}", e);
                    }
                }
            });
        }

        // Render signal
        {
            let scene = scene.clone();
            let renderer_storage = renderer_storage.clone();
            gl_area.connect_render(move |area, _| {
                if let Some(renderer) = renderer_storage.borrow().as_ref() {
                    let w = area.width();
                    let h = area.height();
                    let sc = scene.borrow();
                    unsafe {
                        renderer.render(&sc, w, h);
                    }
                }
                glib::Propagation::Proceed
            });
        }

        // Drag gesture for camera orbit
        let drag = GestureDrag::new();
        {
            let scene = scene.clone();
            let gl_area_clone = gl_area.clone();
            let last_pos = Rc::new(RefCell::new((0.0f64, 0.0f64)));

            let last_pos_drag = last_pos.clone();
            drag.connect_drag_begin(move |_, x, y| {
                *last_pos_drag.borrow_mut() = (x, y);
            });

            drag.connect_drag_update(move |_, x, y| {
                let (last_x, last_y) = *last_pos.borrow();
                let dx = (x - last_x) as f32;
                let dy = (y - last_y) as f32;
                *last_pos.borrow_mut() = (x, y);

                scene.borrow_mut().camera.rotate(dx, dy);
                gl_area_clone.queue_render();
            });
        }
        gl_area.add_controller(drag);

        // Scroll gesture for camera zoom
        let scroll = EventControllerScroll::new(EventControllerScrollFlags::VERTICAL);
        {
            let scene = scene.clone();
            let gl_area_clone = gl_area.clone();
            scroll.connect_scroll(move |_, _, dy| {
                scene.borrow_mut().camera.zoom(dy as f32);
                gl_area_clone.queue_render();
                glib::Propagation::Stop
            });
        }
        gl_area.add_controller(scroll);

        // Click gesture for cycling/selecting components
        let click = GestureClick::new();
        {
            let scene = scene.clone();
            let gl_area_clone = gl_area.clone();
            click.connect_released(move |_, _, _, _| {
                scene.borrow_mut().select_next();
                gl_area_clone.queue_render();
            });
        }
        gl_area.add_controller(click);

        Self { gl_area, scene }
    }

    pub fn update_telemetry(&self, snap: &TelemetrySnapshot, dt: f32) {
        self.scene.borrow_mut().update(snap, dt);
        self.gl_area.queue_render();
    }
}
