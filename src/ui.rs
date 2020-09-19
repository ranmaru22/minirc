use crate::interface::Interface;
use pancurses::*;

#[macro_export]
macro_rules! refresh_all {
    [$($w:expr),+] => {
        $( $w.refresh(); )+;
    };
}

pub fn init_curses(debug: bool) -> Window {
    let term = initscr();
    curs_set(0);

    if debug {
        cbreak();
    }
    noecho();

    term.timeout(5);
    term.clear();
    term.refresh();
    term.keypad(true);

    if has_colors() {
        use_default_colors();
        start_color();
        define_colour_pairs();
    }

    term
}

fn define_colour_pairs() {
    init_pair(0, -1, -1); // defaults, no colours
    init_pair(1, COLOR_RED, -1); // red on black
    init_pair(2, COLOR_BLUE, -1); // blue on black
    init_pair(3, COLOR_GREEN, -1); // green on black
}

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

pub fn handle_input(inp: &mut String, w: &Window, term: &Window) -> bool {
    let (y, x) = w.get_cur_yx();
    match term.getch() {
        Some(Input::KeyResize) => {
            resize_term(0, 0);
        }
        Some(Input::KeyBackspace) => {
            w.mv(y, x - 1);
            inp.pop();
            w.delch();
        }
        Some(Input::KeyLeft) => {
            w.chgat(1, A_NORMAL, 0);
            w.mv(y, x - 1);
            w.chgat(1, A_REVERSE, 0);
        }
        Some(Input::KeyUp) => {
            w.chgat(1, A_NORMAL, 0);
            w.mv(y, 0);
            w.chgat(1, A_REVERSE, 0);
        }
        Some(Input::KeyRight) => {
            if x < inp.len() as i32 {
                w.chgat(1, A_NORMAL, 0);
                w.mv(y, x + 1);
                w.chgat(1, A_REVERSE, 0);
            }
        }
        Some(Input::KeyDown) => {
            w.chgat(1, A_NORMAL, 0);
            w.mv(y, inp.len() as i32);
            w.chgat(1, A_REVERSE, 0);
        }
        Some(Input::Character(c)) if c == '\n' => {
            return true;
        }
        Some(Input::Character(c)) => {
            inp.push(c);
            w.chgat(1, A_NORMAL, 0);
            w.insch(c);
            w.mv(y, x + 1);
            w.chgat(1, A_REVERSE, 0);
        }
        Some(_) | None => (),
    }

    false
}

pub fn shift_lines_up(w: &Window, last_line: i32) {
    if w.get_cur_y() >= last_line {
        w.mv(0, 0);
        w.deleteln();
        w.mv(last_line, 0);
    }
}

pub fn refresh_buffers(w: &Window, interface: &Interface) {
    if interface.should_refresh_buffers() || w.is_touched() {
        w.mv(0, 0);
        w.deleteln();
        for i in 0..interface.channels_len() {
            let name = interface.get_channel(i).unwrap();
            if i == interface.get_active_channel_pos() {
                w.attron(A_BOLD);
                w.color_set(1);
            }
            w.addstr(&format!("[{}]{} ", i, name));
            w.attroff(A_BOLD);
            w.color_set(0);
        }
        w.refresh();
        interface.toggle_refresh_buffers_flag();
    }
}
