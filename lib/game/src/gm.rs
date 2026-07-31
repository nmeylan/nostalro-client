//! Manner points and the GM `/check` stat block.

/// Points sent per manner adjustment; the server reads the amount as minutes of
/// chat block.
pub const MANNER_POINT_STEP: i16 = 60;

pub const MANNER_TYPE_PLUS: u8 = 0;
pub const MANNER_TYPE_MINUS: u8 = 1;

/// Feedback for the `result` of a manner-point request. `5` (a chat block
/// lifted by an operator) carries no line.
pub fn manner_result_line(result: u32) -> Option<&'static str> {
    match result {
        0 => Some("The manner point was sent successfully."),
        1 => Some("You have already used your manner points today."),
        2 => Some("A month has not passed since you last gave this player a manner point."),
        3 => Some("A GM has blocked your chat because of your ill-mannered behaviour."),
        4 => Some("The anti-spam system has blocked your chat."),
        _ => None,
    }
}

pub fn manner_given_line(positive: bool, other_name: &str) -> String {
    if positive {
        format!("You received a plus manner point from {other_name}.")
    } else {
        format!("You received a minus manner point from {other_name}.")
    }
}

/// Stat block of another character, as answered to a GM `/check`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GmStatus {
    pub str: u8,
    pub str_cost: u8,
    pub agi: u8,
    pub agi_cost: u8,
    pub vit: u8,
    pub vit_cost: u8,
    pub int: u8,
    pub int_cost: u8,
    pub dex: u8,
    pub dex_cost: u8,
    pub luk: u8,
    pub luk_cost: u8,
    pub atk: i16,
    pub atk_plus: i16,
    pub matk_max: i16,
    pub matk_min: i16,
    pub def: i16,
    pub def_plus: i16,
    pub mdef: i16,
    pub mdef_plus: i16,
    pub hit: i16,
    pub flee: i16,
    pub flee_plus: i16,
    pub critical: i16,
    pub aspd: i16,
    pub aspd_plus: i16,
}

impl GmStatus {
    pub fn lines(&self) -> Vec<String> {
        vec![
            format!(
                "STR {}  AGI {}  VIT {}  INT {}  DEX {}  LUK {}",
                self.str, self.agi, self.vit, self.int, self.dex, self.luk
            ),
            format!(
                "Point cost: STR {}  AGI {}  VIT {}  INT {}  DEX {}  LUK {}",
                self.str_cost,
                self.agi_cost,
                self.vit_cost,
                self.int_cost,
                self.dex_cost,
                self.luk_cost
            ),
            format!(
                "ATK {}+{}  MATK {}~{}  HIT {}  CRIT {}",
                self.atk, self.atk_plus, self.matk_min, self.matk_max, self.hit, self.critical
            ),
            format!(
                "DEF {}+{}  MDEF {}+{}  FLEE {}+{}  ASPD {}+{}",
                self.def,
                self.def_plus,
                self.mdef,
                self.mdef_plus,
                self.flee,
                self.flee_plus,
                self.aspd,
                self.aspd_plus
            ),
        ]
    }
}
