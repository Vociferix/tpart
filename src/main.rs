use std::io;
use std::time::Instant;

use ratatui::{
    buffer::Buffer,
    crossterm::{
        event::{
            self, DisableMouseCapture, EnableMouseCapture, KeyCode, KeyEventKind, MouseEventKind,
        },
        execute,
    },
    layout::Rect,
    style::Color,
    widgets::StatefulWidget,
    DefaultTerminal,
};

use rand::Rng;

use rayon::prelude::*;

const DENSITY: f32 = 0.15f32;
const GRAVITY_STRENGTH: f32 = 1.2f32;
const FRICTION_PER_SECOND: f32 = 0.7f32;
const UPPER_BLOCK: &'static str = "▀";

struct Particle {
    x: f32,
    y: f32,
    dx: f32,
    dy: f32,
}

#[derive(Clone, Copy)]
struct Mouse {
    x: f32,
    y: f32,
}

struct SimulationWidget;

struct Simulation {
    particles: Vec<Particle>,
    time: Instant,
    mouse: Option<Mouse>,
}

fn generate_particles(density: f32, width: u16, height: u16) -> Vec<Particle> {
    let w = width as usize;
    let h = height as usize;
    let mut rng = rand::thread_rng();
    let count = ((w * h) as f32 * density) as usize;
    let mut particles = Vec::with_capacity(count);

    for _ in 0..count {
        particles.push(Particle {
            x: rng.gen_range(0.0..width as f32),
            y: rng.gen_range(0.0..height as f32),
            dx: rng.gen_range(-1.0..1.0),
            dy: rng.gen_range(-1.0..1.0),
        });
    }

    particles
}

impl StatefulWidget for SimulationWidget {
    type State = Simulation;

    fn render(self, area: Rect, buf: &mut Buffer, sim: &mut Simulation) {
        let curr_time = Instant::now();
        let delta_time = curr_time.saturating_duration_since(sim.time).as_secs_f32();
        sim.time = curr_time;
        let mouse = sim.mouse.clone();

        buf.content.par_iter_mut().for_each(|cell| {
            if cell.symbol() != UPPER_BLOCK {
                cell.set_symbol(UPPER_BLOCK);
            }
            cell.set_fg(Color::Black);
            cell.set_bg(Color::Black);
        });

        struct UnsafeBuf(*mut Buffer);

        unsafe impl Send for UnsafeBuf {}

        unsafe impl Sync for UnsafeBuf {}

        impl Clone for UnsafeBuf {
            fn clone(&self) -> Self {
                Self(self.0.clone())
            }
        }

        impl Copy for UnsafeBuf {}

        impl UnsafeBuf {
            fn buf(&self) -> &mut Buffer {
                unsafe { &mut *self.0 }
            }
        }

        let unsafe_buf = UnsafeBuf(buf as *mut Buffer);

        sim.particles.par_iter_mut().for_each(move |particle| {
            if let Some(mouse) = mouse {
                let dx = mouse.x - particle.x;
                let dy = mouse.y - particle.y;
                let distance = (dx * dx + dy * dy).sqrt();

                if distance > 0.2 {
                    let inv_gravity = GRAVITY_STRENGTH / distance;
                    particle.dx += dx * inv_gravity * 3.0;
                    particle.dy += dy * inv_gravity * 3.0;
                }
            }

            let friction = FRICTION_PER_SECOND.powf(delta_time);
            particle.dx *= friction;
            particle.dy *= friction;
            particle.x += particle.dx * delta_time;
            particle.y += particle.dy * delta_time;

            if particle.x < area.width as f32
                && particle.y < (area.height * 2) as f32
                && particle.x >= 0.0f32
                && particle.y >= 0.0f32
            {
                let x = particle.x as u16;
                let y = particle.y as u16;
                let red = (particle.x / area.width as f32 * 255.0 * 0.8) as u8;
                let green = (particle.y / ((area.height * 2) as f32) * 255.0 * 0.8) as u8;
                let blue = (255.0 * 0.6) as u8;
                if let Some(cell) = unsafe_buf.buf().cell_mut((x, y / 2)) {
                    if y % 2 == 0 {
                        cell.set_fg(Color::Rgb(red, green, blue));
                    } else {
                        cell.set_bg(Color::Rgb(red, green, blue));
                    }
                }
            }
        });
    }
}

fn run(mut terminal: DefaultTerminal) -> io::Result<()> {
    let tick_len = std::time::Duration::from_millis(34);

    let particles = {
        let size = terminal.size()?;
        generate_particles(DENSITY, size.width, size.height * 2)
    };
    let mut simulation = Simulation {
        particles,
        time: Instant::now(),
        mouse: None,
    };

    loop {
        terminal.draw(|frame| {
            frame.render_stateful_widget(SimulationWidget, frame.area(), &mut simulation);
        })?;

        if event::poll(
            tick_len.saturating_sub(Instant::now().saturating_duration_since(simulation.time)),
        )? {
            match event::read()? {
                event::Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                    KeyCode::Char('q') => {
                        return Ok(());
                    }
                    KeyCode::Backspace => {
                        let size = terminal.size()?;
                        simulation.particles =
                            generate_particles(DENSITY, size.width, size.height * 2);
                        continue;
                    }
                    _ => {}
                },
                event::Event::Mouse(m) => {
                    if matches!(m.kind, MouseEventKind::Down(_) | MouseEventKind::Drag(_)) {
                        simulation.mouse = Some(Mouse {
                            x: m.column as f32,
                            y: (m.row * 2) as f32,
                        });
                    } else if matches!(m.kind, MouseEventKind::Up(_)) {
                        simulation.mouse = None;
                    }
                }
                _ => {}
            }
        }
    }
}

fn main() -> io::Result<()> {
    let mut terminal = ratatui::init();
    terminal.clear()?;
    execute!(io::stdout(), EnableMouseCapture)?;
    let app_result = run(terminal);
    execute!(io::stdout(), DisableMouseCapture)?;
    ratatui::restore();
    app_result
}
