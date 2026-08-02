use ragnarok_game::monster_info::MonsterInfo;

#[test]
fn probe_three_mobs() {
    for (who, info) in [
        (
            "poring",
            MonsterInfo {
                name: "Poring".into(),
                job: 1002,
                level: 1,
                size: 1,
                hp: 50,
                def: 0,
                race: 3,
                mdef: 5,
                property: 1,
                resistances: [100; 9],
            },
        ),
        (
            "pupa",
            MonsterInfo {
                name: "Pupa".into(),
                job: 1008,
                level: 2,
                size: 0,
                hp: 427,
                def: 0,
                race: 4,
                mdef: 20,
                property: 2,
                resistances: [100; 9],
            },
        ),
        (
            "hydra",
            MonsterInfo {
                name: "Hydra".into(),
                job: 1068,
                level: 14,
                size: 0,
                hp: 660,
                def: 0,
                race: 3,
                mdef: 40,
                property: 1,
                resistances: [100; 9],
            },
        ),
    ] {
        println!("--- {who}");
        let r = std::panic::catch_unwind(|| info.info_lines());
        match r {
            Ok(lines) => println!("{} lines: {:?}", lines.len(), &lines[..3]),
            Err(_) => println!("PANIC for {who}"),
        }
    }
}
