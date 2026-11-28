//! Append-only audit log for capability decisions.

use super::capability::Capability;
use super::diagnostic::{write_json_string, NativeCallSite};

/// Outcome of one native dispatch decision.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuditDecision {
    Allowed,
    Denied,
}

impl AuditDecision {
    pub const fn as_str(self) -> &'static str {
        match self {
            AuditDecision::Allowed => "allow",
            AuditDecision::Denied => "deny",
        }
    }
}

/// One recorded decision. `sequence` is assigned by [`AuditLog::record`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuditEvent {
    pub sequence: u64,
    pub decision: AuditDecision,
    pub call_site: NativeCallSite,
    pub capability: Capability,
    pub detail: String,
}

impl AuditEvent {
    pub(crate) fn allowed(call_site: NativeCallSite, capability: Capability) -> Self {
        Self {
            sequence: 0,
            decision: AuditDecision::Allowed,
            call_site,
            capability,
            detail: String::new(),
        }
    }

    pub(crate) fn denied(
        call_site: NativeCallSite,
        capability: Capability,
        detail: String,
    ) -> Self {
        Self {
            sequence: 0,
            decision: AuditDecision::Denied,
            call_site,
            capability,
            detail,
        }
    }

    pub fn to_json(&self) -> String {
        let mut out = String::new();
        out.push('{');
        out.push_str("\"sequence\":");
        out.push_str(&self.sequence.to_string());
        out.push_str(",\"decision\":");
        write_json_string(&mut out, self.decision.as_str());
        out.push_str(",\"capability\":");
        write_json_string(&mut out, self.capability.name());
        out.push_str(",\"call_site\":");
        write_json_string(&mut out, &self.call_site.to_string());
        out.push_str(",\"detail\":");
        write_json_string(&mut out, &self.detail);
        out.push('}');
        out
    }
}

/// In-memory audit trail. One instance lives with each `SecurityEnforcer`.
#[derive(Clone, Debug, Default)]
pub struct AuditLog {
    events: Vec<AuditEvent>,
    next_sequence: u64,
}

impl AuditLog {
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            next_sequence: 0,
        }
    }

    pub fn record(&mut self, mut event: AuditEvent) {
        event.sequence = self.next_sequence;
        self.next_sequence = self.next_sequence.wrapping_add(1);
        self.events.push(event);
    }

    pub fn events(&self) -> &[AuditEvent] {
        &self.events
    }

    pub fn allowed_count(&self) -> usize {
        self.events
            .iter()
            .filter(|event| event.decision == AuditDecision::Allowed)
            .count()
    }

    pub fn denied_count(&self) -> usize {
        self.events
            .iter()
            .filter(|event| event.decision == AuditDecision::Denied)
            .count()
    }

    pub fn to_json_array(&self) -> String {
        let mut out = String::with_capacity(32 * self.events.len());
        out.push('[');
        for (index, event) in self.events.iter().enumerate() {
            if index > 0 {
                out.push(',');
            }
            event_to_json_into(event, &mut out);
        }
        out.push(']');
        out
    }
}

fn event_to_json_into(event: &AuditEvent, out: &mut String) {
    let json = event.to_json();
    out.push_str(&json);
}
