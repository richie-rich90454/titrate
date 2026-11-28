//! Typed capability tags and the native-to-capability registry.
//!
//! Every native entry point is classified into exactly one capability. An
//! untrusted policy grants a small safe set; anything else is denied at the
//! dispatch gate. Unknown names are treated as `Pure` only because the registry
//! is conservative on the *dangerous* side: all families known to touch the
//! process, network, FFI, filesystem, or host resources are explicitly listed.
//! Hosts that add natives should register an explicit capability via
//! [`CapabilityRegistry::register`].

use std::collections::HashMap;

/// A typed capability that a native call may require.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Capability {
    Pure,
    Io,
    FileSystemRead,
    FileSystemWrite,
    Process,
    Network,
    Ffi,
    Signals,
    Registry,
    SystemControl,
    Threading,
    Database,
    Graphics,
    Audio,
    Environment,
    ResourceBudget,
}

impl Capability {
    /// All variants, in bit order. Kept explicit so adding a variant forces an
    /// update to the bitmask below.
    pub const ALL: [Capability; 16] = [
        Capability::Pure,
        Capability::Io,
        Capability::FileSystemRead,
        Capability::FileSystemWrite,
        Capability::Process,
        Capability::Network,
        Capability::Ffi,
        Capability::Signals,
        Capability::Registry,
        Capability::SystemControl,
        Capability::Threading,
        Capability::Database,
        Capability::Graphics,
        Capability::Audio,
        Capability::Environment,
        Capability::ResourceBudget,
    ];

    /// Bit position used by [`CapabilitySet`].
    pub const fn bit(self) -> u32 {
        1u32 << (self as u32)
    }

    /// Stable uppercase name used in diagnostics and audit records.
    pub const fn name(self) -> &'static str {
        match self {
            Capability::Pure => "PURE",
            Capability::Io => "IO",
            Capability::FileSystemRead => "FILESYSTEM_READ",
            Capability::FileSystemWrite => "FILESYSTEM_WRITE",
            Capability::Process => "PROCESS",
            Capability::Network => "NETWORK",
            Capability::Ffi => "FFI",
            Capability::Signals => "SIGNALS",
            Capability::Registry => "REGISTRY",
            Capability::SystemControl => "SYSTEM_CONTROL",
            Capability::Threading => "THREADING",
            Capability::Database => "DATABASE",
            Capability::Graphics => "GRAPHICS",
            Capability::Audio => "AUDIO",
            Capability::Environment => "ENVIRONMENT",
            Capability::ResourceBudget => "RESOURCE_BUDGET",
        }
    }

    /// Whether this capability is withheld from the untrusted default.
    pub const fn is_dangerous(self) -> bool {
        !matches!(
            self,
            Capability::Pure
                | Capability::Io
                | Capability::FileSystemRead
                | Capability::FileSystemWrite
        )
    }
}

/// A compact set of granted capabilities.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CapabilitySet {
    bits: u32,
}

impl CapabilitySet {
    pub const fn empty() -> Self {
        Self { bits: 0 }
    }

    /// Every capability — used by trusted policies only.
    pub const fn all() -> Self {
        Self {
            bits: (1u32 << (Capability::ALL.len() as u32)) - 1,
        }
    }

    /// Capabilities an untrusted program may use. Filesystem access is granted
    /// here but confined by `enforce_path`, so it cannot escape the sandbox.
    pub const fn safe_default() -> Self {
        Self {
            bits: Capability::Pure.bit()
                | Capability::Io.bit()
                | Capability::FileSystemRead.bit()
                | Capability::FileSystemWrite.bit(),
        }
    }

    pub fn allow(mut self, capability: Capability) -> Self {
        self.bits |= capability.bit();
        self
    }

    pub fn deny(mut self, capability: Capability) -> Self {
        self.bits &= !capability.bit();
        self
    }

    pub const fn allows(&self, capability: Capability) -> bool {
        self.bits & capability.bit() != 0
    }
}

/// Static classification rules. Longer, more specific prefixes come first so
/// that e.g. `File_writeFile` is tagged as a write rather than a generic read.
const CLASSIFICATION_RULES: &[(&[&str], Capability)] = &[
    (&["Ctypes_", "Struct_", "Mmap_"], Capability::Ffi),
    (
        &["Subprocess_", "Process_", "Multiprocessing_"],
        Capability::Process,
    ),
    (&["Sys_exit", "Os_", "Platform_"], Capability::SystemControl),
    (&["Signal_"], Capability::Signals),
    (&["WinReg_"], Capability::Registry),
    (
        &["Net_", "Http_", "Socket_", "UdpSocket_", "Ssl_", "Ftp_"],
        Capability::Network,
    ),
    (
        &[
            "File_write",
            "File_append",
            "File_delete",
            "Dir_create",
            "Dir_delete",
            "TempFile_",
        ],
        Capability::FileSystemWrite,
    ),
    (&["File_", "Dir_", "Path_"], Capability::FileSystemRead),
    (&["Sqlite_"], Capability::Database),
    (&["Graphics_", "Window_", "Render_"], Capability::Graphics),
    (&["Audio_", "Sound_"], Capability::Audio),
    (&["Thread_"], Capability::Threading),
    (&["Env_", "Environment_"], Capability::Environment),
];

/// Classify a native name using the built-in rules.
pub fn classify_native(name: &str) -> Capability {
    for (prefixes, capability) in CLASSIFICATION_RULES {
        if prefixes.iter().any(|prefix| name.starts_with(prefix)) {
            return *capability;
        }
    }
    Capability::Pure
}

/// Per-VM registry: built-in rules plus explicit host overrides.
#[derive(Clone, Debug, Default)]
pub struct CapabilityRegistry {
    overrides: HashMap<String, Capability>,
}

impl CapabilityRegistry {
    pub fn new() -> Self {
        Self {
            overrides: HashMap::new(),
        }
    }

    /// Bind a native name to an explicit capability, overriding the built-in
    /// rule. Registering host-added natives prevents them from falling through
    /// to `Capability::Pure`.
    pub fn register(&mut self, native_name: impl Into<String>, capability: Capability) {
        self.overrides.insert(native_name.into(), capability);
    }

    pub fn capability_for(&self, name: &str) -> Capability {
        self.overrides
            .get(name)
            .copied()
            .unwrap_or_else(|| classify_native(name))
    }
}
