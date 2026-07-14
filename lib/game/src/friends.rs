#[derive(Debug, Clone, Default)]
pub struct FriendList {
    pub friends: Vec<Friend>,
}

#[derive(Debug, Clone)]
pub struct Friend {
    pub aid: u32,
    pub gid: u32,
    pub name: String,
    pub online: bool,
}

impl FriendList {
    pub fn index_of(&self, aid: u32, gid: u32) -> Option<usize> {
        self.friends.iter().position(|f| f.aid == aid && f.gid == gid)
    }

    pub fn set_all(&mut self, friends: Vec<Friend>) {
        self.friends = friends;
    }

    pub fn upsert(&mut self, friend: Friend) {
        match self.index_of(friend.aid, friend.gid) {
            Some(i) => self.friends[i] = friend,
            None => self.friends.push(friend),
        }
    }

    pub fn set_state(&mut self, aid: u32, gid: u32, online: bool) {
        if let Some(i) = self.index_of(aid, gid) {
            self.friends[i].online = online;
        }
    }

    pub fn remove(&mut self, aid: u32, gid: u32) {
        self.friends.retain(|f| !(f.aid == aid && f.gid == gid));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn friend(aid: u32, gid: u32, name: &str, online: bool) -> Friend {
        Friend {
            aid,
            gid,
            name: name.to_string(),
            online,
        }
    }

    #[test]
    fn friend_list_add_update_state_remove() {
        let mut list = FriendList::default();
        list.set_all(vec![friend(1, 10, "Alice", true), friend(2, 20, "Bob", false)]);
        assert_eq!(list.friends.len(), 2);

        list.upsert(friend(3, 30, "Carol", true));
        assert_eq!(list.friends.len(), 3);

        list.set_state(2, 20, true);
        assert!(list.friends[1].online);

        list.remove(1, 10);
        assert_eq!(list.friends.len(), 2);
        assert!(list.index_of(1, 10).is_none());
    }
}
