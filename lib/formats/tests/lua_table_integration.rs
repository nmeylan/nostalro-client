use ragnarok_formats::lua_table;

#[test]
fn parse_real_accessory_lua_files() {
    let id_bytes = include_bytes!("fixtures/accessoryid.lua");
    let name_bytes = include_bytes!("fixtures/accname.lua");

    let id_content = lua_table::decode_euc_kr(id_bytes);
    let name_content = lua_table::decode_euc_kr(name_bytes);

    let table = lua_table::build_accessory_table(&id_content, &name_content);

    assert!(
        table.len() > 500,
        "expected many entries, got {}",
        table.len()
    );

    assert_eq!(table.get(&1).map(|s| s.as_str()), Some("_고글"));
    assert_eq!(table.get(&17).map(|s| s.as_str()), Some("_리본"));
    assert_eq!(table.get(&14).map(|s| s.as_str()), Some("_캡"));
}

#[test]
fn parse_real_idnum2itemresnametable() {
    let data = include_bytes!("fixtures/idnum2itemresnametable.txt");

    let table = lua_table::parse_item_res_table(data);

    assert!(
        table.len() > 1000,
        "expected many entries, got {}",
        table.len()
    );

    assert_eq!(table.get(&501).map(|s| s.as_str()), Some("빨간포션"));
    assert_eq!(table.get(&664).map(|s| s.as_str()), Some("선물상자_1"));
}

#[test]
fn parse_accessory_lub_lua50() {
    let table = lua_table::build_accessory_table_from_lub(
        include_bytes!("fixtures/acid_lua50.lub"),
        include_bytes!("fixtures/aname_lua50.lub"),
    )
    .expect("lua 5.0 accessory chunks");

    assert_eq!(table.len(), 859);
    assert_eq!(table.get(&1).map(|s| s.as_str()), Some("_고글"));
    assert_eq!(table.get(&14).map(|s| s.as_str()), Some("_캡"));
    assert_eq!(table.get(&17).map(|s| s.as_str()), Some("_리본"));
}

#[test]
fn parse_accessory_lub_lua51() {
    let table = lua_table::build_accessory_table_from_lub(
        include_bytes!("fixtures/acid_lua51.lub"),
        include_bytes!("fixtures/aname_lua51.lub"),
    )
    .expect("lua 5.1 accessory chunks");

    assert_eq!(table.len(), 862);
    assert_eq!(table.get(&1).map(|s| s.as_str()), Some("_고글"));
    assert_eq!(table.get(&14).map(|s| s.as_str()), Some("_캡"));
    assert_eq!(table.get(&17).map(|s| s.as_str()), Some("_리본"));
}

#[test]
fn parse_npcidentity_lub() {
    let lua50 = lua_table::parse_jt_identity_lub(include_bytes!("fixtures/npcid_lua50.lub"))
        .expect("lua 5.0 identity chunk");
    let lua51 = lua_table::parse_jt_identity_lub(include_bytes!("fixtures/npcid_lua51.lub"))
        .expect("lua 5.1 identity chunk");

    for table in [&lua50, &lua51] {
        assert!(
            table.len() > 2000,
            "expected many entries, got {}",
            table.len()
        );
        assert_eq!(table.get(&1002).map(|s| s.as_str()), Some("JT_PORING"));
        assert_eq!(table.get(&1885).map(|s| s.as_str()), Some("JT_GOPINICH"));
    }
}

#[test]
fn job_name_table_resolves_homunculus_sprite_names() {
    let identity = include_bytes!("fixtures/npcid_lua51.lub").as_slice();
    let job_name = include_bytes!("fixtures/jname_lua51.lub").as_slice();

    let names = lua_table::parse_job_name_lub(&[identity, job_name]).expect("jobname chunk");

    assert!(
        names.len() > 2000,
        "expected many entries, got {}",
        names.len()
    );
    // The identity constants are JT_MER_*; the sprite files are not.
    assert_eq!(names.get(&6001).map(|s| s.as_str()), Some("LIF"));
    assert_eq!(names.get(&6003).map(|s| s.as_str()), Some("FILIR"));
    assert_eq!(names.get(&6011).map(|s| s.as_str()), Some("FILIR_H"));
    assert_eq!(names.get(&1002).map(|s| s.as_str()), Some("Poring"));
}
