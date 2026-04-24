use std::fs;
use std::io::{self, Stdout};
use std::path::{Path, PathBuf};

use crate::r#const::DEFAULT_BANK_PATH;
use anyhow::{Context, Result, bail};
use card::Card;
use clap::Parser;
use cli::{Cli, Command};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{EnterAlternateScreen, enable_raw_mode};
use deck::Deck;
use exam_bank::ExamBank;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use session_action::SessionAction;
use session_card::SessionCard;
use session_state::SessionState;

mod card;
mod cli;
mod r#const;
mod deck;
mod drawing;
mod exam_bank;
mod session_action;
mod session_card;
mod session_state;

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
        println!(
            "- {} ({}) [{} карточек]",
            deck.name,
            deck.id,
            deck.cards.len()
        );
    }
    Ok(())
}

fn run_convert(
    input: &Path,
    output: &Path,
    title: &str,
    deck_id: &str,
    deck_name: &str,
) -> Result<()> {
    let content = fs::read_to_string(input)
        .with_context(|| format!("не удалось прочитать {}", input.display()))?;
    let cards = parse_legacy_cards(&content);

    if cards.is_empty() {
        bail!("в {} не найдено пар вопрос-ответ", input.display());
    }

    let bank = ExamBank {
        title: title.to_string(),
        decks: vec![Deck {
            id: deck_id.to_string(),
            name: deck_name.to_string(),
            cards,
        }],
    };

    let yaml = serde_yaml::to_string(&bank).context("не удалось сериализовать YAML-банк")?;
    fs::write(output, yaml).with_context(|| format!("не удалось записать {}", output.display()))?;
    println!("Файл {} записан", output.display());
    Ok(())
}

fn load_bank(path: &Path) -> Result<ExamBank> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("не удалось прочитать {}", path.display()))?;
    let bank: ExamBank = serde_yaml::from_str(&content)
        .with_context(|| format!("не удалось разобрать {}", path.display()))?;

    if bank.decks.is_empty() {
        bail!("{} не содержит ни одной колоды", path.display());
    }

    Ok(bank)
}

fn select_deck<'a>(bank: &'a ExamBank, deck_id: Option<&str>) -> Result<&'a Deck> {
    if let Some(deck_id) = deck_id {
        return bank
            .decks
            .iter()
            .find(|deck| deck.id == deck_id)
            .with_context(|| format!("колода '{deck_id}' не найдена"));
    }

    bank.decks
        .first()
        .context("банк вопросов не содержит ни одной колоды")
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
                .trim_start_matches(|ch: char| {
                    ch.is_ascii_digit() || ch == '.' || ch == '-' || ch.is_whitespace()
                })
                .trim()
                .to_string();
            let answer = lines[1..]
                .join(" ")
                .trim_start_matches("Ответ:")
                .trim()
                .to_string();

            if question.is_empty() || answer.is_empty() {
                None
            } else {
                Some(Card { question, answer })
            }
        })
        .collect()
}

fn run_tui(title: &str, deck_name: &str, state: &mut SessionState) -> Result<String> {
    enable_raw_mode().context("не удалось включить raw mode")?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen).context("не удалось перейти в альтернативный экран")?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).context("не удалось инициализировать терминал")?;

    let result = session_loop(&mut terminal, title, deck_name, state);
    drawing::restore_terminal(terminal)?;

    result
}

fn session_loop(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    title: &str,
    deck_name: &str,
    state: &mut SessionState,
) -> Result<String> {
    loop {
        terminal.draw(|frame| drawing::draw_ui(frame, title, deck_name, state))?;

        if state.finished {
            return Ok(drawing::summary_line(state));
        }

        if let Event::Key(key) = event::read().context("не удалось прочитать событие терминала")?
        {
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
