use std::io::Result;
use termion::event::Key;
use termion::input::TermRead;
use termion::AsyncReader;

pub fn split_line(s: &str, max_len: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let words = s.split_whitespace();
    let mut line = String::with_capacity(max_len);

    for word in words {
        if word.len() + line.len() < max_len {
            line.push_str(word);
            line.push(' ');
        } else {
            line.insert(line.len() - 1, '\n');
            lines.push(line);
            line = String::with_capacity(max_len);
            line.push_str(word);
            line.push(' ');
        }
    }

    if !line.is_empty() {
        line.insert(line.len() - 1, '\n');
        lines.push(line);
    }

    lines
}

pub fn handle_input(inp: &mut String, stdin: &mut AsyncReader) -> Result<()> {
    let mut keys = stdin.keys();
    if let Some(Ok(key)) = keys.next() {
        match key {
            Key::Char(c) => inp.push(c),
            Key::Backspace => {
                inp.pop();
            }
            _ => (),
        }
    }
    Ok(())
}
