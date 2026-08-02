use ragnarok_formats::grf::GrfArchive;
use ragnarok_formats::lua_table::decode_euc_kr;

const MSG_STRING_PATH: &str = ragnarok_resources::table::MSG_STRING;

const MSI_CANNOT_PARTYCALL: u16 = 1221;
const MSI_NO_PARTYMEM_ON_THISMAP: u16 = 1222;

/// Which of the server's msgstringtable messages are shown in the error colour
/// rather than the default notice colour.
pub fn is_error_msg(id: u16) -> bool {
    matches!(id, MSI_CANNOT_PARTYCALL | MSI_NO_PARTYMEM_ON_THISMAP)
}

/// Server text feedback, indexed zero-based by the id carried in `ZC_MSG`.
pub struct MsgStringTable {
    entries: Vec<String>,
}

impl MsgStringTable {
    pub fn parse(data: &[u8]) -> Self {
        let text = decode_euc_kr(data);
        let entries = text
            .split('#')
            .map(|entry| entry.trim_start_matches(['\r', '\n']).to_string())
            .collect();
        Self { entries }
    }

    pub fn load(grf: &GrfArchive) -> Self {
        let table = grf
            .read_file(MSG_STRING_PATH)
            .map(|data| Self::parse(&data))
            .unwrap_or_else(|_| Self {
                entries: Vec::new(),
            });

        tracing::info!("Loaded msg string table: {} entries", table.entries.len());
        table
    }

    pub fn get(&self, id: u16) -> Option<&str> {
        self.entries
            .get(id as usize)
            .map(|s| s.as_str())
            .filter(|s| !s.is_empty())
    }

    /// Fills the entry's `%s` and `%d` placeholders from `args`, in order.
    pub fn format(&self, id: u16, args: &[&str]) -> Option<String> {
        self.get(id).map(|template| substitute(template, args))
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

fn substitute(template: &str, args: &[&str]) -> String {
    let mut out = String::with_capacity(template.len());
    let mut chars = template.chars();
    let mut args = args.iter();
    while let Some(c) = chars.next() {
        if c != '%' {
            out.push(c);
            continue;
        }
        match chars.clone().next() {
            Some('%') => {
                chars.next();
                out.push('%');
            }
            Some(placeholder @ ('s' | 'd')) => {
                chars.next();
                match args.next() {
                    Some(arg) => out.push_str(arg),
                    None => {
                        out.push('%');
                        out.push(placeholder);
                    }
                }
            }
            _ => out.push('%'),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indexes_are_zero_based_across_crlf_lines() {
        let table =
            MsgStringTable::parse(b"Do you agree?#\r\nFailed to Connect to Server.#\r\nLast line");

        assert_eq!(table.get(0), Some("Do you agree?"));
        assert_eq!(table.get(1), Some("Failed to Connect to Server."));
        assert_eq!(table.get(2), Some("Last line"));
        assert_eq!(table.get(3), None);
    }

    #[test]
    fn format_fills_placeholders_in_order_and_keeps_missing_ones() {
        let table = MsgStringTable::parse(b"[Mission] Target: %s (%d%%)#Hi %s and %s");

        assert_eq!(
            table.format(0, &["Poring", "40"]).as_deref(),
            Some("[Mission] Target: Poring (40%)")
        );
        assert_eq!(
            table.format(1, &["Bongun"]).as_deref(),
            Some("Hi Bongun and %s")
        );
    }
}
