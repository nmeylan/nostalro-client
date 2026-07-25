/// Monster info the server packs into the party name field of `ZC_ACK_REQNAMEALL`.
/// The field is 24 bytes, so a string carrying several segments arrives truncated;
/// segments that do not parse in full are dropped.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MobInfo {
    pub level: Option<u16>,
    pub hp: Option<u32>,
    pub max_hp: Option<u32>,
    pub hp_percent: Option<u8>,
}

impl MobInfo {
    pub fn parse(raw: &str) -> Option<MobInfo> {
        let mut info = MobInfo::default();
        for segment in raw.split('|').map(str::trim).filter(|s| !s.is_empty()) {
            if let Some(level) = segment.strip_prefix("Lv. ") {
                info.level = level.trim().parse().ok();
            } else if let Some(hp) = segment.strip_prefix("HP: ") {
                let hp = hp.trim();
                if let Some(percent) = hp.strip_suffix('%') {
                    info.hp_percent = percent.parse().ok();
                } else if let Some((current, max)) = hp.split_once('/')
                    && let (Ok(current), Ok(max)) = (current.parse(), max.parse())
                {
                    info.hp = Some(current);
                    info.max_hp = Some(max);
                }
            }
        }
        (info != MobInfo::default()).then_some(info)
    }

    pub fn hp_ratio(&self) -> Option<f32> {
        match (self.hp, self.max_hp) {
            (Some(hp), Some(max_hp)) if max_hp > 0 => Some(hp as f32 / max_hp as f32),
            _ => self.hp_percent.map(|percent| percent as f32 / 100.0),
        }
    }

    pub fn label(&self) -> String {
        let mut segments: Vec<String> = Vec::new();
        if let Some(level) = self.level {
            segments.push(format!("Lv. {level}"));
        }
        if let (Some(hp), Some(max_hp)) = (self.hp, self.max_hp) {
            segments.push(format!("HP: {hp}/{max_hp}"));
        }
        if let Some(percent) = self.hp_percent {
            segments.push(format!("HP: {percent}%"));
        }
        segments.join(" | ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_shapes_the_server_sends() {
        let hp = MobInfo::parse("HP: 100/200").unwrap();
        assert_eq!(hp.hp_ratio(), Some(0.5));
        assert_eq!(hp.label(), "HP: 100/200");

        let percent = MobInfo::parse("HP: 50%").unwrap();
        assert_eq!(percent.hp_ratio(), Some(0.5));
        assert_eq!(percent.label(), "HP: 50%");

        let all = MobInfo::parse("Lv. 99 | HP: 100/200 | HP: 50%").unwrap();
        assert_eq!(all.level, Some(99));
        assert_eq!(all.hp_ratio(), Some(0.5));
        assert_eq!(all.label(), "Lv. 99 | HP: 100/200 | HP: 50%");

        let truncated = MobInfo::parse("Lv. 99 | HP: 100/200 |").unwrap();
        assert_eq!(truncated.label(), "Lv. 99 | HP: 100/200");

        let cut_mid_number = MobInfo::parse("Lv. 99 | HP: 12345/123").unwrap();
        assert_eq!(cut_mid_number.label(), "Lv. 99 | HP: 12345/123");

        let level_only = MobInfo::parse("Lv. 42").unwrap();
        assert_eq!(level_only.hp_ratio(), None);
        assert_eq!(level_only.label(), "Lv. 42");

        assert!(MobInfo::parse("Knights").is_none());
        assert!(MobInfo::parse("").is_none());
    }
}
