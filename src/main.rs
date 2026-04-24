use std::fs;
use std::io::{self, Stdout};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode};
use rand::seq::SliceRandom;
use ratatui::Frame;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use serde::{Deserialize, Serialize};

const DEFAULT_BANK_PATH: &str = "exam_bank.yaml";

#[derive(Parser)]
#[command(author, version, about = "Unified exam preparation trainer")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    Study {
        #[arg(short, long, default_value = DEFAULT_BANK_PATH)]
        bank: PathBuf,
        #[arg(short, long)]
        deck: Option<String>,
    },
    List {
        #[arg(short, long, default_value = DEFAULT_BANK_PATH)]
        bank: PathBuf,
    },
    Convert {
        #[arg(short, long)]
        input: PathBuf,
        #[arg(short, long, default_value = DEFAULT_BANK_PATH)]
        output: PathBuf,
        #[arg(short, long, default_value = "Project Defense")]
        title: String,
        #[arg(long, default_value = "general")]
        deck_id: String,
        #[arg(long, default_value = "General")]
        deck_name: String,
    },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct ExamBank {
    title: String,
    decks: Vec<Deck>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct Deck {
    id: String,
    name: String,
    cards: Vec<Card>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct Card {
    question: String,
    answer: String,
}

#[derive(Debug, Clone)]
struct SessionCard {
    question: String,
    answer: String,
}

#[derive(Debug, Clone, Copy)]
enum SessionAction {
    Show,
    Know,
    DontKnow,
    Quit,
}

#[derive(Debug)]
struct SessionState {
    queue: Vec<SessionCard>,
    total: usize,
    mastered: usize,
    answer_shown: bool,
    finished: bool,
    aborted: bool,
}

impl SessionState {
    fn new(mut cards: Vec<SessionCard>) -> Result<Self> {
        if cards.is_empty() {
            bail!("question bank is empty");
        }

        let mut rng = rand::rng();
        cards.shuffle(&mut rng);

        let total = cards.len();
        Ok(Self {
            queue: cards,
            total,
            mastered: 0,
            answer_shown: false,
            finished: false,
            aborted: false,
        })
    }

    fn current(&self) -> Option<&SessionCard> {
        self.queue.first()
    }

    fn remaining(&self) -> usize {
        self.queue.len()
    }

    fn apply(&mut self, action: SessionAction) {
        match action {
            SessionAction::Show if !self.answer_shown => {
                self.answer_shown = true;
            }
            SessionAction::Know => {
                if !self.queue.is_empty() {
                    self.queue.remove(0);
                    self.mastered += 1;
                    self.answer_shown = false;
                }
            }
            SessionAction::DontKnow => {
                if !self.queue.is_empty() {
                    let card = self.queue.remove(0);
                    self.queue.push(card);
                    let mut rng = rand::rng();
                    self.queue.shuffle(&mut rng);
                    self.answer_shown = false;
                }
            }
            SessionAction::Quit => {
                self.aborted = true;
                self.finished = true;
            }
            SessionAction::Show => {}
        }

        if self.queue.is_empty() {
            self.finished = true;
        }
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command.unwrap_or(Command::Study {
        bank: PathBuf::from(DEFAULT_BANK_PATH),
        deck: None,
    }) {
        Command::Study { bank, deck } => run_study(&bank, deck.as_deref()),
        Command::List { bank } => run_list(&bank),
        Command::Convert {
            input,
            output,
            title,
            deck_id,
            deck_name,
        } => run_convert(&input, &output, &title, &deck_id, &deck_name),
    }
}

fn run_study(path: &Path, deck_id: Option<&str>) -> Result<()> {
    let bank = load_bank(path)?;
    let deck = select_deck(&bank, deck_id)?;
    let cards = deck
        .cards
        .iter()
        .cloned()
        .map(|card| SessionCard {
            question: card.question,
            answer: card.answer,
        })
        .collect::<Vec<_>>();

    let mut state = SessionState::new(cards)?;
    let summary = run_tui(&bank.title, &deck.name, &mut state)?;
    println!("{summary}");
    Ok(())
}

fn run_list(path: &Path) -> Result<()> {
    let bank = load_bank(path)?;
    println!("{}", bank.title);
    for deck in &bank.decks {
        println!("- {} ({}) [{} cards]", deck.name, deck.id, deck.cards.len());
    }
    Ok(())
}

fn run_convert(input: &Path, output: &Path, title: &str, deck_id: &str, deck_name: &str) -> Result<()> {
    let content = fs::read_to_string(input)
        .with_context(|| format!("failed to read {}", input.display()))?;
    let cards = parse_legacy_cards(&content);

    if cards.is_empty() {
        bail!("no question-answer pairs found in {}", input.display());
    }

    let bank = ExamBank {
        title: title.to_string(),
        decks: vec![Deck {
            id: deck_id.to_string(),
            name: deck_name.to_string(),
            cards,
        }],
    };

    let yaml = serde_yaml::to_string(&bank).context("failed to serialize YAML bank")?;
    fs::write(output, yaml).with_context(|| format!("failed to write {}", output.display()))?;
    println!("Wrote {}", output.display());
    Ok(())
}

fn load_bank(path: &Path) -> Result<ExamBank> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    let bank: ExamBank = serde_yaml::from_str(&content)
        .with_context(|| format!("failed to parse {}", path.display()))?;

    if bank.decks.is_empty() {
        bail!("{} does not contain any decks", path.display());
    }

    Ok(bank)
}

fn select_deck<'a>(bank: &'a ExamBank, deck_id: Option<&str>) -> Result<&'a Deck> {
    if let Some(deck_id) = deck_id {
        return bank
            .decks
            .iter()
            .find(|deck| deck.id == deck_id)
            .with_context(|| format!("deck '{deck_id}' was not found"));
    }

    bank.decks
        .first()
        .context("question bank does not contain any decks")
}

fn parse_legacy_cards(content: &str) -> Vec<Card> {
    content
        .split("\n\n")
        .filter_map(|block| {
            let lines = block
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .collect::<Vec<_>>();

            if lines.len() < 2 {
                return None;
            }

            let question = lines[0]
                .trim_start_matches(|ch: char| ch.is_ascii_digit() || ch == '.' || ch == '-' || ch.is_whitespace())
                .trim()
                .to_string();
            let answer = lines[1..].join(" ").trim_start_matches("Ответ:").trim().to_string();

            if question.is_empty() || answer.is_empty() {
                None
            } else {
                Some(Card { question, answer })
            }
        })
        .collect()
}

fn run_tui(title: &str, deck_name: &str, state: &mut SessionState) -> Result<String> {
    enable_raw_mode().context("failed to enable raw mode")?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen).context("failed to enter alternate screen")?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).context("failed to initialize terminal")?;

    let result = session_loop(&mut terminal, title, deck_name, state);
    restore_terminal(terminal)?;

    result
}

fn session_loop(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    title: &str,
    deck_name: &str,
    state: &mut SessionState,
) -> Result<String> {
    loop {
        terminal.draw(|frame| draw_ui(frame, title, deck_name, state))?;

        if state.finished {
            return Ok(summary_line(state));
        }

        if let Event::Key(key) = event::read().context("failed to read terminal event")? {
            if key.kind != KeyEventKind::Press {
                continue;
            }

            match map_action(key.code, state.answer_shown) {
                Some(action) => state.apply(action),
                None => {}
            }
        }
    }
}

fn map_action(code: KeyCode, answer_shown: bool) -> Option<SessionAction> {
    match code {
        KeyCode::Char('1') if !answer_shown => Some(SessionAction::Show),
        KeyCode::Char('2') | KeyCode::Enter => Some(SessionAction::Know),
        KeyCode::Char('3') => Some(SessionAction::DontKnow),
        KeyCode::Char('q') | KeyCode::Esc => Some(SessionAction::Quit),
        _ => None,
    }
}

fn draw_ui(frame: &mut Frame, title: &str, deck_name: &str, state: &SessionState) {
    let area = frame.area();
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(if state.answer_shown { 8 } else { 4 }),
            Constraint::Length(3),
        ])
        .split(area);

    let header = Paragraph::new(Text::from(vec![
        Line::from(Span::styled(
            title,
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            format!("Deck: {deck_name}"),
            Style::default().fg(Color::Yellow),
        )),
        Line::from(vec![
            Span::styled("1", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::raw(" show answer  "),
            Span::styled("2", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::raw(" know  "),
            Span::styled("3", Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
            Span::raw(" don't know  "),
            Span::styled("q", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            Span::raw(" quit"),
        ]),
    ]))
    .alignment(Alignment::Center)
    .block(Block::default().borders(Borders::ALL).title("Trainer").border_style(Style::default().fg(Color::Blue)));
    frame.render_widget(header, layout[0]);

    let progress = Paragraph::new(Line::from(vec![
        Span::styled("Progress: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(format!("{}/{}", state.mastered, state.total)),
        Span::raw("    "),
        Span::styled("Remaining in queue: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(state.remaining().to_string()),
    ]))
    .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)));
    frame.render_widget(progress, layout[1]);

    let question = state
        .current()
        .map(|card| card.question.as_str())
        .unwrap_or("All questions completed.");
    let question_widget = Paragraph::new(question)
        .wrap(Wrap { trim: true })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Question")
                .border_style(Style::default().fg(Color::Magenta)),
        );
    frame.render_widget(question_widget, layout[2]);

    let answer_text = if state.answer_shown {
        state.current().map(|card| card.answer.as_str()).unwrap_or("")
    } else {
        "Press 1 to reveal the answer."
    };
    let answer_style = if state.answer_shown {
        Style::default().fg(Color::Green)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let answer_widget = Paragraph::new(answer_text)
        .style(answer_style)
        .wrap(Wrap { trim: true })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Answer")
                .border_style(Style::default().fg(Color::Green)),
        );
    frame.render_widget(answer_widget, layout[3]);

    let footer_message = if state.finished {
        summary_line(state)
    } else if state.answer_shown {
        "Press 2 if you know it now, 3 if you still don't, q to quit.".to_string()
    } else {
        "Press 1 to show the answer, 2 if you know it, 3 if you don't, q to quit.".to_string()
    };
    let footer = Paragraph::new(footer_message)
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Blue)));
    frame.render_widget(footer, layout[4]);

    if state.finished {
        let popup_area = centered_rect(60, 20, area);
        frame.render_widget(Clear, popup_area);
        let popup = Paragraph::new(summary_line(state))
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(if state.aborted { "Stopped" } else { "Completed" })
                    .border_style(Style::default().fg(if state.aborted { Color::Yellow } else { Color::Green })),
            );
        frame.render_widget(popup, popup_area);
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, area: ratatui::layout::Rect) -> ratatui::layout::Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}

fn restore_terminal(mut terminal: Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
    disable_raw_mode().context("failed to disable raw mode")?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen).context("failed to leave alternate screen")?;
    terminal.show_cursor().context("failed to show cursor")?;
    Ok(())
}

fn summary_line(state: &SessionState) -> String {
    if state.aborted {
        format!("Stopped. Progress: {}/{}", state.mastered, state.total)
    } else {
        format!("Completed. Reviewed questions: {}/{}", state.mastered, state.total)
    }
}
