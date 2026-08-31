use std::io::Read;
use wayland_client::protocol::{wl_keyboard, wl_registry, wl_seat};
use wayland_client::{Connection, Dispatch, QueueHandle, WEnum};

/// The compositor hands out the keymap on `wl_keyboard`, which the windowing
/// layer keeps to itself, so this opens a connection of its own and closes it
/// again once the keymap has arrived.
pub fn keymap_string() -> Option<String> {
    let connection = Connection::connect_to_env().ok()?;
    let mut queue = connection.new_event_queue();
    let handle = queue.handle();
    connection.display().get_registry(&handle, ());

    let mut state = State::default();
    for _ in 0..3 {
        queue.roundtrip(&mut state).ok()?;
        if state.keymap.is_some() {
            break;
        }
    }
    state.keymap.take()
}

#[derive(Default)]
struct State {
    keymap: Option<String>,
}

impl Dispatch<wl_registry::WlRegistry, ()> for State {
    fn event(
        _: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        handle: &QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
            && interface == "wl_seat"
        {
            registry.bind::<wl_seat::WlSeat, _, _>(name, version.min(5), handle, ());
        }
    }
}

impl Dispatch<wl_seat::WlSeat, ()> for State {
    fn event(
        _: &mut Self,
        seat: &wl_seat::WlSeat,
        event: wl_seat::Event,
        _: &(),
        _: &Connection,
        handle: &QueueHandle<Self>,
    ) {
        if let wl_seat::Event::Capabilities {
            capabilities: WEnum::Value(capabilities),
        } = event
            && capabilities.contains(wl_seat::Capability::Keyboard)
        {
            seat.get_keyboard(handle, ());
        }
    }
}

impl Dispatch<wl_keyboard::WlKeyboard, ()> for State {
    fn event(
        state: &mut Self,
        _: &wl_keyboard::WlKeyboard,
        event: wl_keyboard::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wl_keyboard::Event::Keymap {
            format: WEnum::Value(wl_keyboard::KeymapFormat::XkbV1),
            fd,
            size,
        } = event
        {
            let mut source = std::fs::File::from(fd).take(size as u64);
            let mut text = String::new();
            if source.read_to_string(&mut text).is_ok() {
                state.keymap = Some(text.trim_end_matches('\0').to_string());
            }
        }
    }
}
