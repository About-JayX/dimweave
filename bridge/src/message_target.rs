//! Bridge-side mirror of the canonical `MessageTarget` wire contract.
//!
//! Must emit and accept the **exact same JSON shape** as
//! `src-tauri/src/daemon/message_target.rs`. Both crates share the wire but
//! compile independently, so this file is a byte-level mirror — if you edit
//! one, edit the other identically. Tests assert the canonical form.
//!
//! Wire form:
//! ```json
//! { "kind": "user" | "role" | "agent", "role": "<or ''>", "agentId": "<or ''>" }
//! ```

use serde::de::{self, MapAccess, Visitor};
use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageTarget {
    User,
    Role { role: String },
    Agent { agent_id: String },
}

impl Serialize for MessageTarget {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut s = serializer.serialize_struct("MessageTarget", 3)?;
        match self {
            MessageTarget::User => {
                s.serialize_field("kind", "user")?;
                s.serialize_field("role", "")?;
                s.serialize_field("agentId", "")?;
            }
            MessageTarget::Role { role } => {
                s.serialize_field("kind", "role")?;
                s.serialize_field("role", role)?;
                s.serialize_field("agentId", "")?;
            }
            MessageTarget::Agent { agent_id } => {
                s.serialize_field("kind", "agent")?;
                s.serialize_field("role", "")?;
                s.serialize_field("agentId", agent_id)?;
            }
        }
        s.end()
    }
}

impl<'de> Deserialize<'de> for MessageTarget {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_map(MessageTargetVisitor)
    }
}

struct MessageTargetVisitor;

impl<'de> Visitor<'de> for MessageTargetVisitor {
    type Value = MessageTarget;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter
            .write_str("a MessageTarget object with `kind` and optional `role`/`agentId`")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<MessageTarget, A::Error> {
        let mut kind: Option<String> = None;
        let mut role: Option<String> = None;
        let mut agent_id: Option<String> = None;

        while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
                "kind" => {
                    if kind.is_some() {
                        return Err(de::Error::duplicate_field("kind"));
                    }
                    kind = Some(map.next_value()?);
                }
                "role" => {
                    role = Some(map.next_value()?);
                }
                "agentId" => {
                    agent_id = Some(map.next_value()?);
                }
                _ => {
                    let _: serde::de::IgnoredAny = map.next_value()?;
                }
            }
        }

        let kind = kind.ok_or_else(|| de::Error::missing_field("kind"))?;

        match kind.as_str() {
            "user" => Ok(MessageTarget::User),
            "role" => {
                let role_val = role.unwrap_or_default();
                if role_val.trim().is_empty() {
                    return Err(de::Error::custom(
                        "MessageTarget with kind=\"role\" requires non-empty `role` field",
                    ));
                }
                Ok(MessageTarget::Role { role: role_val })
            }
            "agent" => {
                let id = agent_id.unwrap_or_default();
                if id.trim().is_empty() {
                    return Err(de::Error::custom(
                        "MessageTarget with kind=\"agent\" requires non-empty `agentId` field",
                    ));
                }
                Ok(MessageTarget::Agent { agent_id: id })
            }
            other => Err(de::Error::unknown_variant(
                other,
                &["user", "role", "agent"],
            )),
        }
    }
}

#[cfg(test)]
#[path = "message_target_tests.rs"]
mod tests;
