use pancurses::*;

pub fn shift_lines_up(w: &Window, last_line: i32) {
    if w.get_cur_y() >= last_line {
        w.mv(0, 0);
        w.deleteln();
        w.mv(last_line, 0);
    }
}
