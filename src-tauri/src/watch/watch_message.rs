use notify::Event;

pub(super) enum WatchMessage {
    Change(notify::Result<Event>),
    Stop,
}
