use models::enums::effect_id::EffectId;

const MAP_CLOUDS: &[(&str, EffectId)] = &[
    ("gef_fild07", EffectId::Cloud),
    ("mjolnir_01", EffectId::Cloud),
    ("yuno", EffectId::Cloud2),
    ("gonryun", EffectId::Cloud2),
    ("gon_dun02", EffectId::Cloud2),
    ("ra_temsky", EffectId::Cloud2),
    ("que_temsky", EffectId::Cloud2),
    ("sch_gld", EffectId::Cloud2),
    ("bat_fild02", EffectId::Cloud2),
    ("bat_b01", EffectId::Cloud2),
    ("bat_b02", EffectId::Cloud2),
    ("valkyrie", EffectId::Cloud3),
    ("rwc01", EffectId::Cloud3),
    ("himinn", EffectId::Cloud3),
    ("que_qsch01", EffectId::Cloud3),
    ("que_qsch02", EffectId::Cloud3),
    ("que_qsch03", EffectId::Cloud3),
    ("que_qsch04", EffectId::Cloud3),
    ("que_qsch05", EffectId::Cloud3),
    ("que_qaru01", EffectId::Cloud3),
    ("que_qaru02", EffectId::Cloud3),
    ("que_qaru03", EffectId::Cloud3),
    ("que_qaru04", EffectId::Cloud3),
    ("que_qaru05", EffectId::Cloud3),
    ("einbroch", EffectId::Cloud4),
    ("airplane", EffectId::Cloud5),
    ("airplane_01", EffectId::Cloud5),
    ("thana_boss", EffectId::Cloud6),
    ("moc_fild22", EffectId::Cloud6),
    ("moc_fild22b", EffectId::Cloud6),
    ("6@tower", EffectId::Cloud7),
    ("5@tower", EffectId::Cloud8),
];

const SKY_LIGHT_BLUE: [u8; 3] = [153, 204, 255];
const SKY_DEEP_BLUE: [u8; 3] = [102, 153, 204];
const SKY_SAND: [u8; 3] = [224, 213, 194];
const SKY_VIOLET: [u8; 3] = [51, 0, 51];

const MAP_BACKGROUNDS: &[(&str, [u8; 3])] = &[
    ("yuno", SKY_LIGHT_BLUE),
    ("valkyrie", SKY_LIGHT_BLUE),
    ("rwc01", SKY_LIGHT_BLUE),
    ("himinn", SKY_LIGHT_BLUE),
    ("airplane", SKY_LIGHT_BLUE),
    ("airplane_01", SKY_LIGHT_BLUE),
    ("sch_gld", SKY_LIGHT_BLUE),
    ("bat_fild02", SKY_LIGHT_BLUE),
    ("que_qsch01", SKY_LIGHT_BLUE),
    ("que_qsch02", SKY_LIGHT_BLUE),
    ("que_qsch03", SKY_LIGHT_BLUE),
    ("que_qsch04", SKY_LIGHT_BLUE),
    ("que_qsch05", SKY_LIGHT_BLUE),
    ("que_qaru01", SKY_LIGHT_BLUE),
    ("que_qaru02", SKY_LIGHT_BLUE),
    ("que_qaru03", SKY_LIGHT_BLUE),
    ("que_qaru04", SKY_LIGHT_BLUE),
    ("que_qaru05", SKY_LIGHT_BLUE),
    ("bat_b01", SKY_LIGHT_BLUE),
    ("bat_b02", SKY_LIGHT_BLUE),
    ("gonryun", SKY_DEEP_BLUE),
    ("gon_dun02", SKY_DEEP_BLUE),
    ("ra_temsky", SKY_DEEP_BLUE),
    ("que_temsky", SKY_DEEP_BLUE),
    ("thana_boss", SKY_SAND),
    ("5@tower", SKY_VIOLET),
];

fn map_stem(map_name: &str) -> &str {
    let base = map_name
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(map_name)
        .trim_end_matches(char::from(0));
    match base.len().checked_sub(4) {
        Some(cut) if base[cut..].eq_ignore_ascii_case(".rsw") => &base[..cut],
        _ => base,
    }
}

pub fn map_cloud_effect(map_name: &str) -> Option<EffectId> {
    let stem = map_stem(map_name);
    MAP_CLOUDS
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(stem))
        .map(|(_, id)| *id)
}

pub fn map_background_color(map_name: &str) -> Option<[f32; 3]> {
    let stem = map_stem(map_name);
    MAP_BACKGROUNDS
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(stem))
        .map(|(_, [r, g, b])| [*r as f32 / 255.0, *g as f32 / 255.0, *b as f32 / 255.0])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_each_variant_and_ignores_case_path_and_extension() {
        assert_eq!(map_cloud_effect("einbroch"), Some(EffectId::Cloud4));
        assert_eq!(map_cloud_effect("EINBROCH.RSW"), Some(EffectId::Cloud4));
        assert_eq!(
            map_cloud_effect("data\\einbroch.rsw"),
            Some(EffectId::Cloud4)
        );
        assert_eq!(map_cloud_effect("mjolnir_01"), Some(EffectId::Cloud));
        assert_eq!(map_cloud_effect("yuno"), Some(EffectId::Cloud2));
        assert_eq!(map_cloud_effect("valkyrie"), Some(EffectId::Cloud3));
        assert_eq!(map_cloud_effect("airplane_01"), Some(EffectId::Cloud5));
        assert_eq!(map_cloud_effect("thana_boss"), Some(EffectId::Cloud6));
    }

    #[test]
    fn tower_floors_are_matched_literally_and_not_swapped() {
        assert_eq!(map_cloud_effect("6@tower"), Some(EffectId::Cloud7));
        assert_eq!(map_cloud_effect("5@tower"), Some(EffectId::Cloud8));
        // 1-4 are Endless Tower floors the original gives no cloud field.
        assert_eq!(map_cloud_effect("1@tower"), None);
        assert_eq!(map_cloud_effect("4@tower"), None);
    }

    #[test]
    fn unlisted_maps_get_nothing() {
        assert_eq!(map_cloud_effect("prontera"), None);
        assert_eq!(map_cloud_effect("einbech"), None);
        assert_eq!(map_cloud_effect(""), None);
    }

    #[test]
    fn background_colours_split_the_blue_maps_and_leave_the_rest_black() {
        let light = map_background_color("yuno").unwrap();
        let deep = map_background_color("GONRYUN.RSW").unwrap();
        assert_eq!(light, [153.0 / 255.0, 204.0 / 255.0, 255.0 / 255.0]);
        assert_eq!(deep, [102.0 / 255.0, 153.0 / 255.0, 204.0 / 255.0]);
        assert_eq!(map_background_color("thana_boss").unwrap()[0], 224.0 / 255.0);
        assert_eq!(map_background_color("5@tower").unwrap(), [51.0 / 255.0, 0.0, 51.0 / 255.0]);

        // Cloud maps that stay black.
        for map in ["einbroch", "mjolnir_01", "gef_fild07", "moc_fild22", "6@tower"] {
            assert!(map_cloud_effect(map).is_some());
            assert_eq!(map_background_color(map), None, "{map}");
        }
        assert_eq!(map_background_color("prontera"), None);
    }
}
