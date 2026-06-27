use ragnarok_formats::grf::GrfArchive;
use std::path::PathBuf;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let grf = args.get(1).ok_or("usage: dump_file <grf> <name> <out>")?;
    let name = args.get(2).ok_or("need name")?;
    let out = args.get(3).ok_or("need out")?;
    let archive = GrfArchive::open(&PathBuf::from(grf))?;
    let data = archive.read_file(name)?;
    std::fs::write(out, &data)?;
    eprintln!("wrote {} bytes", data.len());
    Ok(())
}
