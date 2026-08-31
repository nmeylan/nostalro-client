use x11rb::connection::Connection;
use x11rb::protocol::xproto::{AtomEnum, ConnectionExt};

/// `_XKB_RULES_NAMES` on the root window: NUL-separated rules, model, layout,
/// variant and options, which is what the server was configured with.
pub fn rule_names() -> Option<[String; 5]> {
    let (connection, screen) = x11rb::connect(None).ok()?;
    let root = connection.setup().roots.get(screen)?.root;
    let atom = connection
        .intern_atom(true, b"_XKB_RULES_NAMES")
        .ok()?
        .reply()
        .ok()?
        .atom;
    let property = connection
        .get_property(false, root, atom, AtomEnum::STRING, 0, 1024)
        .ok()?
        .reply()
        .ok()?;

    let mut fields = property
        .value
        .split(|byte| *byte == 0)
        .map(|field| String::from_utf8_lossy(field).into_owned());
    Some([
        fields.next()?,
        fields.next().unwrap_or_default(),
        fields.next().unwrap_or_default(),
        fields.next().unwrap_or_default(),
        fields.next().unwrap_or_default(),
    ])
}
