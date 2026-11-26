//! A tiny bubbletea program exercising the runtime: alt screen, mouse, key
//! decoding, window size, a ticking timer and a text input.
//!
//! `cargo run -p sst-tuikit --example counter` (press `esc` to quit).

use std::time::Duration;

use sst_tuikit::bubbles::textinput;
use sst_tuikit::lipgloss;
use sst_tuikit::tea::{self, Cmd, KeyMsg, Model, MouseMsg, Msg, Program, WindowSizeMsg, KEY_ESC};

struct Tick;

struct App {
    ticks: u32,
    last: String,
    size: (i32, i32),
    input: textinput::Model,
}

fn tick() -> Option<Cmd> {
    tea::tick(Duration::from_millis(250), |_| Tick)
}

impl Model for App {
    fn init(&self) -> Option<Cmd> {
        // Go's Init has a value receiver: focusing here would be lost, so the
        // input is focused in main and Init only starts the blink.
        tea::batch(vec![tick(), textinput::blink()])
    }

    fn update(&mut self, msg: Msg) -> Option<Cmd> {
        if msg.downcast_ref::<Tick>().is_some() {
            self.ticks += 1;
            return tick();
        }
        if let Some(ws) = msg.downcast_ref::<WindowSizeMsg>() {
            self.size = (ws.width, ws.height);
            return None;
        }
        if let Some(m) = msg.downcast_ref::<MouseMsg>() {
            self.last = format!("mouse {} at {},{}", m.string(), m.x, m.y);
            return None;
        }
        if let Some(k) = msg.downcast_ref::<KeyMsg>() {
            self.last = format!("key {:?}", k.string());
            if k.typ == KEY_ESC {
                return tea::quit();
            }
        }
        self.input.update(&msg)
    }

    fn view(&self) -> String {
        let title = lipgloss::new_style()
            .bold(true)
            .foreground(lipgloss::color("#00E5FF"))
            .border(lipgloss::rounded_border(), &[])
            .padding(&[0, 1])
            .render(&format!(
                "ticks: {}  size: {}x{}",
                self.ticks, self.size.0, self.size.1
            ));
        lipgloss::join_vertical(lipgloss::LEFT, &[&title, &self.last, &self.input.view()])
    }
}

fn main() {
    let mut input = textinput::new();
    input.prompt = "COMMAND> ".into();
    let _ = input.focus();
    let app = App {
        ticks: 0,
        last: String::new(),
        size: (0, 0),
        input,
    };
    let p = Program::new(
        app,
        &[tea::with_alt_screen(), tea::with_mouse_cell_motion()],
    );
    match p.run() {
        Ok(m) => println!(
            "bye after {} ticks, last {}, input {:?}",
            m.ticks,
            m.last,
            m.input.value()
        ),
        Err(e) => eprintln!("Error running program: {e}"),
    }
}
