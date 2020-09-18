use crate::interface::Interface;
use pancurses::*;

pub fn shift_lines_up(w: &Window, last_line: i32) {
    if w.get_cur_y() >= last_line {
        w.mv(0, 0);
        w.deleteln();
        w.mv(last_line, 0);
    }
}

pub fn refresh_buffers(w: &Window, interface: &Interface) {
    if interface.should_refresh_buffers() {
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
