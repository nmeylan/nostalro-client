use crate::App;
use ragnarok_game::event::FriendData;
use ragnarok_game::friends::Friend;

impl App {
    pub(super) fn handle_friend_list_received(&mut self, friends: Vec<FriendData>) {
        self.game.friends.set_all(
            friends
                .into_iter()
                .map(|f| Friend {
                    aid: f.aid,
                    gid: f.gid,
                    name: f.name,
                    online: f.online,
                })
                .collect(),
        );
    }

    pub(super) fn handle_friend_state_changed(&mut self, aid: u32, gid: u32, online: bool) {
        self.game.friends.set_state(aid, gid, online);
    }

    pub(super) fn handle_friend_add_result(&mut self, result: u8, aid: u32, gid: u32, name: String) {
        let text = match result {
            0 => {
                self.game.friends.upsert(Friend {
                    aid,
                    gid,
                    name: name.clone(),
                    online: true,
                });
                format!("You are now friends with {name}.")
            }
            1 => format!("{name} does not want to be friends with you."),
            2 => "Your friend list is full.".to_string(),
            3 => format!("{name}'s friend list is full."),
            _ => format!("Failed to add {name} as a friend."),
        };
        self.game.chat_window.add_system(text);
    }

    pub(super) fn handle_friend_removed(&mut self, aid: u32, gid: u32) {
        self.game.friends.remove(aid, gid);
    }

    pub(super) fn handle_friend_request_received(&mut self, req_aid: u32, req_gid: u32, name: String) {
        self.game.pending_confirms.pending_friend_request = Some((req_aid, req_gid));
        self.game.pending_confirms.friend_request_result.set(None);
        let msg = format!("{name} wishes to be friends with you. Accept?");
        self.game.confirm_dialog.show_with_out(
            &msg,
            true,
            self.game.pending_confirms.friend_request_result.clone(),
            |_| {},
        );
    }
}
