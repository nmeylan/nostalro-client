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

    assert!(table.get(&501).is_some(), "expected item 501 to exist");
}
