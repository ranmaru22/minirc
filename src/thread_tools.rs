use crate::channel::Channel;
use crate::command::Command;
use crate::interface::Interface;
use std::io::Result;
use std::sync::mpsc::Sender;

pub fn parse_incoming_cmd(cmd: Command<'_>, itf: &Interface, pipe: &Sender<String>) -> Result<()> {
    match cmd {
        Command::Privmsg(sender, target, _) => {
            let printable = cmd.to_printable().unwrap();

            let log_target = match target {
                t if t == itf.get_username() => sender,
                _ => target,
            };

            if let Some(pos) = itf.get_channel_pos(&log_target) {
                itf.write_to_chan(pos, &printable)?;
                if itf.is_active(&log_target) {
                    pipe.send(printable).expect("Could not send to stdout");
                }
            } else {
                let server = itf.get_server();
                let mut c = Channel::new(log_target, &server);
                c.write(&printable)?;
                itf.push_channel(c);
                itf.toggle_refresh_buffers_flag();
            }
        }

        _ => {
            if let Some(printable) = cmd.to_printable() {
                itf.write_to_chan(0, &printable)?;
                if itf.get_active_channel_pos() == 0 {
                    pipe.send(printable).expect("Could not send to stdout");
                }
            }
        }
    }
    Ok(())
}

pub fn parse_user_cmd<'inp>(
    inp: &'inp str,
    itf: &'_ Interface,
    pipe: &'_ Sender<String>,
    argv: &'inp [&'inp str],
) -> Command<'inp> {
    match &inp[1..2] {
        "q" => {
            let quitmsg = if argv.is_empty() {
                "Quitting ..."
            } else {
                &inp[2..]
            };
            itf.set_shutdown_flag();
            Command::Quit(quitmsg)
        }

        "j" => {
            // TODO: check whether join is successful
            for channel in argv {
                itf.push_channel(Channel::new(channel, &itf.get_server()));
            }
            itf.store_active_channel(itf.channels_len() - 1);
            itf.toggle_refresh_buffers_flag();
            Command::Join(&argv)
        }

        "p" => {
            for channel in argv {
                if let Some(index) = itf.get_channel_pos(*channel) {
                    itf.remove_channel(index);
                    if itf.is_active(*channel) {
                        itf.store_active_channel(itf.channels_len() - 1);
                    }
                    itf.toggle_refresh_buffers_flag();
                }
            }
            Command::Part(&argv)
        }

        "c" => {
            if let Ok(target) = &inp[2..].trim().parse::<usize>() {
                if itf.get_channel(*target).is_some() {
                    itf.store_active_channel(*target);
                    itf.toggle_refresh_buffers_flag();
                }
            } else {
                let mut printable = String::from("Buffers: ");
                for i in 0..itf.channels_len() {
                    let name = itf.get_channel(i).unwrap();
                    printable.push_str(&format!("[{}]{} ", i, name));
                }
                pipe.send(printable).expect("Could not send to stdout");
            }
            Command::Unknown
        }

        &_ => Command::Unknown,
    }
}
