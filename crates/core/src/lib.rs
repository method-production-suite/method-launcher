//! Method Launcher's logic, with no window and no network: the built-in catalog of the
//! suite's apps (`catalog`), the systems and the per-system traits (`platform`), version
//! comparison (`version`), release checksums (`checksums`), the macOS checks a downloaded app
//! must pass before it is copied (`verify`, `macho`) and the hosts the launcher may reach
//! (`net`). The rules behind each are in docs/spec.md.

pub mod catalog;
pub mod checksums;
pub mod macho;
pub mod net;
pub mod platform;
pub mod verify;
pub mod version;
