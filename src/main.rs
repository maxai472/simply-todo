use std::{char, iter};

use color_eyre::{
    eyre::{Ok, Result},
    owo_colors::CssColors::{Green, LightBlue},
};
use crossterm::{event::KeyModifiers, terminal::window_size};
use ratatui::{
    DefaultTerminal, Frame, crossterm::event::{self, Event, KeyEvent}, layout::{Constraint, Layout}, macros::row, style::{Color, Style, Stylize}, text::{Span, ToSpan}, widgets::{
        Block, BorderType, List, ListItem, ListState, Padding, Paragraph, Widget,
        canvas::MapResolution,
    },
};
use serde::{Deserialize, Serialize};

use crate::{Row::Item, Windows::Input};

#[derive(Debug, Default)]
struct AppState {
    items: Vec<TodoItem>,
    list_state: ListState,
    active_window: Windows,
    input_value: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct TodoItem {
    is_done: bool,
    description: String,
    subtasks: Vec<TodoItem>,
    #[serde(skip)]
    expanded: bool,
}

#[derive(Debug, Default, PartialEq)]
enum Windows {
    #[default]
    List,
    Input,
}

enum FormAction {
    None,
    Submit,
    Escape,
    SubmitSubTask
}

enum Row {
    Item(usize),
    Sub(usize, usize)
}

fn main() -> Result<()> {
    color_eyre::install()?;
    let mut state = AppState::default();
    state.items = load_items()?;
    if !state.items.is_empty() {
        state.list_state.select(Some(0));
    }

    let terminal = ratatui::init();

    let res = run(terminal, &mut state);
    save_items(&state.items)?;
    ratatui::restore();
    res
}

fn build_rows(items: &[TodoItem]) -> Vec<Row> {
    let mut rows = Vec::new();
    for (i, item) in items.iter().enumerate() {
        rows.push(Row::Item(i));
        if item.expanded {
            for j in 0..item.subtasks.len() {
                rows.push(Row::Sub(j, i));
            }
        }
    }
    rows
}

fn selected_item_mut<'a>(items: &'a mut [TodoItem], rows: &[Row], selected: usize) -> Option<&'a mut TodoItem> {
   match rows.get(selected)? {
       Row::Item(i) => items.get_mut(*i),
       Row::Sub(i, j) => items.get_mut(*i)?.subtasks.get_mut(*j),
    } 
}

fn save_items(items: &[TodoItem]) -> Result<()> {
    let json = serde_json::to_string_pretty(items)?;
    std::fs::write("todos.json", json)?;
    Ok(())
}

fn load_items() -> Result<Vec<TodoItem>> {
    if !std::path::Path::new("todos.json").exists() {
        return Ok(Vec::new());
    }

    let json = std::fs::read_to_string("todos.json")?;
    let items = serde_json::from_str(&json)?;

    Ok(items)
}

fn run(mut terminal: DefaultTerminal, app_state: &mut AppState) -> Result<()> {
    let rows = build_rows(&app_state.items);
    loop {
        // Rendering
        terminal.draw(|f| render(f, app_state))?;
        // Input Handling
        if let Event::Key(key) = event::read()? {
            if app_state.active_window == Windows::Input {
                match handle_input(key, app_state) {
                    FormAction::None => {}
                    FormAction::Submit => {
                        app_state.items.push(TodoItem {
                            is_done: false,
                            description: app_state.input_value.clone(),
                            subtasks: Vec::new(),
                            expanded: false
                        });
                        app_state.input_value.clear();
                    }
                    FormAction::Escape => {
                        app_state.active_window = Windows::List;
                        app_state.input_value.clear();
                    }
                    FormAction::SubmitSubTask => {
                        if let Some(index) = app_state.list_state.selected() {
                            if let Some(item) = selected_item_mut(&mut app_state.items, &rows, index) {
                                item.subtasks.push(TodoItem{
                                    is_done: false,
                                    description: app_state.input_value.clone(),
                                    subtasks: Vec::new(),
                                    expanded: false,
                                });
                                app_state.input_value.clear();
                            }
                        }
                    },
                }
            } else {
                if handle_list(key, app_state) {
                    break;
                }
            }
        }
    }
    Ok(())
}

fn handle_list(key: KeyEvent, app_state: &mut AppState) -> bool {
    let rows = build_rows(&app_state.items);
    match key.code {
        event::KeyCode::Esc => {
            return true;
        }
        event::KeyCode::Tab => {
            app_state.active_window = Windows::Input;
        }
        event::KeyCode::Enter => {
            if let Some(index) = app_state.list_state.selected() {
                if let Some(item) = selected_item_mut(&mut app_state.items, &rows, index) {
                    item.is_done = !item.is_done;
                }
            }
        }
        event::KeyCode::Right => {
            if let Some(index) = app_state.list_state.selected() {
                if let Some(item) = selected_item_mut(&mut app_state.items, &rows, index) {
                    item.expanded = true;
                }
            }
        }
        event::KeyCode::Left => {
            if let Some(index) = app_state.list_state.selected() {
                if let Some(item) = selected_item_mut(&mut app_state.items, &rows, index)  {
                    item.expanded = false;
                }
            }
        }
        event::KeyCode::Char(char) => match char {
            'q' => {
                return true;
            }
            'j' => {
                app_state.list_state.select_next();
            }
            'k' => {
                app_state.list_state.select_previous();
            }
            'x' => {
                if let Some(index) = app_state.list_state.selected() {
                    if let Some(item) =  selected_item_mut(&mut app_state.items, &rows, index) {
                        item.is_done = !item.is_done;
                    }
                }
            }
            'd' => {
                if let Some(index) = app_state.list_state.selected() {
                    app_state.items.remove(index);
                }
            }
            'a' => {
                app_state.active_window = Input;
            }

            _ => {}
        },
        _ => {}
    }
    return false;
}

fn handle_input(key: KeyEvent, app_state: &mut AppState) -> FormAction {
    match (key.code, key.modifiers) {
        (event::KeyCode::Esc, _) => {
            return FormAction::Escape;
        }
        (event::KeyCode::Tab, _) => {
            app_state.active_window = Windows::List;
        }
        (event::KeyCode::Backspace, _) => {
            app_state.input_value.pop();
        }
        (event::KeyCode::Char('a'), event::KeyModifiers::CONTROL) => {
            return FormAction::SubmitSubTask;
        }
        (event::KeyCode::Enter, _) => {
            return FormAction::Submit;
        }
        (event::KeyCode::Char('n'), KeyModifiers::CONTROL) => {
            app_state.list_state.select_next();
        }
        (event::KeyCode::Char('p'), KeyModifiers::CONTROL) => {
            app_state.list_state.select_previous();
        }
        (event::KeyCode::Char(c), KeyModifiers::NONE) => {
            app_state.input_value.push(c);
        }
        _ => {}
    }
    return FormAction::None;
}

fn render(frame: &mut Frame, app_state: &mut AppState) {
    let input_colour = if app_state.active_window == Windows::Input {
        Color::Green
    } else {
        Color::LightBlue
    };
    let list_colour = if app_state.active_window == Windows::List {
        Color::Green
    } else {
        Color::LightBlue
    };

    let [title_area, list_area, input_area, help_text_area] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Fill(1),
        Constraint::Length(3),
        Constraint::Length(1),
    ])
    .margin(1)
    .areas(frame.area());

    Paragraph::new("To do List".to_span().bold().underlined())
        .render(title_area, frame.buffer_mut());

    let list_block = Block::bordered()
        .title(" List items ".to_span().into_centered_line())
        .border_type(BorderType::Rounded)
        .fg(list_colour);

    let rows = build_rows(&app_state.items);
    let list = List::new(rows.iter().map(|row| {
        match row {
            Row::Item(i) => {
                let item = &app_state.items[*i];
                let has_subtasks = if item.subtasks.is_empty() {""} else {"*"};
                let prefix = if item.is_done { "[x]  " } else { "[] " };
                let text = format!("{has_subtasks} {prefix}{}", item.description);
                ListItem::from(if item.is_done { Span::from(text).crossed_out() } else { Span::from(text) })
            },
            Row::Sub(i, j) => {
                let sub = &app_state.items[*i].subtasks[*j];
                let prefix = if sub.is_done { "[x]  " } else { "[] " };
                let text = format!("  └{prefix}{}", sub.description);
                ListItem::from(if sub.is_done { Span::from(text).crossed_out() } else { Span::from(text) })
            },
        }
    }))
    .block(list_block)
    .highlight_style(Style::default().fg(Color::Green))
    .highlight_symbol(">");

    frame.render_stateful_widget(list, list_area, &mut app_state.list_state);

    Paragraph::new(app_state.input_value.as_str())
        .block(
            Block::bordered()
                .title(" Input ".to_span().into_centered_line())
                .fg(input_colour)
                .padding(Padding::horizontal(1))
                .border_type(BorderType::Rounded),
        )
        .render(input_area, frame.buffer_mut());

    let help_text_value = match app_state.active_window {
        Windows::List => " j/k - navigate, a - new item, x - del item, <enter> - toggle_status ",
        Input => " <Enter> - submit, <Esc> - Cancel, <Tab> - Cycle Window ",
    };
    Paragraph::new(help_text_value.to_span().into_centered_line())
        .render(help_text_area, frame.buffer_mut());
}
