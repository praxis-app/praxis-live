use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::{closure::Closure, prelude::*, JsCast};
use web_sys::{window, CanvasRenderingContext2d, HtmlCanvasElement, Window};

const CELL: f64 = 24.0;

struct State {
    canvas: HtmlCanvasElement,
    ctx: CanvasRenderingContext2d,
    width: f64,
    height: f64,
    dpr: f64,
}

#[wasm_bindgen]
pub struct PixelBackground {
    window: Window,
    _state: Rc<RefCell<State>>,
    animation: Rc<RefCell<Option<Closure<dyn FnMut(f64)>>>>,
    resize: Closure<dyn FnMut()>,
    frame_id: Rc<RefCell<Option<i32>>>,
}

#[wasm_bindgen]
impl PixelBackground {
    #[wasm_bindgen(constructor)]
    pub fn new(canvas: HtmlCanvasElement) -> Result<PixelBackground, JsValue> {
        let window = window().ok_or_else(|| JsValue::from_str("window unavailable"))?;
        let context = canvas
            .get_context("2d")?
            .ok_or_else(|| JsValue::from_str("2d context unavailable"))?
            .dyn_into::<CanvasRenderingContext2d>()?;

        let state = Rc::new(RefCell::new(State {
            canvas,
            ctx: context,
            width: 0.0,
            height: 0.0,
            dpr: 1.0,
        }));

        resize_canvas(&state)?;

        let resize_state = Rc::clone(&state);
        let resize = Closure::wrap(Box::new(move || {
            let _ = resize_canvas(&resize_state);
        }) as Box<dyn FnMut()>);

        window.add_event_listener_with_callback("resize", resize.as_ref().unchecked_ref())?;

        let animation: Rc<RefCell<Option<Closure<dyn FnMut(f64)>>>> =
            Rc::new(RefCell::new(None));
        let frame_id = Rc::new(RefCell::new(None));
        let animation_state = Rc::clone(&state);
        let animation_window = window.clone();
        let animation_slot = Rc::clone(&animation);
        let frame_slot = Rc::clone(&frame_id);

        *animation.borrow_mut() = Some(Closure::wrap(Box::new(move |time: f64| {
            render(&animation_state, time);

            if let Some(callback) = animation_slot.borrow().as_ref() {
                if let Ok(id) = animation_window
                    .request_animation_frame(callback.as_ref().unchecked_ref())
                {
                    *frame_slot.borrow_mut() = Some(id);
                }
            }
        }) as Box<dyn FnMut(f64)>));

        if let Some(callback) = animation.borrow().as_ref() {
            let id = window.request_animation_frame(callback.as_ref().unchecked_ref())?;
            *frame_id.borrow_mut() = Some(id);
        }

        Ok(PixelBackground {
            window,
            _state: state,
            animation,
            resize,
            frame_id,
        })
    }

    pub fn stop(&self) {
        if let Some(frame_id) = *self.frame_id.borrow() {
            let _ = self.window.cancel_animation_frame(frame_id);
        }

        let _ = self
            .window
            .remove_event_listener_with_callback("resize", self.resize.as_ref().unchecked_ref());
    }
}

impl Drop for PixelBackground {
    fn drop(&mut self) {
        self.stop();
        self.animation.borrow_mut().take();
    }
}

fn resize_canvas(state: &Rc<RefCell<State>>) -> Result<(), JsValue> {
    let mut state = state.borrow_mut();
    let rect = state.canvas.get_bounding_client_rect();
    let dpr = window()
        .and_then(|win| win.device_pixel_ratio().try_into().ok())
        .unwrap_or(1.0);

    let width = rect.width().max(1.0);
    let height = rect.height().max(1.0);

    state.canvas.set_width((width * dpr).round() as u32);
    state.canvas.set_height((height * dpr).round() as u32);

    state
        .canvas
        .style()
        .set_property("width", &format!("{width}px"))?;
    state
        .canvas
        .style()
        .set_property("height", &format!("{height}px"))?;

    state.width = width;
    state.height = height;
    state.dpr = dpr;

    state.ctx.set_transform(1.0, 0.0, 0.0, 1.0, 0.0, 0.0)?;
    state.ctx.scale(dpr, dpr)?;

    Ok(())
}

fn render(state: &Rc<RefCell<State>>, time: f64) {
    let state = state.borrow();
    let t = time / 1000.0;
    let ctx = &state.ctx;
    let width = state.width;
    let height = state.height;

    ctx.set_fill_style_str("#09090b");
    ctx.fill_rect(0.0, 0.0, width, height);

    draw_pixel_field(ctx, width, height, t);
    draw_pixel_clusters(ctx, width, height, t);
    draw_glow(ctx, width, height, t);
}

fn draw_pixel_field(ctx: &CanvasRenderingContext2d, width: f64, height: f64, t: f64) {
    let columns = (width / CELL).ceil() as i32 + 2;
    let rows = (height / CELL).ceil() as i32 + 2;
    let x_shift = ((t * 12.0) % CELL).floor();
    let y_shift = ((t * 8.0) % CELL).floor();

    for row in 0..rows {
        for col in 0..columns {
            let seed = hash(col, row, (t * 0.5).floor() as i32);
            if seed < 0.52 {
                continue;
            }

            let color = if seed > 0.88 {
                "rgba(251, 191, 36, 0.10)"
            } else if seed > 0.76 {
                "rgba(244, 114, 182, 0.09)"
            } else if seed > 0.64 {
                "rgba(167, 139, 250, 0.10)"
            } else {
                "rgba(103, 232, 249, 0.08)"
            };

            let x = (col as f64 * CELL) - x_shift;
            let y = (row as f64 * CELL) - y_shift;
            ctx.set_fill_style_str(color);
            ctx.fill_rect(x, y, CELL - 6.0, CELL - 6.0);
        }
    }
}

fn draw_pixel_clusters(ctx: &CanvasRenderingContext2d, width: f64, height: f64, t: f64) {
    let palettes = [
        "rgba(125, 211, 252, 0.12)",
        "rgba(196, 181, 253, 0.12)",
        "rgba(253, 186, 116, 0.10)",
        "rgba(244, 114, 182, 0.10)",
    ];

    for index in 0..8 {
        let phase = t * (0.08 + index as f64 * 0.007);
        let x = ((phase.sin() * 0.35 + 0.5) * width).floor();
        let y = ((phase.cos() * 0.28 + 0.5) * height).floor();
        let size = 48.0 + ((phase * 1.7).sin() + 1.0) * 18.0;

        ctx.set_fill_style_str(palettes[index % palettes.len()]);
        for y_index in 0..3 {
            for x_index in 0..3 {
                if hash(index as i32, x_index, y_index) < 0.35 {
                    continue;
                }

                ctx.fill_rect(
                    x + x_index as f64 * (size * 0.33),
                    y + y_index as f64 * (size * 0.33),
                    size * 0.28,
                    size * 0.28,
                );
            }
        }
    }
}

fn draw_glow(ctx: &CanvasRenderingContext2d, width: f64, height: f64, t: f64) {
    let gradient = ctx.create_radial_gradient(
        width * (0.5 + 0.12 * (t * 0.2).sin()),
        height * (0.45 + 0.12 * (t * 0.18).cos()),
        0.0,
        width * 0.5,
        height * 0.5,
        width.max(height) * 0.55,
    );

    let Ok(gradient) = gradient else {
        return;
    };

    let _ = gradient.add_color_stop(0.0, "rgba(103, 232, 249, 0.10)");
    let _ = gradient.add_color_stop(0.35, "rgba(167, 139, 250, 0.08)");
    let _ = gradient.add_color_stop(0.7, "rgba(251, 191, 36, 0.05)");
    let _ = gradient.add_color_stop(1.0, "rgba(9, 9, 11, 0.0)");
    ctx.set_fill_style(&gradient.into());
    ctx.fill_rect(0.0, 0.0, width, height);
}

fn hash(a: i32, b: i32, c: i32) -> f64 {
    let mut value = (a as i64 * 374_761_393) ^ (b as i64 * 668_265_263) ^ (c as i64 * 362_437);
    value = (value ^ (value >> 13)) * 1_274_126_177;
    let value = value ^ (value >> 16);
    (value & 1023) as f64 / 1023.0
}
