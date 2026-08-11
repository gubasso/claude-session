//! The wrapper's configuration file type and its reflectable description.
//!
//! The description lives beside the type it describes and is the source the
//! generated artifacts render from, never a parallel document that can rot
//! ([ADR-0013](../../docs/decisions/ADR-0013-generate-config-examples-from-types.md)).
//! `every_configuration_field_has_a_described_key` is what makes that a
//! mechanism rather than a convention.

// The items below are `pub` so the library target can export them to `xtask`,
// which trips `unreachable_pub` inside the binary target that also compiles
// this file. Both targets are real; only one of them can reach these.
#![allow(
    unreachable_pub,
    reason = "these items are reachable through the library target"
)]

use std::path::PathBuf;

use serde::Deserialize;

/// The wrapper's configuration file, decoded strictly.
///
/// Read by the loader in the binary target. The library target compiles this
/// file for the descriptor below and constructs nothing.
#[allow(dead_code, reason = "the loader lives in the binary target")]
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct FileConfig {
    pub(crate) child_bin: Option<PathBuf>,
    pub(crate) default_account: Option<String>,
    pub(crate) default_profile: Option<String>,
}

/// One configuration key, as the generated artifacts render it.
///
/// The binary compiles this file for `FileConfig` and reads nothing else here;
/// the descriptor is the library target's face, which `xtask` renders from.
#[allow(
    dead_code,
    reason = "the descriptor is read through the library target"
)]
pub struct ConfigKey {
    /// The key's file spelling.
    pub name: &'static str,
    /// The JSON Schema type the value takes.
    pub type_name: &'static str,
    /// Whether a valid file must set it.
    pub required: bool,
    /// What the key means, taken from `configuration.md#keys`.
    pub description: &'static str,
    /// An obviously fake value, so a copied example cannot be used as-is.
    pub placeholder: &'static str,
}

/// One field of the profile document, as the generated example renders it.
///
/// The example's table row claims it is rendered from the profile type, so the
/// field names come from a descriptor beside that type rather than from a
/// hand-written sample that a rename would silently falsify.
#[allow(
    dead_code,
    reason = "the descriptor is read through the library target"
)]
pub struct ProfileField {
    /// The field's YAML spelling.
    pub name: &'static str,
    /// What the field means.
    pub description: &'static str,
    /// The rendered body, already indented for its nesting.
    pub body: &'static [&'static str],
}

/// Every field of the profile document, in file order.
#[allow(
    dead_code,
    reason = "the descriptor is read through the library target"
)]
pub const PROFILE_FIELDS: &[ProfileField] = &[
    ProfileField {
        name: "layers",
        description: "The ordered piece names, resolved under the settings directory. Later wins.",
        body: &["  - replace-me", "  - replace-me-too"],
    },
    ProfileField {
        name: "array_strategies",
        description: concat!(
            "Per-key exceptions to the default array rule, which is replace. ",
            "Keyed by RFC 6901 JSON Pointer, so a settings key containing a dot ",
            "stays addressable. Omit this table entirely to take replace everywhere.",
        ),
        body: &[
            "  # Elements appended in layer order.",
            "  \"/permissions/allow\": { strategy: concat }",
            "  # Elements matched on the named field and merged; unmatched ones appended.",
            "  \"/hooks/PreToolUse\": { strategy: merge-by-key, key: matcher }",
        ],
    },
];

/// Every key of the wrapper's configuration file, in file order.
///
/// Every key is optional, so an example whose keys are all commented out is a
/// minimal valid configuration and each commented key is independently usable.
#[allow(
    dead_code,
    reason = "the descriptor is read through the library target"
)]
pub const KEYS: &[ConfigKey] = &[
    ConfigKey {
        name: "child_bin",
        type_name: "string",
        required: false,
        description: "Absolute path to the claude binary to run. Unset searches PATH.",
        placeholder: "/path/to/claude",
    },
    ConfigKey {
        name: "default_account",
        type_name: "string",
        required: false,
        description: concat!(
            "Account to select when --account is absent. ",
            "Unset falls through to the last-used marker.",
        ),
        // Obviously fake and still a valid identifier: a placeholder the program
        // would reject makes an uncommented example fail for the wrong reason.
        placeholder: "replace-me",
    },
    ConfigKey {
        name: "default_profile",
        type_name: "string",
        required: false,
        description: "Profile to compose when --profile is absent. Unset composes nothing.",
        placeholder: "replace-me",
    },
];
