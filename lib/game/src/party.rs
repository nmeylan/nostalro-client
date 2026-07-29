#[derive(Debug, Clone, Default)]
pub struct Party {
    pub name: String,
    pub members: Vec<PartyMember>,
    pub exp_share: bool,
    pub item_pickup_rule: u8,
    pub item_division_rule: u8,
}

#[derive(Debug, Clone)]
pub struct PartyMember {
    pub aid: u32,
    pub name: String,
    pub map: String,
    pub leader: bool,
    pub online: bool,
    pub hp: Option<u32>,
    pub max_hp: Option<u32>,
    pub x: u16,
    pub y: u16,
    /// The server sends coordinates only while the member shares our map, and
    /// -1,-1 when they leave it.
    pub has_live_position: bool,
}

impl Party {
    pub fn new(name: String) -> Self {
        Self {
            name,
            ..Default::default()
        }
    }

    pub fn member_mut(&mut self, aid: u32) -> Option<&mut PartyMember> {
        self.members.iter_mut().find(|m| m.aid == aid)
    }

    pub fn member(&self, aid: u32) -> Option<&PartyMember> {
        self.members.iter().find(|m| m.aid == aid)
    }

    pub fn leader_aid(&self) -> Option<u32> {
        self.members.iter().find(|m| m.leader).map(|m| m.aid)
    }

    pub fn upsert_member(&mut self, member: PartyMember) {
        if let Some(existing) = self.member_mut(member.aid) {
            // Preserve HP we already learned from 0x106 — the member-info packet doesn't carry it.
            let hp = member.hp.or(existing.hp);
            let max_hp = member.max_hp.or(existing.max_hp);
            *existing = PartyMember {
                hp,
                max_hp,
                ..member
            };
        } else {
            self.members.push(member);
        }
    }

    pub fn remove_member(&mut self, aid: u32) {
        self.members.retain(|m| m.aid != aid);
    }

    pub fn set_hp(&mut self, aid: u32, hp: u32, max_hp: u32) {
        if let Some(m) = self.member_mut(aid) {
            m.hp = Some(hp);
            m.max_hp = Some(max_hp);
        }
    }

    pub fn set_position(&mut self, aid: u32, x: u16, y: u16) {
        if let Some(m) = self.member_mut(aid) {
            m.x = x;
            m.y = y;
            m.has_live_position = true;
        }
    }

    pub fn clear_position_of(&mut self, aid: u32) {
        if let Some(m) = self.member_mut(aid) {
            m.has_live_position = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn member(aid: u32, name: &str, leader: bool) -> PartyMember {
        PartyMember {
            aid,
            name: name.to_string(),
            map: "prontera.gat".to_string(),
            leader,
            online: true,
            hp: None,
            max_hp: None,
            x: 0,
            y: 0,
            has_live_position: false,
        }
    }

    #[test]
    fn party_lifecycle_add_update_remove() {
        let mut party = Party::new("Adventurers".to_string());
        party.upsert_member(member(1, "Leader", true));
        party.upsert_member(member(2, "Sidekick", false));
        assert_eq!(party.members.len(), 2);
        assert_eq!(party.leader_aid(), Some(1));

        party.set_hp(2, 80, 200);
        // A fresh member-info update for the same member must not wipe known HP.
        party.upsert_member(member(2, "Sidekick", false));
        let m = party.member(2).unwrap();
        assert_eq!((m.hp, m.max_hp), (Some(80), Some(200)));

        party.set_position(2, 150, 160);
        let m = party.member(2).unwrap();
        assert_eq!((m.x, m.y, m.has_live_position), (150, 160, true));
        party.clear_position_of(2);
        assert!(!party.member(2).unwrap().has_live_position);

        party.remove_member(1);
        assert_eq!(party.members.len(), 1);
        assert_eq!(party.leader_aid(), None);
    }
}
