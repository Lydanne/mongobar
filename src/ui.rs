use std::{
    error::Error,
    io,
    time::{Duration, Instant},
};

use ratatui::{
    backend::{Backend, CrosstermBackend},
    crossterm::{
        self,
        event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    },
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    symbols::{self, Marker},
    terminal::{Frame, Terminal},
    text::{Line, Masked, Span},
    widgets::{
        block::Title, Axis, Block, Borders, Chart, Dataset, Gauge, GraphType, LegendPosition, List,
        ListItem, Paragraph,
    },
};

use crate::mongobar::Mongobar;

#[derive(Clone)]
struct SinSignal {
    x: f64,
    interval: f64,
    period: f64,
    scale: f64,
}

impl SinSignal {
    const fn new(interval: f64, period: f64, scale: f64) -> Self {
        Self {
            x: 0.0,
            interval,
            period,
            scale,
        }
    }
}

impl Iterator for SinSignal {
    type Item = (f64, f64);
    fn next(&mut self) -> Option<Self::Item> {
        let point = (self.x, (self.x * 1.0 / self.period).sin() * self.scale);
        self.x += self.interval;
        Some(point)
    }
}

struct App {
    signal1: SinSignal,
    data1: Vec<(f64, f64)>,
    signal2: SinSignal,
    data2: Vec<(f64, f64)>,
    window: [f64; 2],

    progress: f64,
    log_scroll: u16,

    active_tabs: Vec<Span<'static>>,
    active_tab: usize,
    tabs_path: Vec<(Span<'static>, Vec<Span<'static>>)>,

    mongobar: Mongobar,
}

impl App {
    fn new(mongobar: Mongobar) -> Self {
        let mut signal1 = SinSignal::new(0.2, 3.0, 18.0);
        let mut signal2 = SinSignal::new(0.1, 2.0, 10.0);
        let data1 = signal1.by_ref().take(200).collect::<Vec<(f64, f64)>>();
        let data2 = signal2.by_ref().take(200).collect::<Vec<(f64, f64)>>();
        Self {
            signal1,
            data1,
            signal2,
            data2,
            window: [0.0, 20.0],

            progress: 0.5,
            log_scroll: 0,
            active_tabs: vec!["Stress".into(), "Replay".into(), "Quit".red()],
            active_tab: 0,
            tabs_path: vec![],

            mongobar,
        }
    }

    fn on_tick(&mut self) {
        self.data1.drain(0..5);
        self.data1.extend(self.signal1.by_ref().take(5));

        self.data2.drain(0..10);
        self.data2.extend(self.signal2.by_ref().take(10));

        self.window[0] += 1.0;
        self.window[1] += 1.0;
    }

    pub fn get_tabs_path_string(&self) -> String {
        self.tabs_path
            .iter()
            .map(|(tab, _)| tab.clone().to_string())
            .collect::<Vec<_>>()
            .join(" / ")
    }
}

pub fn main(mongobar: Mongobar) -> Result<(), Box<dyn Error>> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    let tick_rate = Duration::from_millis(1000);
    let app = App::new(mongobar);
    let res = run_app(&mut terminal, app, tick_rate);

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{err:?}");
    }

    Ok(())
}

fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    mut app: App,
    tick_rate: Duration,
) -> io::Result<()> {
    let mut last_tick = Instant::now();
    loop {
        terminal.draw(|f| ui(f, &app))?;

        let timeout = tick_rate.saturating_sub(last_tick.elapsed());
        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Char('q') {
                    return Ok(());
                }

                if key.code == KeyCode::Up {
                    if app.active_tab > 0 {
                        app.active_tab -= 1;
                    }
                }

                if key.code == KeyCode::Down {
                    if app.active_tab < app.active_tabs.len() - 1 {
                        app.active_tab += 1;
                    }
                }

                if key.code == KeyCode::Enter {
                    let tab = app.active_tabs[app.active_tab].clone();
                    if tab.to_string().contains("Quit") {
                        return Ok(());
                    } else if tab.to_string().contains("..") || tab.to_string().contains("Stop") {
                        let back = app.tabs_path.pop();
                        if let Some((tab, tabs)) = back {
                            app.active_tabs = tabs;
                            app.active_tab = 0;
                        }
                    } else {
                        let old = app.active_tabs.clone();
                        if tab.to_string().contains("Stress") {
                            app.active_tabs = vec!["..".gray(), "Start".light_green()];
                            app.tabs_path.push((tab, old));
                            app.active_tab = 1;
                        } else if tab.to_string().contains("Replay") {
                            app.active_tabs = vec!["..".gray(), "Start".light_green()];
                            app.tabs_path.push((tab, old));
                            app.active_tab = 1;
                        } else if tab.to_string().contains("Start") {
                            app.active_tabs =
                                vec!["Stop".red().bold(), "Boost+".yellow(), "Boost-".yellow()];
                            app.active_tab = 0;
                        }
                    }
                }
            }
        }
        if last_tick.elapsed() >= tick_rate {
            app.on_tick();
            last_tick = Instant::now();
        }
    }
}

fn ui(frame: &mut Frame, app: &App) {
    let area = frame.size();

    if app.get_tabs_path_string().contains("Stress") {
        render_stress_view(frame, area, app);
    } else if app.get_tabs_path_string().contains("Replay") {
        render_replay_view(frame, area, app);
    } else {
        render_main_view(frame, area, app);
    }
}

fn render_replay_view(frame: &mut Frame, area: Rect, app: &App) {
    let [tab, content] =
        Layout::horizontal([Constraint::Percentage(10), Constraint::Percentage(90)]).areas(area);

    render_tabs(frame, tab, app);
    render_title(frame, content, app, "will realize soon...");
}

fn render_main_view(frame: &mut Frame, area: Rect, app: &App) {
    let [tab, content] =
        Layout::horizontal([Constraint::Percentage(10), Constraint::Percentage(90)]).areas(area);

    let op_workdir = app.mongobar.op_workdir.to_str().unwrap();
    render_tabs(frame, tab, app);
    render_title(
        frame,
        content,
        app,
        &format!(
            "Welcome to Mongobar\n\nworkdir: {}\nconnect: {}\n\nPress Enter to start...",
            op_workdir,
            app.mongobar.config.uri.split('@').last().unwrap()
        ),
    );
}

fn render_title(f: &mut Frame, area: Rect, app: &App, title: &str) {
    let block = Block::new()
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::LightGreen));
    f.render_widget(block, area);
    let [_, title_block] =
        Layout::vertical([Constraint::Percentage(30), Constraint::Percentage(70)]).areas(area);
    let title = Paragraph::new(title).alignment(Alignment::Center);

    f.render_widget(title, title_block);
}

fn render_stress_view(frame: &mut Frame, area: Rect, app: &App) {
    let [tab, content] =
        Layout::horizontal([Constraint::Percentage(10), Constraint::Percentage(90)]).areas(area);
    let [chart, progress, log] = Layout::vertical([
        Constraint::Percentage(40),
        Constraint::Length(3),
        Constraint::Percentage(60),
    ])
    .areas(content);

    render_tabs(frame, tab, app);
    render_chart(frame, chart, app);
    render_progress(frame, progress, app);
    render_log(frame, log, app);
}

fn render_progress(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::new().borders(Borders::ALL);
    let gauge = Gauge::default()
        .block(block)
        .gauge_style(ratatui::style::Style::default().fg(ratatui::style::Color::Green))
        .label(format!("{:.0}%", app.progress * 100.0))
        .ratio(app.progress);
    f.render_widget(gauge, area);
}

fn render_tabs(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::new()
        .borders(Borders::ALL)
        .title(format!("Mongobar"));
    let items: Vec<ListItem> = app
        .active_tabs
        .iter()
        .enumerate()
        .map(|(i, t)| {
            if i == app.active_tab {
                ListItem::new(t.clone()).bg(Color::DarkGray)
            } else {
                ListItem::new(t.clone())
            }
        })
        .collect();
    let list = List::new(items).block(block);

    f.render_widget(list, area);
}

fn render_log(f: &mut Frame, area: Rect, app: &App) {
    let text = vec![
        Line::from("$ OPStress Bootstrapping"),
        Line::from("$ OPStress [1720515320] count: 403/s cost: 600.69ms progress: 0.08%"),
        Line::from("$ OPStress [1720515320] count: 403/s cost: 600.69ms progress: 0.08%"),
        Line::from("$ OPStress [1720515320] count: 403/s cost: 600.69ms progress: 0.08%"),
        Line::from("$ OPStress [1720515320] count: 403/s cost: 600.69ms progress: 0.08%"),
        Line::from("$ OPStress [1720515320] count: 403/s cost: 600.69ms progress: 0.08%"),
        Line::from("$ OPStress [1720515320] count: 403/s cost: 600.69ms progress: 0.08%"),
        Line::from("$ OPStress [1720515320] count: 403/s cost: 600.69ms progress: 0.08%"),
        Line::from("$ OPStress [1720515320] count: 403/s cost: 600.69ms progress: 0.08%"),
        Line::from("$ OPStress [1720515320] count: 403/s cost: 600.69ms progress: 0.08%"),
        Line::from("$ OPStress [1720515320] count: 403/s cost: 600.69ms progress: 0.08%"),
        Line::from("$ OPStress [1720515320] count: 403/s cost: 600.69ms progress: 0.08%"),
        Line::from("$ OPStress [1720515320] count: 403/s cost: 600.69ms progress: 0.08%"),
        Line::from("$ OPStress [1720515320] count: 403/s cost: 600.69ms progress: 0.08%"),
        Line::from("$ OPStress [1720515320] count: 403/s cost: 600.69ms progress: 0.08%"),
        Line::from("$ OPStress [1720515320] count: 403/s cost: 600.69ms progress: 0.08%"),
    ];
    let block = Block::new().borders(Borders::ALL).title(format!("Logs"));
    let paragraph = Paragraph::new(text.clone())
        .style(Style::default().fg(Color::Gray))
        .block(block)
        .scroll((app.log_scroll, 0));
    f.render_widget(paragraph, area);
}

fn render_chart(f: &mut Frame, area: Rect, app: &App) {
    let x_labels = vec![
        Span::styled(
            format!("{}", app.window[0]),
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!("{}", (app.window[0] + app.window[1]) / 2.0)),
        Span::styled(
            format!("{}", app.window[1]),
            Style::default().add_modifier(Modifier::BOLD),
        ),
    ];
    let datasets = vec![
        Dataset::default()
            .name("data2")
            .marker(symbols::Marker::Dot)
            .style(Style::default().fg(Color::Cyan))
            .data(&app.data1),
        Dataset::default()
            .name("data3")
            .marker(symbols::Marker::Braille)
            .style(Style::default().fg(Color::Yellow))
            .data(&app.data2),
    ];

    let chart = Chart::new(datasets)
        .block(Block::bordered().title(app.get_tabs_path_string()))
        .x_axis(
            Axis::default()
                // .title("Progress")
                .style(Style::default().fg(Color::Gray))
                // .labels(x_labels)
                .bounds(app.window),
        )
        .y_axis(
            Axis::default()
                // .title("Query")
                .style(Style::default().fg(Color::Gray))
                // .labels(vec!["-20".bold(), "0".into(), "20".bold()])
                .bounds([-20.0, 20.0]),
        );

    f.render_widget(chart, area);
}
