use std::{env, fs, io::{self, Write}};
use crossterm::{
    cursor as ct_cursor,
    event::{self, Event, KeyCode, KeyModifiers},
    execute, queue,
    style::Print,
    terminal::{self, ClearType},
};
use vix_editor::{Buffer, Cursor, Direction, Granularity, Mode};

fn main() -> io::Result<()> {
    // Load file from argv or start empty
    let mut buf = match env::args().nth(1) {
        Some(path) => {
            let text = fs::read_to_string(&path).unwrap_or_default();
            Buffer::from_str(&text)
        }
        None => Buffer::new(),
    };

    let mut cursor = Cursor::new();
    let mut mode   = Mode::Normal;

    let mut stdout = io::stdout();
    terminal::enable_raw_mode()?;
    execute!(stdout, terminal::EnterAlternateScreen, ct_cursor::Hide)?;

    loop {
        let (cols, rows) = terminal::size()?;
        let rows = rows as usize;

        // ── render ────────────────────────────────────────────────────────
        let cur_line = buf.byte_to_line(cursor.byte);
        let scroll   = cur_line.saturating_sub(rows.saturating_sub(2));

        queue!(stdout, terminal::Clear(ClearType::All), ct_cursor::MoveTo(0, 0))?;

        for screen_row in 0..rows.saturating_sub(1) {
            let buf_line = screen_row + scroll;
            if buf_line >= buf.len_lines() {
                queue!(stdout, Print("~\r\n"))?;
            } else {
                let line = buf.line(buf_line);
                let s: String = line.chars()
                    .take(cols as usize)
                    .collect::<String>()
                    .replace('\n', "")
                    .replace('\r', "");
                queue!(stdout, Print(&s), Print("\r\n"))?;
            }
        }

        // status bar
        let file_name = env::args().nth(1).unwrap_or_else(|| "[no file]".into());
        let dirty     = if buf.dirty { " [+]" } else { "" };
        let mode_str  = match mode { Mode::Normal => "NOR", Mode::Insert => "INS" };
        let status = format!(
            " {} | {} {} | {}:{} ",
            mode_str,
            file_name,
            dirty,
            cur_line + 1,
            cursor.col(&buf) + 1,
        );
        let padded = format!("{:<width$}", status, width = cols as usize);
        queue!(stdout, Print(&padded))?;

        // position the terminal cursor
        let screen_row = (cur_line - scroll) as u16;
        let screen_col = cursor.col(&buf) as u16;
        queue!(stdout, ct_cursor::MoveTo(screen_col, screen_row), ct_cursor::Show)?;

        stdout.flush()?;

        // ── input ─────────────────────────────────────────────────────────
        let Event::Key(key) = event::read()? else { continue };

        match mode {
            Mode::Normal => match key.code {
                // quit
                KeyCode::Char('q') => break,

                // save
                KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    if let Some(path) = env::args().nth(1) {
                        let text: String = buf.rope.to_string();
                        fs::write(&path, text)?;
                        buf.dirty = false;
                    }
                }

                // mode switch
                KeyCode::Char('i') => mode = Mode::Insert,
                KeyCode::Char('a') => {
                    cursor.move_by(Direction::Right, Granularity::Char, &buf);
                    mode = Mode::Insert;
                }

                // movement
                KeyCode::Char('h') | KeyCode::Left  =>
                    cursor.move_by(Direction::Left,  Granularity::Char, &buf),
                KeyCode::Char('l') | KeyCode::Right =>
                    cursor.move_by(Direction::Right, Granularity::Char, &buf),
                KeyCode::Char('k') | KeyCode::Up    =>
                    cursor.move_by(Direction::Up,    Granularity::Char, &buf),
                KeyCode::Char('j') | KeyCode::Down  =>
                    cursor.move_by(Direction::Down,  Granularity::Char, &buf),
                KeyCode::Char('w') =>
                    cursor.move_by(Direction::Right, Granularity::Word, &buf),
                KeyCode::Char('b') =>
                    cursor.move_by(Direction::Left,  Granularity::Word, &buf),
                KeyCode::Char('0') =>
                    cursor.move_by(Direction::Left,  Granularity::Line, &buf),
                KeyCode::Char('$') =>
                    cursor.move_by(Direction::Right, Granularity::Line, &buf),

                // undo / redo
                KeyCode::Char('u') => { buf.undo(); cursor.clamp(&buf); }
                KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    buf.redo(); cursor.clamp(&buf);
                }

                // delete char under cursor (x)
                KeyCode::Char('x') => {
                    if cursor.byte < buf.len_bytes() {
                        buf.delete(cursor.byte, 1);
                        cursor.clamp(&buf);
                    }
                }

                // open line below (o)
                KeyCode::Char('o') => {
                    cursor.move_by(Direction::Right, Granularity::Line, &buf);
                    buf.insert(cursor.byte, "\n");
                    cursor.move_by(Direction::Right, Granularity::Char, &buf);
                    mode = Mode::Insert;
                }

                _ => {}
            },

            Mode::Insert => match key.code {
                KeyCode::Esc => mode = Mode::Normal,

                KeyCode::Char(c) => {
                    let mut tmp = [0u8; 4];
                    let s = c.encode_utf8(&mut tmp);
                    buf.insert(cursor.byte, s);
                    cursor.move_by(Direction::Right, Granularity::Char, &buf);
                }

                KeyCode::Enter => {
                    buf.insert(cursor.byte, "\n");
                    cursor.move_by(Direction::Right, Granularity::Char, &buf);
                }

                KeyCode::Backspace => {
                    if cursor.byte > 0 {
                        cursor.move_by(Direction::Left, Granularity::Char, &buf);
                        buf.delete(cursor.byte, 1);
                    }
                }

                KeyCode::Left  => cursor.move_by(Direction::Left,  Granularity::Char, &buf),
                KeyCode::Right => cursor.move_by(Direction::Right, Granularity::Char, &buf),
                KeyCode::Up    => cursor.move_by(Direction::Up,    Granularity::Char, &buf),
                KeyCode::Down  => cursor.move_by(Direction::Down,  Granularity::Char, &buf),

                _ => {}
            },
        }
    }

    execute!(stdout, terminal::LeaveAlternateScreen, ct_cursor::Show)?;
    terminal::disable_raw_mode()?;
    Ok(())
}
