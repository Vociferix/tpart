use std::io;
use std::time::Instant;

use ratatui::{
    crossterm::{execute, event::{self, KeyCode, KeyEventKind, MouseEventKind, EnableMouseCapture}},
    widgets::Widget,
    layout::Rect,
    buffer::Buffer,
    style::Color,
    DefaultTerminal,
};

use rand::Rng;

const NUM_PARTICLES: usize = 2000;
const GRAVITY_STRENGTH: f32 = 1.2f32;
const FRICTION_PER_SECOND: f32 = 0.7f32;

const FULL_BLOCK: &'static str = "█";
const UPPER_BLOCK: &'static str = "▀";
const LOWER_BLOCK: &'static str = "▄";
const NO_BLOCK: &'static str = " ";

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

struct Particles(Vec<Particle>);

fn generate_particles(count: usize, width: u16, height: u16) -> Particles {
    let mut rng = rand::thread_rng();
    let mut particles = Vec::with_capacity(count);

    for _ in 0..count {
        particles.push(Particle {
            x: rng.gen_range(0.0..width as f32),
            y: rng.gen_range(0.0..height as f32),
            dx: rng.gen_range(-1.0..1.0),
            dy: rng.gen_range(-1.0..1.0),
        });
    }

    Particles(particles)
}

fn step(particles: &mut [Particle], delta_time: f32, mouse: Option<Mouse>) {
    for particle in particles.iter_mut() {
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
    }
}

impl Widget for &Particles {
    fn render(self, area: Rect, buf: &mut Buffer)
        where Self: Sized
    {
        for x in area.x..(area.x + area.width) {
            for y in area.y..(area.y + area.height) {
                if let Some(cell) = buf.cell_mut((x, y)) {
                    if cell.symbol() != UPPER_BLOCK {
                        cell.set_symbol(UPPER_BLOCK);
                    }
                    cell.set_fg(Color::Black);
                    cell.set_bg(Color::Black);
                }
            }
        }

        for particle in &self.0 {
            let x = particle.x;
            let y = particle.y;
            if x < area.width as f32 && y < (area.height * 2) as f32 && x >= 0.0f32 && y >= 0.0f32 {
                let x = x as u16;
                let y = y as u16;
                let red = (particle.x / area.width as f32 * 255.0 * 0.8) as u8;
                let green = (particle.y / (area.height * 2) as f32 * 255.0 * 0.8) as u8;
                let blue = (255.0*0.6) as u8;
                if let Some(cell) = buf.cell_mut((x, y / 2)) {
                    if y % 2 == 0 {
                        cell.set_fg(Color::Rgb(red, green, blue));
                        //cell.set_fg(Color::White);
                    } else {
                        cell.set_bg(Color::Rgb(red, green, blue));
                        //cell.set_bg(Color::White);
                    }
                }
            }
        }
    }
}

fn update_time(prev_time: &mut Instant) -> f32 {
    let curr = Instant::now();
    let delta = curr.saturating_duration_since(*prev_time);
    *prev_time = curr;
    delta.as_secs_f32()
}

fn run(mut terminal: DefaultTerminal) -> io::Result<()> {
    let size = terminal.size()?;
    let tick_len = std::time::Duration::from_millis(34);

    let mut particles = generate_particles(NUM_PARTICLES, size.width, size.height * 2);

    let mut prev_time = Instant::now();

    let mut mouse: Option<Mouse> = None;

    loop {
        terminal.draw(|frame| {
            frame.render_widget(&particles, frame.area());
        })?;

        if event::poll(tick_len)? {
            match event::read()? {
                event::Event::Key(key) if key.kind == KeyEventKind::Press => {
                    match key.code {
                        KeyCode::Char('q') => {
                            return Ok(());
                        },
                        KeyCode::Backspace => {
                            particles = generate_particles(NUM_PARTICLES, size.width, size.height * 2);
                            continue;
                        },
                        _ => {},
                    }
                },
                event::Event::Mouse(m) => {
                    if matches!(m.kind, MouseEventKind::Down(_) | MouseEventKind::Drag(_)) {
                        mouse = Some(Mouse { x: m.column as f32, y: (m.row * 2) as f32 });
                    } else if matches!(m.kind, MouseEventKind::Up(_)) {
                        mouse = None;
                    } else if matches!(m.kind, MouseEventKind::Moved) && mouse.is_some() {
                        mouse = Some(Mouse { x: m.column as f32, y: (m.row * 2) as f32 });
                    }
                },
                _ => {
                }
            }
        }
        step(&mut particles.0, update_time(&mut prev_time), mouse);
    }
}

fn main() -> io::Result<()> {
    let mut terminal = ratatui::init();
    terminal.clear()?;
    execute!(io::stdout(), EnableMouseCapture)?;
    let app_result = run(terminal);
    ratatui::restore();
    app_result
}
