use anyhow::Result;

#[derive(Debug, Clone, Copy)]
pub struct ManagerInfo {
    pub name: &'static str,
    pub display_name: &'static str,
    pub is_wayland: bool,
}

pub const KNOWN_MANAGERS: &[ManagerInfo] = &[
    // Wayland
    ManagerInfo {
        name: "wl-clipboard",
        display_name: "wl-clipboard (wl-copy/wl-paste)",
        is_wayland: true,
    },
    ManagerInfo {
        name: "clipman",
        display_name: "clipman",
        is_wayland: true,
    },
    ManagerInfo {
        name: "wofi-clipboard",
        display_name: "wofi-clipboard",
        is_wayland: true,
    },
    ManagerInfo {
        name: "cliphist",
        display_name: "cliphist",
        is_wayland: true,
    },
    ManagerInfo {
        name: "greenclip",
        display_name: "greenclip",
        is_wayland: true,
    },
    // X11
    ManagerInfo {
        name: "parcellite",
        display_name: "parcellite",
        is_wayland: false,
    },
    ManagerInfo {
        name: "clipit",
        display_name: "clipit",
        is_wayland: false,
    },
    ManagerInfo {
        name: "diodon",
        display_name: "diodon",
        is_wayland: false,
    },
    ManagerInfo {
        name: "copyq",
        display_name: "copyq",
        is_wayland: false,
    },
    ManagerInfo {
        name: "gpaste-daemon",
        display_name: "gpaste (gpaste-daemon)",
        is_wayland: false,
    },
    ManagerInfo {
        name: "clipster",
        display_name: "clipster",
        is_wayland: false,
    },
];

#[cfg(target_os = "linux")]
mod linux {
    use super::{KNOWN_MANAGERS, Result};
    use anyhow::bail;
    use std::env;
    use std::sync::OnceLock;
    use wayland_client::{
        Connection, Dispatch, QueueHandle,
        globals::{GlobalListContents, registry_queue_init},
        protocol::wl_registry::WlRegistry,
    };
    use x11rb::protocol::xproto::ConnectionExt;

    /// Empty state required by the Wayland event queue.
    struct AppState;

    impl Dispatch<WlRegistry, GlobalListContents> for AppState {
        fn event(
            _state: &mut Self,
            _proxy: &WlRegistry,
            _event: <WlRegistry as wayland_client::Proxy>::Event,
            _data: &GlobalListContents,
            _conn: &Connection,
            _qhandle: &QueueHandle<Self>,
        ) {
            // registry_queue_init populates the globals list automatically
        }
    }

    /// Check for known standalone clipboard manager processes in a single /proc scan.
    fn check_clipboard_processes() -> bool {
        static SCAN_CACHE: OnceLock<bool> = OnceLock::new();

        *SCAN_CACHE.get_or_init(|| {
            let Ok(entries) = std::fs::read_dir("/proc") else {
                return false;
            };

            for entry in entries.flatten() {
                let path = entry.path();
                let Some(filename) = path.file_name().and_then(|n| n.to_str()) else {
                    continue;
                };
                if !filename.chars().all(|c| c.is_ascii_digit()) {
                    continue;
                }

                // Check comm (exact match, truncated to 15 chars by kernel)
                if let Ok(comm) = std::fs::read_to_string(path.join("comm")) {
                    let trimmed = comm.trim();
                    if KNOWN_MANAGERS.iter().any(|m| m.name == trimmed) {
                        return true;
                    }
                }

                // Fallback to cmdline for longer process names (e.g. gpaste-daemon, wofi-clipboard)
                if let Ok(cmdline) = std::fs::read_to_string(path.join("cmdline")) {
                    if cmdline.split('\0').any(|arg| {
                        let base = std::path::Path::new(arg)
                            .file_name()
                            .and_then(|f| f.to_str())
                            .unwrap_or(arg);
                        KNOWN_MANAGERS.iter().any(|m| m.name == base)
                    }) {
                        return true;
                    }
                }
            }
            false
        })
    }

    /// Check for Wayland clipboard support via data-control protocols,
    /// falling back to a process scan.
    pub fn wayland_check() -> bool {
        let Ok(conn) = Connection::connect_to_env() else {
            return check_clipboard_processes();
        };

        let Ok((globals, _event_queue)) = registry_queue_init::<AppState>(&conn) else {
            return check_clipboard_processes();
        };

        for global in globals.contents().clone_list() {
            if global.interface == "zwlr_data_control_manager_v1"
                || global.interface == "ext_data_control_manager_v1"
            {
                return true;
            }
        }

        check_clipboard_processes()
    }

    /// Check for X11 clipboard support via x11rb (pure Rust, safe, no C libX11 dependency),
    /// checking CLIPBOARD_MANAGER and CLIPBOARD ownership, then falling back to a process scan.
    pub fn x11_check() -> bool {
        let Ok((conn, _screen_num)) = x11rb::connect(None) else {
            return check_clipboard_processes();
        };

        // ICCCM convention: a clipboard manager owns CLIPBOARD_MANAGER
        if let Ok(atom_cookie) = conn.intern_atom(true, b"CLIPBOARD_MANAGER") {
            if let Ok(atom_reply) = atom_cookie.reply() {
                if atom_reply.atom != 0 {
                    if let Ok(owner_cookie) = conn.get_selection_owner(atom_reply.atom) {
                        if let Ok(owner_reply) = owner_cookie.reply() {
                            if owner_reply.owner != 0 {
                                return true;
                            }
                        }
                    }
                }
            }
        }

        // Fallback: anything owning the CLIPBOARD selection
        if let Ok(atom_cookie) = conn.intern_atom(true, b"CLIPBOARD") {
            if let Ok(atom_reply) = atom_cookie.reply() {
                if atom_reply.atom != 0 {
                    if let Ok(owner_cookie) = conn.get_selection_owner(atom_reply.atom) {
                        if let Ok(owner_reply) = owner_cookie.reply() {
                            if owner_reply.owner != 0 {
                                return true;
                            }
                        }
                    }
                }
            }
        }

        check_clipboard_processes()
    }

    /// The possible session states clp can run in.
    pub enum Platform {
        Wayland,
        X11,
        Headless,
        Both,
    }

    /// Detect the session type from environment variables.
    pub fn detect_platform() -> Platform {
        let wayland = env::var("WAYLAND_DISPLAY").is_ok();
        let x11 = env::var("DISPLAY").is_ok();

        match (x11, wayland) {
            (true, true) => Platform::Both,
            (true, false) => Platform::X11,
            (false, true) => Platform::Wayland,
            (false, false) => Platform::Headless,
        }
    }

    /// Check whether a usable clipboard manager is present.
    pub fn can_proceed() -> Result<bool> {
        match detect_platform() {
            Platform::Both => Ok(wayland_check() || x11_check()),
            Platform::Wayland => Ok(wayland_check()),
            Platform::X11 => Ok(x11_check()),
            Platform::Headless => {
                if check_clipboard_processes() {
                    Ok(true)
                } else {
                    bail!("Headless server not supported")
                }
            }
        }
    }
}

#[cfg(target_os = "linux")]
pub use linux::can_proceed;

#[cfg(not(target_os = "linux"))]
pub fn can_proceed() -> Result<bool> {
    Ok(true)
}
