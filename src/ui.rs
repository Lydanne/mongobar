use std::{
    borrow::BorrowMut,
    error::Error,
    io,
    sync::{Arc, Mutex},
    thread,
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
use tokio::runtime::Builder;

use crate::{
    indicator::{self, Metric},
    mongobar::Mongobar,
};

struct App {
    log_scroll: u16,

    active_tabs: Vec<Span<'static>>,
    active_tab: usize,
    tabs_path: Vec<(Span<'static>, Vec<Span<'static>>)>,

    target: String,
    indicator: indicator::Indicator,
    signal: Arc<crate::signal::Signal>, // 0 初始状态，1 是停止，2 是停止成功

    boot_at: i64,
    current_at: Metric,
    stress_start_at: Metric,

    query_chart_data: Vec<(f64, f64)>,
    query_count_max: f64,
    query_count_min: f64,
    last_query_count: usize,
    diff_query_count: usize,

    cost_chart_data: Vec<(f64, f64)>,
    cost_max: f64,
    cost_min: f64,
    last_cost: f64,
    diff_cost: f64,

    v: f64,
}

impl App {
    fn new() -> Self {
        let indic = indicator::Indicator::new().init(vec![
            "boot_worker".to_string(),
            "query_count".to_string(),
            "cost_ms".to_string(),
            "progress".to_string(),
            "logs".to_string(),
            "progress_total".to_string(),
            "thread_count".to_string(),
            "done_worker".to_string(),
            "dyn_threads".to_string(),
        ]);
        Self {
            log_scroll: 0,
            active_tabs: vec!["Stress".into(), "Replay".into(), "Quit".red()],
            active_tab: 0,
            tabs_path: vec![],

            target: "".to_string(),

            indicator: indic,
            signal: Arc::new(crate::signal::Signal::new()),

            boot_at: chrono::Local::now().timestamp(), // s
            current_at: Metric::new(),                 // s
            stress_start_at: Metric::new(),            // s

            query_count_max: f64::MIN,
            query_count_min: f64::MAX,
            query_chart_data: vec![],
            last_query_count: 0,
            diff_query_count: 0,

            cost_max: f64::MIN,
            cost_min: f64::MAX,
            cost_chart_data: vec![],
            last_cost: 0.,
            diff_cost: 0.,

            v: 0.0,
        }
    }

    fn update_current_at(&self) {
        if self.signal.get() == 0 {
            self.current_at
                .set(chrono::Local::now().timestamp() as usize);
        }
    }

    fn update_stress_start_at(&self) {
        self.stress_start_at
            .set(chrono::Local::now().timestamp() as usize);
    }

    fn reset(&mut self) {
        self.query_chart_data.clear();
        self.query_count_max = f64::MIN;
        self.query_count_min = f64::MAX;

        self.cost_chart_data.clear();
        self.cost_max = f64::MIN;
        self.cost_min = f64::MAX;
    }

    fn on_tick(&mut self, tick_index: usize) {
        if self.signal.get() != 0 {
            return;
        }
        let current_at = chrono::Local::now().timestamp() as f64;
        let stress_start_at = self.stress_start_at.get() as f64;
        let dur = current_at - stress_start_at;
        {
            let query_count = self.indicator.take("query_count").unwrap().get() as f64;
            if tick_index == 0 {
                let diff_query_count = query_count - self.last_query_count as f64;
                self.last_query_count = query_count as usize;
                self.diff_query_count = diff_query_count as usize;
            }
            let v = self.diff_query_count as f64;

            if !(v.is_infinite() || v.is_nan()) {
                if v > self.query_count_max {
                    self.query_count_max = v;
                }
                if v < self.query_count_min {
                    self.query_count_min = v;
                }
            }

            let v = normalize_to_100(v, self.query_count_min, self.query_count_max);

            self.query_chart_data
                .push((self.query_chart_data.len() as f64, v));

            if self.query_chart_data.len() > 200 {
                self.query_chart_data.remove(0);
                self.query_chart_data
                    .iter_mut()
                    .enumerate()
                    .for_each(|(i, (x, _))| {
                        *x = i as f64;
                    });
            }
        }
        {
            let cost = self.indicator.take("cost_ms").unwrap().get() as f64;
            if tick_index == 0 {
                let diff_cost = cost - self.last_cost;
                self.last_cost = cost;
                self.diff_cost = (diff_cost / self.diff_query_count as f64);
            }
            let v = self.diff_cost as f64;
            if !(v.is_infinite() || v.is_nan()) {
                if v > self.cost_max {
                    self.cost_max = v;
                }
                if v < self.cost_min {
                    self.cost_min = v;
                }
            }
            let v = normalize_to_100(v, self.cost_min, self.cost_max);

            self.cost_chart_data
                .push((self.cost_chart_data.len() as f64, v));

            if self.cost_chart_data.len() > 200 {
                self.cost_chart_data.remove(0);
                self.cost_chart_data
                    .iter_mut()
                    .enumerate()
                    .for_each(|(i, (x, _))| {
                        *x = i as f64;
                    });
            }
        }

        // if (dur as u32) % 5 == 0 {
        //     // self.cost_max = f64::MIN;
        //     self.cost_min = f64::MAX;
        //     // self.query_count_max = f64::MIN;
        //     self.query_count_min = f64::MAX;
        // }
    }

    pub fn get_tabs_path(&self) -> String {
        self.tabs_path
            .iter()
            .map(|(tab, _)| tab.clone().to_string())
            .collect::<Vec<_>>()
            .join(" > ")
    }
}

pub fn boot(target: &str) -> Result<(), Box<dyn Error>> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    let tick_rate = Duration::from_millis(100);
    let mut app = App::new();
    app.target = target.to_string();
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
    let mut tick_index = 0;
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
                        if let Some((_, tabs)) = back {
                            app.active_tabs = tabs;
                            app.active_tab = 0;
                        }
                        if tab.to_string().contains("Stop") {
                            app.signal.set(1);
                        }
                    } else {
                        let prev = app.active_tabs.clone();
                        if tab.to_string().contains("Stress") {
                            app.active_tabs = vec!["..".gray(), "Start".light_green()];
                            app.tabs_path.push((tab, prev));
                            app.active_tab = 1;
                        } else if tab.to_string().contains("Replay") {
                            app.active_tabs = vec!["..".gray(), "Start".light_green()];
                            app.tabs_path.push((tab, prev));
                            app.active_tab = 1;
                        } else if tab.to_string().contains("Start") {
                            app.active_tabs = vec![
                                "Stop".red().bold(),
                                "Boost+10".yellow(),
                                "Boost+100".yellow(),
                                "Boost+1000".yellow(),
                            ];
                            app.tabs_path.push((tab, prev));
                            app.active_tab = 0;
                            if app.get_tabs_path().starts_with("Stress > Start") {
                                let target = app.target.clone();
                                let indicator = app.indicator.clone();
                                let inner_indicator = app.indicator.clone();
                                let signal = app.signal.clone();
                                signal.set(0);
                                inner_indicator.reset();
                                app.reset();
                                app.update_stress_start_at();
                                thread::spawn(move || {
                                    let runtime =
                                        Builder::new_multi_thread().enable_all().build().unwrap();
                                    let inner_signal = signal.clone();
                                    runtime.block_on(async {
                                        let r = Mongobar::new(&target)
                                            .set_signal(signal)
                                            .set_indicator(indicator)
                                            .init()
                                            .op_stress()
                                            .await;
                                        if let Err(err) = r {
                                            eprintln!("Error: {}", err);
                                        }
                                    });
                                    inner_signal.set(2);
                                    inner_indicator
                                        .take("logs")
                                        .unwrap()
                                        .push("Done".to_string());
                                });
                            }
                        } else if tab.to_string().starts_with("Boost+1000")
                            && app.get_tabs_path().starts_with("Stress > Start")
                        {
                            let dyn_threads = app.indicator.take("dyn_threads").unwrap();

                            dyn_threads.set(dyn_threads.get() + 1000);
                        } else if tab.to_string().starts_with("Boost+100")
                            && app.get_tabs_path().starts_with("Stress > Start")
                        {
                            let dyn_threads = app.indicator.take("dyn_threads").unwrap();

                            dyn_threads.set(dyn_threads.get() + 100);
                        } else if tab.to_string().starts_with("Boost+10")
                            && app.get_tabs_path().starts_with("Stress > Start")
                        {
                            let dyn_threads = app.indicator.take("dyn_threads").unwrap();

                            dyn_threads.set(dyn_threads.get() + 10);
                        }
                    }
                }
            }
        }
        if last_tick.elapsed() >= tick_rate {
            app.on_tick(tick_index);
            last_tick = Instant::now();
            tick_index = tick_index + 1;
            tick_index = tick_index % 10;
        }
    }
}

fn ui(frame: &mut Frame, app: &App) {
    let area = frame.size();

    if app.get_tabs_path().starts_with("Stress > Start") {
        app.update_current_at();
        render_stress_view(frame, area, app);
    } else if app.get_tabs_path().starts_with("Stress") {
        render_stress_start_view(frame, area, app);
    } else if app.get_tabs_path().starts_with("Replay") {
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

fn render_stress_start_view(frame: &mut Frame, area: Rect, app: &App) {
    let [tab, content] =
        Layout::horizontal([Constraint::Percentage(10), Constraint::Percentage(90)]).areas(area);

    render_tabs(frame, tab, app);
    render_title(
        frame,
        content,
        app,
        &format!(
            "Stress\n\nStatus:[{}]\n\nPress Enter to start...",
            match app.signal.get() {
                0 => "Init",
                1 => "Stop",
                2 => "Stopped",
                _ => "Unknown",
            }
        ),
    );
}

fn render_main_view(frame: &mut Frame, area: Rect, app: &App) {
    let [tab, content] =
        Layout::horizontal([Constraint::Percentage(10), Constraint::Percentage(90)]).areas(area);

    render_tabs(frame, tab, app);
    render_title(
        frame,
        content,
        app,
        &format!(
            "Welcome to Mongobar\n\nCurrent: {}\n\nPress Enter to start...",
            &app.target,
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
    let progress = app.indicator.take("progress").unwrap().get();
    let progress_total = app.indicator.take("progress_total").unwrap().get();
    if progress_total == 0 {
        let block = Block::new().borders(Borders::ALL);
        let gauge = Gauge::default()
            .block(block)
            .gauge_style(ratatui::style::Style::default().fg(ratatui::style::Color::Green))
            .label(format!("count: {}", progress))
            .ratio(0.);
        f.render_widget(gauge, area);
    } else {
        let mut current_progress = progress as f64 / progress_total as f64;
        if current_progress.is_nan() {
            current_progress = 0.0;
        }

        if current_progress > 0.999 {
            current_progress = 1.0
        }

        let block = Block::new().borders(Borders::ALL);
        let gauge = Gauge::default()
            .block(block)
            .gauge_style(ratatui::style::Style::default().fg(ratatui::style::Color::Green))
            .label(format!("{:.2}%", current_progress * 100.0))
            .ratio(current_progress);
        f.render_widget(gauge, area);
    }
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
    let logs = app.indicator.take("logs").unwrap();
    let cost_ms = app.indicator.take("cost_ms").unwrap().get();
    let query_count = app.indicator.take("query_count").unwrap().get();
    let thread_count = app.indicator.take("thread_count").unwrap().get();
    let boot_worker = app.indicator.take("boot_worker").unwrap().get();
    let dyn_threads = app.indicator.take("dyn_threads").unwrap().get();

    let mut text = vec![
        Line::from("> OPStress Bootstrapping"),
        Line::from(format!(
            "> Thread: {}/{}+{}",
            boot_worker, thread_count, dyn_threads
        )),
        Line::from(format!(
            "> Query : {:.2}/s {}/s",
            (query_count as f64) / (app.current_at.get() - app.stress_start_at.get()) as f64,
            app.diff_query_count
        )),
        Line::from(format!(
            "> Cost  : {:.2}ms {:.2}/ms",
            (cost_ms as f64) / query_count as f64,
            app.diff_cost
        )),
        Line::from(format!(
            "> Query Stats: min({:.2}) max({:.2})",
            app.query_count_min, app.query_count_max,
        )),
        Line::from(format!(
            "> Cost Stats: min({:.2}) max({:.2})",
            app.cost_min, app.cost_max,
        )),
    ];
    logs.logs().iter().for_each(|v| {
        text.push(Line::from(format!("> {}", v.as_str())));
    });
    let block = Block::new().borders(Borders::ALL).title(format!("Console"));
    let paragraph = Paragraph::new(text.clone())
        .style(Style::default().fg(Color::Gray))
        .block(block)
        .scroll((app.log_scroll, 0));
    f.render_widget(paragraph, area);
}

fn render_chart(f: &mut Frame, area: Rect, app: &App) {
    // let x_labels = vec![
    //     Span::styled(
    //         format!("{}", app.window[0]),
    //         Style::default().add_modifier(Modifier::BOLD),
    //     ),
    //     Span::raw(format!("{}", (app.window[0] + app.window[1]) / 2.0)),
    //     Span::styled(
    //         format!("{}", app.window[1]),
    //         Style::default().add_modifier(Modifier::BOLD),
    //     ),
    // ];
    let datasets = vec![
        Dataset::default()
            .name("Query")
            .marker(symbols::Marker::Braille)
            .style(Style::default().fg(Color::Cyan))
            .data(&app.query_chart_data),
        Dataset::default()
            .name("Cost")
            .marker(symbols::Marker::Braille)
            .style(Style::default().fg(Color::Yellow))
            .data(&app.cost_chart_data),
    ];

    let chart: Chart = Chart::new(datasets)
        .block(Block::bordered().title(app.get_tabs_path()))
        .x_axis(
            Axis::default()
                // .title("Progress")
                .style(Style::default().fg(Color::Gray))
                // .labels(x_labels)
                .bounds([0., 200.]),
        )
        .y_axis(
            Axis::default()
                // .title("Query")
                .style(Style::default().fg(Color::Gray))
                // .labels(vec!["-20".bold(), "0".into(), "20".bold()])
                .bounds([0., 100.]),
        );

    f.render_widget(chart, area);
}

fn normalize_to_100(x: f64, min: f64, max: f64) -> f64 {
    ((x - min) / (max - min)) * 100.0
}
