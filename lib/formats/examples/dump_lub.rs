use std::path::Path;

fn main() {
    let grf_path = std::env::args().nth(1).unwrap_or_else(|| "data/data.grf".into());
    let grf = ragnarok_formats::grf::GrfArchive::open(Path::new(&grf_path)).expect("open grf");
    
    // Read the skilleffectinfolist.lub file
    let data = grf.read_file("data/luafiles514/lua files/skilleffectinfo/skilleffectinfolist.lub")
        .expect("read lub");
    
    println!("lub file size: {} bytes", data.len());
    
    // Try to use mlua to load it and print globals
    let lua = mlua::Lua::new();
    match lua.load(&data) {
        chunk => {
            match chunk.exec() {
                Ok(_) => {
                    println!("Loaded successfully. Globals:");
                    let globals = lua.globals();
                    for key in globals.pairs::<mlua::Value, mlua::Value>() {
                        if let Ok((k, v)) = key {
                            println!("  {:?} -> {:?}", k, v.type_name());
                        }
                    }
                }
                Err(e) => println!("Exec error: {}", e),
            }
        }
    }
}
