pub const MAIL_INBOX_CAP: usize = 30;
pub const MAIL_ROWS_PER_PAGE: usize = 7;
pub const MAIL_TO_MAX: usize = 23;
pub const MAIL_TITLE_MAX: usize = 39;
pub const MAIL_BODY_MAX: usize = 199;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MailboxMode {
    Inbox,
    Compose,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MailEntry {
    pub mail_id: u32,
    pub title: String,
    pub read: bool,
    pub sender: String,
    pub time: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MailItem {
    pub nameid: u16,
    pub amount: u32,
    pub item_type: u16,
    pub identified: bool,
    pub damaged: bool,
    pub refine: u8,
    pub cards: [u16; 4],
}

impl MailItem {
    pub fn to_item(&self, name: String, resource_name: Option<String>) -> crate::item::Item {
        use models::enums::EnumWithNumberValue;
        use models::enums::item::ItemType;
        crate::item::Item {
            index: 0,
            item_id: self.nameid,
            item_type: ItemType::from_value(self.item_type as usize),
            count: self.amount as i16,
            is_identified: self.identified,
            is_damaged: self.damaged,
            refining_level: self.refine,
            slot: self.cards,
            location: 0,
            wear_state: 0,
            name,
            resource_name,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenedMail {
    pub mail_id: u32,
    pub title: String,
    pub sender: String,
    pub zeny: u32,
    pub item: Option<MailItem>,
    pub body: String,
}

impl OpenedMail {
    pub fn has_attachment(&self) -> bool {
        self.zeny > 0 || self.item.is_some()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ComposeItem {
    pub inv_index: u16,
    pub item_id: u16,
    pub amount: u32,
    pub identified: bool,
}

/// Compose attachments mirror the server-side pending mail (`sd->mail`); the To /
/// Title / Body text lives in the window's own inputs, not here. Attaching an item
/// removes it from the inventory view server-side, so we cache the id here for the
/// slot icon rather than looking it back up.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ComposeState {
    pub zeny: u32,
    pub item: Option<ComposeItem>,
    pub pending_item: Option<ComposeItem>,
}

pub struct MailState {
    pub window_open: bool,
    pub mode: MailboxMode,
    pub inbox: Vec<MailEntry>,
    pub page: usize,
    pub opened: Option<OpenedMail>,
    pub read_open: bool,
    pub compose: ComposeState,
    pub compose_prefill: Option<(String, String)>,
    pub send_pending: bool,
}

impl Default for MailState {
    fn default() -> Self {
        Self::new()
    }
}

impl MailState {
    pub fn new() -> Self {
        Self {
            window_open: false,
            mode: MailboxMode::Inbox,
            inbox: Vec::new(),
            page: 0,
            opened: None,
            read_open: false,
            compose: ComposeState::default(),
            compose_prefill: None,
            send_pending: false,
        }
    }

    pub fn clear(&mut self) {
        *self = Self::new();
    }

    pub fn page_count(&self) -> usize {
        self.inbox.len().div_ceil(MAIL_ROWS_PER_PAGE).max(1)
    }

    pub fn clamp_page(&mut self) {
        let last = self.page_count().saturating_sub(1);
        if self.page > last {
            self.page = last;
        }
    }

    /// Switching back to inbox view triggers a 0x23f which drops pending compose
    /// attachments server-side, so clear our mirror to match.
    pub fn switch_to_inbox(&mut self) {
        self.mode = MailboxMode::Inbox;
        self.compose = ComposeState::default();
        self.compose_prefill = None;
        self.send_pending = false;
    }
}

/// Formats a unix timestamp as `MM DD YY` (UTC). Implemented here to avoid a
/// date-crate dependency for a single label.
pub fn format_mail_date(time: u32) -> String {
    if time == 0 {
        return String::new();
    }
    let days = (time / 86_400) as i64;
    // Howard Hinnant's days-from-civil, inverted.
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if month <= 2 { y + 1 } else { y };
    let secs = time % 86_400;
    let hour = secs / 3600;
    let minute = (secs % 3600) / 60;
    format!(
        "{:02}/{:02}/{:02} {:02}:{:02}",
        day,
        month,
        year.rem_euclid(100),
        hour,
        minute
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mail_date_formats_known_timestamp() {
        // 2021-03-14 UTC
        assert_eq!(format_mail_date(1_615_680_000), "14/03/21 00:00");
        assert_eq!(format_mail_date(0), "");
    }

    #[test]
    fn switch_to_inbox_drops_compose_attachments() {
        let mut mail = MailState::new();
        mail.mode = MailboxMode::Compose;
        mail.compose.zeny = 5000;
        mail.compose.item = Some(ComposeItem {
            inv_index: 3,
            item_id: 501,
            amount: 2,
            identified: true,
        });
        mail.send_pending = true;
        mail.switch_to_inbox();
        assert_eq!(mail.mode, MailboxMode::Inbox);
        assert_eq!(mail.compose, ComposeState::default());
        assert!(!mail.send_pending);
    }
}
