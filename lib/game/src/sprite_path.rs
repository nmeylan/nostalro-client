fn job_name_kr(job_class: u16) -> &'static str {
    match job_class {
        0 => "초보자",
        1 => "검사",
        2 => "마법사",
        3 => "궁수",
        4 => "성직자",
        5 => "상인",
        6 => "도둑",
        7 => "기사",
        8 => "프리스트",
        9 => "위저드",
        10 => "제철공",
        11 => "헌터",
        12 => "어세신",
        14 => "크루세이더",
        15 => "몽크",
        16 => "세이지",
        17 => "로그",
        18 => "연금술사",
        19 => "바드",
        20 => "무희",
        23 => "슈퍼노비스",
        // Transcendent 1st classes reuse base sprites
        4001 => "초보자",
        4002 => "검사",
        4003 => "마법사",
        4004 => "궁수",
        4005 => "성직자",
        4006 => "상인",
        4007 => "도둑",
        // Transcendent 2nd classes have their own sprites
        4008 => "로드나이트",
        4009 => "하이프리",
        4010 => "하이위저드",
        4011 => "화이트스미스",
        4012 => "스나이퍼",
        4013 => "어쌔신크로스",
        4015 => "팔라딘",
        4016 => "챔피온",
        4017 => "프로페서",
        4018 => "스토커",
        4019 => "크리에이터",
        4020 => "클라운",
        4021 => "집시",
        _ => "초보자",
    }
}

fn sex_kr(sex: u8) -> &'static str {
    if sex == 0 { "여" } else { "남" }
}

/// Returns the GRF path base for a body sprite (without file extension).
/// Example: `data/sprite/인간족/몸통/남/초보자_남`
pub fn body_sprite_path(job_class: u16, sex: u8) -> String {
    let job = job_name_kr(job_class);
    let sex_str = sex_kr(sex);
    format!("data/sprite/인간족/몸통/{sex_str}/{job}_{sex_str}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn novice_male_path() {
        assert_eq!(
            body_sprite_path(0, 1),
            "data/sprite/인간족/몸통/남/초보자_남"
        );
    }

    #[test]
    fn novice_female_path() {
        assert_eq!(
            body_sprite_path(0, 0),
            "data/sprite/인간족/몸통/여/초보자_여"
        );
    }

    #[test]
    fn knight_male_path() {
        assert_eq!(
            body_sprite_path(7, 1),
            "data/sprite/인간족/몸통/남/기사_남"
        );
    }

    #[test]
    fn lord_knight_has_own_sprite() {
        assert_eq!(
            body_sprite_path(4008, 1),
            "data/sprite/인간족/몸통/남/로드나이트_남"
        );
    }

    #[test]
    fn clown_has_own_sprite() {
        assert_eq!(
            body_sprite_path(4020, 1),
            "data/sprite/인간족/몸통/남/클라운_남"
        );
    }

    #[test]
    fn high_priest_path() {
        assert_eq!(
            body_sprite_path(4009, 1),
            "data/sprite/인간족/몸통/남/하이프리_남"
        );
    }

    #[test]
    fn unknown_class_falls_back_to_novice() {
        assert_eq!(
            body_sprite_path(9999, 1),
            "data/sprite/인간족/몸통/남/초보자_남"
        );
    }
}
