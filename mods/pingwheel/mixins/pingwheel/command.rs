// `/pingwheel` (`ClientCommandBuilder`): help, config and channel.
fn pingwheel_command(game: &mut crate::game::Game<'_>, command: &str, ran: &mut bool) {
    use crate::game::pingwheel as m;
    use horizon_client::mod_api::ServerboundPlayPacket as S;
    use horizon_protocol::packets::configuration::CustomPayload;
    if *ran || !game.native_on(m::ID) {
        return;
    }
    let Some(cmd) = m::parse_command(command) else {
        return;
    };
    *ran = true;
    match cmd {
        m::Command::Help => {
            for line in m::HELP {
                game.native_chat(line);
            }
        }
        m::Command::Config => game.native_chat("Ping Wheel Settings: Options > Mods > Ping Wheel > Settings"),
        m::Command::Channel(None) => {
            let ch = m::with(|s| s.channel.clone());
            game.native_chat(&if ch.is_empty() {
                "Currently using default channel".to_owned()
            } else {
                format!("Current channel: \"{ch}\"")
            });
        }
        m::Command::Channel(Some(name)) => {
            if name.chars().count() > m::MAX_CHANNEL_LENGTH {
                game.native_chat(&format!("Channel names are limited to a length of {} characters", m::MAX_CHANNEL_LENGTH));
                return;
            }
            let store = m::with(|s| {
                s.channel = name.clone();
                s.save();
                std::mem::take(&mut s.store)
            });
            for (k, v) in store {
                game.native_set_setting(m::ID, &k, &v);
            }
            let _ = game.native_send_packet(S::CustomPayload(CustomPayload {
                channel: m::C2S_CHANNEL.into(),
                data: m::encode_channel(&name),
            }));
            game.native_chat(&if name.is_empty() {
                "Set channel to default".to_owned()
            } else {
                format!("Set channel to \"{name}\"")
            });
        }
    }
}
