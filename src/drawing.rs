use crate::session_state::SessionState;
use anyhow::Context;
use crossterm::execute;
use crossterm::terminal::{LeaveAlternateScreen, disable_raw_mode};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::prelude::{Color, Line, Modifier, Span, Style, Text};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::{Frame, Terminal};
use std::io::Stdout;

pub fn draw_ui(frame: &mut Frame, title: &str, deck_name: &str, state: &SessionState) {
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
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            format!("Колода: {deck_name}"),
            Style::default().fg(Color::Yellow),
        )),
        Line::from(vec![
            Span::styled(
                "1",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" показать ответ  "),
            Span::styled(
                "2",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" знаю  "),
            Span::styled(
                "3",
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" не знаю  "),
            Span::styled(
                "q",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
            Span::raw(" выход"),
        ]),
    ]))
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title("Тренажер")
            .border_style(Style::default().fg(Color::Blue)),
    );
    frame.render_widget(header, layout[0]);

    let progress = Paragraph::new(Line::from(vec![
        Span::styled("Прогресс: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(format!("{}/{}", state.mastered, state.total)),
        Span::raw("    "),
        Span::styled(
            "Осталось в очереди: ",
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw(state.remaining().to_string()),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    frame.render_widget(progress, layout[1]);

    let question = state
        .current()
        .map(|card| card.question.as_str())
        .unwrap_or("Все вопросы пройдены.");
    let question_widget = Paragraph::new(question).wrap(Wrap { trim: true }).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Вопрос")
            .border_style(Style::default().fg(Color::Magenta)),
    );
    frame.render_widget(question_widget, layout[2]);

    let answer_text = if state.answer_shown {
        state
            .current()
            .map(|card| card.answer.as_str())
            .unwrap_or("")
    } else {
        "Нажмите 1, чтобы показать ответ."
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
                .title("Ответ")
                .border_style(Style::default().fg(Color::Green)),
        );
    frame.render_widget(answer_widget, layout[3]);

    let footer_message = if state.finished {
        summary_line(state)
    } else if state.answer_shown {
        "Нажмите 2, если теперь знаете ответ, 3 - если все еще не знаете, q - для выхода."
            .to_string()
    } else {
        "Нажмите 1, чтобы показать ответ, 2 - если знаете, 3 - если не знаете, q - для выхода."
            .to_string()
    };
    let footer = Paragraph::new(footer_message)
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Blue)),
        );
    frame.render_widget(footer, layout[4]);

    if state.finished {
        let popup_area = centered_rect(60, 20, area);
        frame.render_widget(Clear, popup_area);
        let popup = Paragraph::new(summary_line(state))
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(if state.aborted {
                        "Остановлено"
                    } else {
                        "Завершено"
                    })
                    .border_style(Style::default().fg(if state.aborted {
                        Color::Yellow
                    } else {
                        Color::Green
                    })),
            );
        frame.render_widget(popup, popup_area);
    }
}

fn centered_rect(
    percent_x: u16,
    percent_y: u16,
    area: ratatui::layout::Rect,
) -> ratatui::layout::Rect {
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

pub fn restore_terminal(mut terminal: Terminal<CrosstermBackend<Stdout>>) -> anyhow::Result<()> {
    disable_raw_mode().context("не удалось отключить raw mode")?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)
        .context("не удалось выйти из альтернативного экрана")?;
    terminal
        .show_cursor()
        .context("не удалось показать курсор")?;
    Ok(())
}

pub fn summary_line(state: &SessionState) -> String {
    if state.aborted {
        format!("Остановлено. Прогресс: {}/{}", state.mastered, state.total)
    } else {
        format!(
            "Готово. Пройдено вопросов: {}/{}",
            state.mastered, state.total
        )
    }
}
