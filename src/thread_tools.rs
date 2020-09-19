use crate::channel::Channel;
use crate::command::{Command, UiCommand::*};
use crate::interface::Interface;
use std::io::Result;

pub fn parse_incoming_cmd(cmd: Command, itf: &Interface) -> Result<Option<String>> {
    match cmd {
        Command::Privmsg(ref sender, ref target, _) => {
            let log_target = match target {
                t if t == &itf.get_username() => sender,
                _ => target,
            };

            if let Some(pos) = itf.get_channel_pos(&log_target) {
                let output = cmd.to_printable().unwrap();
                itf.write_to_chan(pos, &output)?;
                if itf.is_active(&log_target) {
                    return Ok(Some(output));
                }
            } else {
                let server = itf.get_server();
                let output = cmd.to_printable().unwrap();
                itf.push_channel(Channel::new(&log_target, &server));
                itf.write_to_chan(itf.channels_len() - 1, &output)?;
                itf.set_refresh_buffers_flag();
            }
        }

        _ => {
            if let Some(output) = cmd.to_printable() {
                itf.write_to_chan(0, &output)?;
                if itf.get_active_channel_pos() == 0 {
                    return Ok(Some(output));
                }
            }
        }
    }
    Ok(None)
}

pub fn parse_user_input(inp: &str, itf: &Interface) -> Command {
    match &inp[1..2] {
        "q" => {
            itf.set_shutdown_flag();
            Command::Quit(String::from("Quitting ..."))
        }

        "j" => {
            // TODO: check whether join is successful
            let mut channels = Vec::with_capacity(inp.len());
            for channel in inp.split_whitespace() {
                itf.push_channel(Channel::new(&channel, &itf.get_server()));
                channels.push(String::from(channel));
            }
            itf.store_active_channel(itf.channels_len() - 1);
            itf.set_refresh_buffers_flag();
            Command::Join(channels)
        }

        "p" => {
            let mut channels = Vec::with_capacity(inp.len());
            for channel in inp.split_whitespace() {
                if let Some(index) = itf.get_channel_pos(&channel) {
                    itf.remove_channel(index);
                    channels.push(String::from(channel));
                    if itf.is_active(&channel) {
                        itf.store_active_channel(itf.channels_len() - 1);
                    }
                }
            }
            itf.set_refresh_buffers_flag();
            Command::Part(channels)
        }

        "c" => {
            if let Ok(target) = &inp[2..].trim().parse::<usize>() {
                if itf.get_channel(*target).is_some() {
                    itf.store_active_channel(*target);
                    itf.set_refresh_buffers_flag();
                }
            }
            Command::Internal(PrintBuffers)
        }

        &_ => Command::Unknown,
    }
}
