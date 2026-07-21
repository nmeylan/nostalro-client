use crate::emotion::emote_type_for_command;

/// A parsed `/command`. String parsing lives here (in `lib/game`, so it is
/// testable and reusable from tools); the network/UI side-effects stay in the
/// client, which matches on this value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatCommand {
    Sit,
    Stand,
    Doridori,
    BingBing,
    BangBang,
    Where,
    Memo,
    ExitRoom,
    LeaveParty,
    MakeParty(String),
    InviteParty(String),
    MakeGuild(String),
    BreakGuild(String),
    StatUp { status_id: u16, amount: u32 },
    ToggleEffect,
    ToggleFog,
    ToggleAura,
    ToggleNoTrade,
    ToggleNoShift,
    ToggleNoCtrl,
    ToggleBgm,
    ToggleSound,
    /// `/v <0-127>` — sound-effect volume.
    SetSfxVolume(u8),
    /// `/bv <0-127>` — background-music volume.
    SetBgmVolume(u8),
    /// `/showexp` — toggle "Gained N experience" chat messages.
    ToggleShowExp,
    /// `/notalkmsg` — toggle hiding public chat in the chat window.
    ToggleHidePublicChat,
    /// `/battlemode` — keyboard skill-bar mode.
    BattleMode,
    /// `/alchemist`, `/blacksmith`, `/taekwon` — request a top-10 ranking.
    Ranking(RankKind),
    /// `/miss` — toggle the "Miss" damage text.
    ToggleMiss,
    /// `/eqopen` — toggle whether other players can view your equipment.
    ToggleEqOpen,
    RefuseParty(bool),
    /// `/gc <msg>` or a `$`-prefixed line — send guild chat.
    GuildChat(String),
    /// `/hi <text>` — whisper every online friend.
    WhisperFriends(String),
    /// `/ex <name>` (block) / `/in <name>` (unblock).
    WhisperBlock { name: String, block: bool },
    /// `/exall` (block) / `/inall` (unblock).
    WhisperBlockAll(bool),
    /// `/ex` with no argument — list currently blocked players.
    WhisperListBlocked,
    /// `/chat` — open the chat-room creation window.
    OpenChatCreate,
    /// `/emotion` — open the emotion-list window.
    OpenEmotionList,
    /// `/hoai` / `/mercai` — open the companion AI settings at the homunculus
    /// (`false`) or mercenary (`true`) tab.
    OpenCompanionAi { mercenary: bool },
    Emote(u8),
    /// `/show_ping` — toggle the network sync/latency overlay.
    ToggleShowPing,
    /// `/show_fps` — toggle the frame-rate overlay.
    ToggleShowFps,
    /// List the supported commands ([`COMMAND_HELP`]) in the chat window.
    Help,
    /// A real client command we don't implement yet (e.g. `/camera`, `/set1`).
    Unsupported,
    /// Recognised but removed from the classic client (`/who`, `/showname`, …).
    Outdated,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RankKind {
    Alchemist,
    Blacksmith,
    Taekwon,
}

/// Commands the original client recognises but which have no backing in this
/// client yet. Kept distinct from [`ChatCommand::Unknown`] (typos) so the user
/// gets an honest "not supported" reply instead of "unknown command".
const UNSUPPORTED: &[&str] = &[
    "/set1",
    "/set2",
    "/set3",
    "/camera",
    "/font",
    "/q1",
    "/q2",
    "/q3",
    "/quickspell",
    "/quickspell2",
    "/quickspell3",
    "/snap",
    "/skillsnap",
    "/itemsnap",
    "/skillfail",
    "/sf",
    "/skip",
    "/stat",
    "/emblem",
    "/tip",
    "/objlight",
    "/lightmap",
    "/clipmouse",
    "/traceai",
    "/hw",
    "/am",
    "/notalkmsg2",
    "/nm2",
    "/window",
    "/wi",
    "/loginout",
    "/li",
    "/shopping",
    "/sh",
    "/pk",
];

/// `(command, description)` shown by `/h`. Descriptions follow the classic
/// control reference; only commands the client actually supports are listed.
pub const COMMAND_HELP: &[(&str, &str)] = &[
    ("/sit", "Makes the character sit."),
    ("/stand", "Makes the character stand."),
    ("/doridori", "Moves the character's head from side to side."),
    ("/bangbang", "Turns the character clockwise."),
    ("/bingbing", "Turns the character counter clockwise."),
    ("/where", "Shows the current map and coordinates."),
    ("/memo", "Memorizes a location for the Warp Portal skill."),
    ("/q", "Leaves a chatroom."),
    ("/leave", "Leaves your party."),
    ("/organize <name>", "Creates a party."),
    ("/invite <name>", "Invites a player to your party."),
    ("/guild <name>", "Creates a guild (requires an Emperium)."),
    ("/breakguild <name>", "Disbands your guild."),
    (
        "/str+ <n>",
        "Raises STR by n (also /agi+ /vit+ /int+ /dex+ /luk+).",
    ),
    ("/effect", "Turns skill effects on and off."),
    ("/fog", "Turns fog on and off."),
    ("/aura", "Minimizes level 99 aura effects."),
    ("/bgm", "Turns background music on and off."),
    ("/sound", "Turns sound effects on and off."),
    ("/bv <0-127>", "Sets background-music volume."),
    ("/v <0-127>", "Sets sound-effect volume."),
    ("/showexp", "Toggles experience-gain messages."),
    ("/notalkmsg or /nm", "Hides public chat in the chat window."),
    ("/battlemode or /bm", "Keyboard skill-bar mode."),
    ("/miss", "Toggles the Miss damage text."),
    ("/eqopen", "Toggles letting others view your equipment."),
    ("/gc <msg>", "Sends a guild-chat message ($ prefix works too)."),
    ("/hi <text>", "Whispers every online friend."),
    ("/ex <name>", "Blocks whispers from a player (/ex lists them)."),
    ("/in <name>", "Unblocks whispers from a player."),
    ("/exall or /inall", "Blocks or unblocks all whispers."),
    ("/chat", "Opens the chat-room creation window."),
    ("/emotion", "Opens the emotion list."),
    ("/hoai", "Opens the homunculus AI settings."),
    ("/mercai", "Opens the mercenary AI settings."),
    (
        "/alchemist /blacksmith /taekwon",
        "Shows the top-10 ranking.",
    ),
    ("/notrade or /nt", "Blocks all trade offers."),
    ("/refuse", "Auto-declines party invites (/accept re-enables)."),
    ("/noctrl or /nc", "Attack monsters with a single left-click."),
    ("/noshift or /ns", "Use support skills without holding Shift."),
    ("/show_ping", "Toggles the network sync/latency overlay."),
    ("/show_fps", "Toggles the frame-rate overlay."),
    ("/h or /help", "Lists the in-game commands."),
];

const STAT_ID_STR: u16 = 13;
const STAT_ID_AGI: u16 = 14;
const STAT_ID_VIT: u16 = 15;
const STAT_ID_INT: u16 = 16;
const STAT_ID_DEX: u16 = 17;
const STAT_ID_LUK: u16 = 18;

fn parse_volume(args: &str) -> Option<u8> {
    args.parse::<u8>().ok().map(|v| v.min(127))
}

fn stat_id(cmd: &str) -> Option<u16> {
    match cmd {
        "/str+" => Some(STAT_ID_STR),
        "/agi+" => Some(STAT_ID_AGI),
        "/vit+" => Some(STAT_ID_VIT),
        "/int+" => Some(STAT_ID_INT),
        "/dex+" => Some(STAT_ID_DEX),
        "/luk+" => Some(STAT_ID_LUK),
        _ => None,
    }
}

/// Parse a slash command line (leading `/` required) into a [`ChatCommand`].
pub fn parse_chat_command(input: &str) -> ChatCommand {
    let input = input.trim();
    let raw_cmd = input.split_whitespace().next().unwrap_or("");
    let args = input[raw_cmd.len()..].trim();
    let cmd = raw_cmd.to_ascii_lowercase();

    if let Some(status_id) = stat_id(&cmd) {
        return match args.parse::<u32>() {
            Ok(amount) if amount > 0 => ChatCommand::StatUp { status_id, amount },
            _ => ChatCommand::Unknown,
        };
    }

    match cmd.as_str() {
        "/v" => return parse_volume(args).map_or(ChatCommand::Unknown, ChatCommand::SetSfxVolume),
        "/bv" => return parse_volume(args).map_or(ChatCommand::Unknown, ChatCommand::SetBgmVolume),
        _ => {}
    }

    match cmd.as_str() {
        "/sit" => ChatCommand::Sit,
        "/stand" => ChatCommand::Stand,
        "/doridori" => ChatCommand::Doridori,
        "/bingbing" => ChatCommand::BingBing,
        "/bangbang" => ChatCommand::BangBang,
        "/where" => ChatCommand::Where,
        "/memo" => ChatCommand::Memo,
        "/q" => ChatCommand::ExitRoom,
        "/leave" => ChatCommand::LeaveParty,
        "/organize" => ChatCommand::MakeParty(args.to_string()),
        "/invite" => ChatCommand::InviteParty(args.to_string()),
        "/guild" => ChatCommand::MakeGuild(args.to_string()),
        "/breakguild" => ChatCommand::BreakGuild(args.to_string()),
        "/fog" => ChatCommand::ToggleFog,
        "/aura" => ChatCommand::ToggleAura,
        "/notrade" | "/nt" => ChatCommand::ToggleNoTrade,
        "/noshift" | "/ns" => ChatCommand::ToggleNoShift,
        "/noctrl" | "/nc" => ChatCommand::ToggleNoCtrl,
        "/bgm" => ChatCommand::ToggleBgm,
        "/sound" => ChatCommand::ToggleSound,
        "/showexp" => ChatCommand::ToggleShowExp,
        "/notalkmsg" | "/nm" => ChatCommand::ToggleHidePublicChat,
        "/battlemode" | "/bm" => ChatCommand::BattleMode,
        "/refuse" => ChatCommand::RefuseParty(true),
        "/accept" => ChatCommand::RefuseParty(false),
        "/miss" => ChatCommand::ToggleMiss,
        "/eqopen" => ChatCommand::ToggleEqOpen,
        "/effect" | "/mineffect" | "/minimize" => ChatCommand::ToggleEffect,
        "/gc" | "/guildchat" => ChatCommand::GuildChat(args.to_string()),
        "/hi" => ChatCommand::WhisperFriends(args.to_string()),
        "/ex" => {
            if args.is_empty() {
                ChatCommand::WhisperListBlocked
            } else {
                ChatCommand::WhisperBlock {
                    name: args.to_string(),
                    block: true,
                }
            }
        }
        "/in" => ChatCommand::WhisperBlock {
            name: args.to_string(),
            block: false,
        },
        "/exall" => ChatCommand::WhisperBlockAll(true),
        "/inall" => ChatCommand::WhisperBlockAll(false),
        "/chat" => ChatCommand::OpenChatCreate,
        "/emotion" => ChatCommand::OpenEmotionList,
        "/hoai" => ChatCommand::OpenCompanionAi { mercenary: false },
        "/mercai" | "/merai" => ChatCommand::OpenCompanionAi { mercenary: true },
        "/alchemist" => ChatCommand::Ranking(RankKind::Alchemist),
        "/blacksmith" => ChatCommand::Ranking(RankKind::Blacksmith),
        "/taekwon" => ChatCommand::Ranking(RankKind::Taekwon),
        "/show_ping" => ChatCommand::ToggleShowPing,
        "/show_fps" => ChatCommand::ToggleShowFps,
        "/h" | "/help" => ChatCommand::Help,
        "/who" | "/w" | "/showname" | "/report" | "/loading" => ChatCommand::Outdated,
        _ => match emote_type_for_command(raw_cmd) {
            Some(emote_type) => ChatCommand::Emote(emote_type),
            None if UNSUPPORTED.contains(&cmd.as_str()) => ChatCommand::Unsupported,
            None => ChatCommand::Unknown,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_commands_args_and_emotes() {
        assert_eq!(parse_chat_command("/sit"), ChatCommand::Sit);
        assert_eq!(parse_chat_command("/nt"), ChatCommand::ToggleNoTrade);
        assert_eq!(
            parse_chat_command("/organize My Party"),
            ChatCommand::MakeParty("My Party".to_string())
        );
        assert_eq!(
            parse_chat_command("/str+ 5"),
            ChatCommand::StatUp {
                status_id: STAT_ID_STR,
                amount: 5
            }
        );
        assert_eq!(parse_chat_command("/lv"), ChatCommand::Emote(3));
        assert_eq!(parse_chat_command("/who"), ChatCommand::Outdated);
        assert_eq!(parse_chat_command("/nope"), ChatCommand::Unknown);
    }

    #[test]
    fn refuse_and_accept_flip_flag() {
        assert_eq!(parse_chat_command("/refuse"), ChatCommand::RefuseParty(true));
        assert_eq!(parse_chat_command("/accept"), ChatCommand::RefuseParty(false));
    }

    #[test]
    fn help_is_not_shadowed_by_emote() {
        assert_eq!(parse_chat_command("/h"), ChatCommand::Help);
        assert_eq!(parse_chat_command("/help"), ChatCommand::Help);
    }

    #[test]
    fn parses_volume_clamped_and_toggles() {
        assert_eq!(parse_chat_command("/bgm"), ChatCommand::ToggleBgm);
        assert_eq!(parse_chat_command("/v 100"), ChatCommand::SetSfxVolume(100));
        assert_eq!(parse_chat_command("/bv 200"), ChatCommand::SetBgmVolume(127));
        assert_eq!(parse_chat_command("/v"), ChatCommand::Unknown);
        assert_eq!(parse_chat_command("/showexp"), ChatCommand::ToggleShowExp);
        assert_eq!(parse_chat_command("/show_ping"), ChatCommand::ToggleShowPing);
        assert_eq!(parse_chat_command("/show_fps"), ChatCommand::ToggleShowFps);
    }

    #[test]
    fn parses_whisper_guild_and_unsupported() {
        assert_eq!(
            parse_chat_command("/ex Bob"),
            ChatCommand::WhisperBlock {
                name: "Bob".to_string(),
                block: true
            }
        );
        assert_eq!(parse_chat_command("/ex"), ChatCommand::WhisperListBlocked);
        assert_eq!(parse_chat_command("/inall"), ChatCommand::WhisperBlockAll(false));
        assert_eq!(
            parse_chat_command("/gc hello team"),
            ChatCommand::GuildChat("hello team".to_string())
        );
        assert_eq!(parse_chat_command("/mineffect"), ChatCommand::ToggleEffect);
        assert_eq!(parse_chat_command("/camera"), ChatCommand::Unsupported);
    }

    #[test]
    fn parses_rankings_and_pk() {
        assert_eq!(
            parse_chat_command("/alchemist"),
            ChatCommand::Ranking(RankKind::Alchemist)
        );
        assert_eq!(
            parse_chat_command("/taekwon"),
            ChatCommand::Ranking(RankKind::Taekwon)
        );
        assert_eq!(parse_chat_command("/pk"), ChatCommand::Unsupported);
    }

    #[test]
    fn parses_companion_ai_tabs() {
        assert_eq!(
            parse_chat_command("/hoai"),
            ChatCommand::OpenCompanionAi { mercenary: false }
        );
        assert_eq!(
            parse_chat_command("/mercai"),
            ChatCommand::OpenCompanionAi { mercenary: true }
        );
        assert_eq!(
            parse_chat_command("/merai"),
            ChatCommand::OpenCompanionAi { mercenary: true }
        );
    }
}
