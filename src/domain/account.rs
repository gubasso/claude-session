//! Pure account metadata, selection, and report values.

use crate::domain::{config::Source, identifier::Identifier, secret::Fingerprint};

/// Authentication modes recognized in durable metadata.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum AuthMode {
    Login,
    Token,
}

impl AuthMode {
    pub(crate) const fn spelling(self) -> &'static str {
        match self {
            Self::Login => "login",
            Self::Token => "token",
        }
    }

    /// Widens a durable mode into the report vocabulary.
    pub(crate) const fn report(self) -> ReportMode {
        match self {
            Self::Login => ReportMode::Login,
            Self::Token => ReportMode::Token,
        }
    }
}

/// The subscription plan a token belongs to, as the user declared it.
///
/// A declaration and never a reading. An injected token is opaque to the
/// wrapper by decision ([ADR-0026]), the child reaches its own answer only from
/// a saved credential that token mode replaces, and no child surface reports
/// the plan back. So this is what the user said, recorded next to the
/// credential it describes and labelled as a declaration wherever it is shown
/// ([ADR-0099]).
///
/// The value is validated for shape and never against a vocabulary. The shape
/// is narrower than an environment value strictly has to be, and deliberately:
/// a mistyped answer is worth refusing where a person can see it rather than
/// passing on as a value the child silently ignores. Which spellings mean
/// something is still the child's, not this type's, so a plan nobody here has
/// heard of is recorded and injected unchanged
/// ([accounts](../../docs/reference/accounts.md#declared-subscription-plan)).
///
/// [ADR-0026]: ../../docs/decisions/ADR-0026-store-and-inject-a-long-lived-subscription-token.md
/// [ADR-0099]: ../../docs/decisions/ADR-0099-declare-the-plan-a-token-cannot-carry.md
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(try_from = "String", into = "String")]
pub(crate) struct Plan(String);

impl Plan {
    /// The longest declaration accepted, well past any plan name.
    const LIMIT: usize = 32;

    /// The spellings the child acts on today, offered by the login prompt.
    ///
    /// An offer rather than a vocabulary: a declaration outside this list is
    /// recorded and injected unchanged, because the wrapper is not the party
    /// that decides which plans exist. Carried against the launch obligation
    /// and registered in `docs/reference/child-facts.yaml`, whose entry names
    /// the release these four were read from.
    pub(crate) const OFFERED: [&'static str; 4] = ["max", "pro", "team", "enterprise"];

    /// Validates one declaration, normalizing case.
    ///
    /// Lowercasing is about this input rather than about the child: the
    /// wrapper is typing an environment value on the user's behalf, and a
    /// person answering a prompt with `Max` meant the same thing as `max`.
    /// Nothing else is adjusted, so a spelling the child does not act on
    /// survives to be injected and to be visible in what the child then says.
    pub(crate) fn parse(value: &str) -> Result<Self, String> {
        let value = value.trim().to_ascii_lowercase();
        if value.is_empty() {
            return Err("a plan cannot be empty".into());
        }
        if value.len() > Self::LIMIT {
            return Err(format!("a plan is at most {} characters", Self::LIMIT));
        }
        if !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
        {
            return Err(
                "a plan is letters, digits, hyphens, and underscores, and nothing else".into(),
            );
        }
        Ok(Self(value))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for Plan {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(&value)
    }
}

impl From<Plan> for String {
    fn from(value: Plan) -> Self {
        value.0
    }
}

/// Strict required metadata fields, plus the token mode's own.
///
/// `fingerprint` is absent in login mode, where there is no wrapper-owned
/// secret to describe, and present in token mode, where it is what makes a
/// crash between the pair's two renames detectable. It is optional in the type
/// rather than in the contract: [`AuthModeMetadata::token_fingerprint`] is the
/// reader that narrows it, so the absent-in-token-mode case is refused once
/// instead of becoming a third state every report has to carry.
///
/// `plan` is absent in login mode for a stronger reason than convention: the
/// child reads its own plan from the credential it saved there, so a wrapper
/// declaration would be a second answer to a question already answered. In
/// token mode it is absent whenever the login did not obtain one, which is
/// every account recorded before [ADR-0099] and every `--stdin` login that
/// passed no `--plan`.
///
/// [ADR-0099]: ../../docs/decisions/ADR-0099-declare-the-plan-a-token-cannot-carry.md
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub(crate) struct AuthModeMetadata {
    pub(crate) mode: AuthMode,
    pub(crate) recorded_at: RecordedAt,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) fingerprint: Option<Fingerprint>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) plan: Option<Plan>,
}

impl AuthModeMetadata {
    /// Returns the recorded fingerprint of a token account.
    ///
    /// `None` means the metadata is malformed rather than that the account has
    /// no fingerprint, so a caller reports it as unusable rather than
    /// substituting a default.
    pub(crate) const fn token_fingerprint(&self) -> Option<&Fingerprint> {
        match self.mode {
            AuthMode::Token => self.fingerprint.as_ref(),
            AuthMode::Login => None,
        }
    }

    /// Returns the plan a token account declared.
    ///
    /// Narrowed to token mode by the same reasoning as the reader above: a
    /// value under a login-mode account answers nothing, so no caller has to
    /// decide what to do with one.
    pub(crate) const fn declared_plan(&self) -> Option<&Plan> {
        match self.mode {
            AuthMode::Token => self.plan.as_ref(),
            AuthMode::Login => None,
        }
    }
}

/// One account's durable profile binding.
///
/// The whole record is the profile it runs with and when that was recorded.
/// Nothing about the profile's content lives here: this names a profile, and
/// the profile document owns everything else about it
/// ([ADR-0096](../../docs/decisions/ADR-0096-bind-a-profile-to-an-account.md)).
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProfileBinding {
    pub(crate) profile: Identifier,
    pub(crate) recorded_at: RecordedAt,
}

/// Where a login reads the secret it was given.
///
/// The two sources are the whole of what the ingest rule permits: a controlling
/// terminal, or standard input. Argv, an environment variable, a file flag, and
/// scraped child output are all absent by design rather than unimplemented.
/// Shared by both secret-bearing logins, because [ADR-0027] governs the entry
/// of a credential rather than of one particular credential.
///
/// [ADR-0027]: ../../docs/decisions/ADR-0027-ingest-secrets-only-from-stdin-or-a-terminal.md
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TokenSource {
    Terminal,
    Stdin,
}

/// A token-mode login's ingest request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TokenIngest {
    pub(crate) source: TokenSource,
    /// The mint time, when the token was not minted during this run.
    ///
    /// Absent means the ingest time is the mint time, which is right for a
    /// token this run just asked the child to produce and wrong for one pasted
    /// from a password manager months later.
    pub(crate) minted_at: Option<RecordedAt>,
    /// The plan the command line declared, when it declared one.
    ///
    /// Absent means the login has not obtained one yet rather than that there
    /// is none: a terminal login asks, and only a login with nowhere to ask
    /// records an account without a plan. Carried on the token request rather
    /// than beside it, so a plan without a token cannot be expressed.
    pub(crate) plan: Option<Plan>,
}

/// The scopes a refresh token was issued with, as the user supplied them.
///
/// Carried because the child requires them beside the refresh token and refuses
/// the exchange without them: it forwards the string to the token endpoint, and
/// asking for a scope the grant does not hold fails the whole login. The
/// wrapper neither issues the token nor learns what it holds, so this is
/// declared for the same reason [`Plan`] is
/// ([ADR-0100](../../docs/decisions/ADR-0100-bootstrap-a-saved-login-from-a-refresh-token.md)).
///
/// Validated for shape and never against a vocabulary. The shape is RFC 6749's
/// `scope-token` — printable ASCII without space, quote, or backslash —
/// because which scopes exist belongs to the issuer, and a set this type has
/// never heard of has to survive to reach the child.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Scopes(String);

impl Scopes {
    /// The longest scope string accepted, well past any issued set.
    const LIMIT: usize = 512;

    /// The scopes a claude.ai login is issued with, used when none is given.
    ///
    /// A default rather than a vocabulary, and the one place the wrapper
    /// guesses on the user's behalf: nearly every refresh token comes from such
    /// a login, and requiring the set to be typed would make the common case
    /// depend on knowing an OAuth detail. A token issued with a different set
    /// passes `--scopes`. Read from child 2.1.220, registered against the
    /// launch obligation in `docs/reference/child-facts.yaml`, and tracked as
    /// perishable in `docs/reference/research-tracking.yaml`, because the set
    /// is the child's to change and is undocumented as a default.
    ///
    /// `org:create_api_key` leads it because the child's own authorize URL
    /// leads with it, and this list is a transcription rather than a judgement
    /// about which scopes a wrapper would want. It was missing until an actual
    /// login was read on 2026-08-14: the first version of this constant was
    /// taken from the child's source rather than from a flow, and it had the
    /// five that looked like Claude Code's own.
    pub(crate) const DEFAULT: [&'static str; 6] = [
        "org:create_api_key",
        "user:profile",
        "user:inference",
        "user:sessions:claude_code",
        "user:mcp_servers",
        "user:file_upload",
    ];

    /// Validates one scope string, normalizing the separators.
    ///
    /// Whitespace is collapsed to the single spaces the child splits on, for
    /// the reason [`Plan::parse`] lowercases: the wrapper is typing an
    /// environment value on the user's behalf, and a set pasted across two
    /// lines meant the same set. Nothing inside a scope is touched, and case is
    /// left alone — the value is forwarded to an endpoint this wrapper does not
    /// speak and cannot second-guess.
    pub(crate) fn parse(value: &str) -> Result<Self, String> {
        if value.len() > Self::LIMIT {
            return Err(format!("a scope set is at most {} characters", Self::LIMIT));
        }
        let scopes: Vec<&str> = value.split_whitespace().collect();
        if scopes.is_empty() {
            return Err("a scope set cannot be empty".into());
        }
        if !scopes.iter().all(|scope| {
            scope
                .bytes()
                .all(|byte| byte.is_ascii_graphic() && byte != b'"' && byte != b'\\')
        }) {
            return Err(
                "a scope is printable ASCII without a quote or a backslash (RFC 6749)".into(),
            );
        }
        Ok(Self(scopes.join(" ")))
    }

    /// The scope set a login uses when the command line named none.
    pub(crate) fn issued_by_a_login() -> Self {
        Self(Self::DEFAULT.join(" "))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

/// A refresh-token login's ingest request.
///
/// No mint time and no plan, unlike [`TokenIngest`]: the child exchanges this
/// credential for one it saves and describes itself, so both questions token
/// mode has to ask are answered by the exchange.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RefreshIngest {
    pub(crate) source: TokenSource,
    pub(crate) scopes: Scopes,
}

/// Which of the three logins one `account login` invocation asked for.
///
/// An enum rather than a pair of options, so "both a token and a refresh token"
/// cannot be expressed past the parser and no arm has to decide what it would
/// mean.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum LoginRequest {
    /// The child's own interactive browser flow.
    Native,
    /// A long-lived subscription token this wrapper stores.
    Token(TokenIngest),
    /// A refresh token the child exchanges for a saved login.
    Refresh(RefreshIngest),
}

/// A validated RFC 3339 UTC timestamp.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(try_from = "String", into = "String")]
pub(crate) struct RecordedAt(String);

impl RecordedAt {
    pub(crate) fn parse(value: String) -> Result<Self, String> {
        let bytes = value.as_bytes();
        let shape = bytes.len() == 20
            && bytes[4] == b'-'
            && bytes[7] == b'-'
            && bytes[10] == b'T'
            && bytes[13] == b':'
            && bytes[16] == b':'
            && bytes[19] == b'Z'
            && bytes.iter().enumerate().all(|(index, byte)| {
                matches!(index, 4 | 7 | 10 | 13 | 16 | 19) || byte.is_ascii_digit()
            });
        if !shape {
            return Err("recorded_at must be RFC 3339 UTC seconds".into());
        }
        let number = |range: std::ops::Range<usize>| {
            std::str::from_utf8(&bytes[range]).ok()?.parse::<u32>().ok()
        };
        let (Some(year), Some(month), Some(day), Some(hour), Some(minute), Some(second)) = (
            number(0..4),
            number(5..7),
            number(8..10),
            number(11..13),
            number(14..16),
            number(17..19),
        ) else {
            return Err("recorded_at contains an invalid number".into());
        };
        let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
        let month_days = match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 if leap => 29,
            2 => 28,
            _ => 0,
        };
        if day == 0 || day > month_days || hour > 23 || minute > 59 || second > 59 {
            return Err("recorded_at contains an out-of-range component".into());
        }
        Ok(Self(value))
    }

    pub(crate) const fn new_unchecked(value: String) -> Self {
        Self(value)
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for RecordedAt {
    type Error = String;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl From<RecordedAt> for String {
    fn from(value: RecordedAt) -> Self {
        value.0
    }
}

/// The public account-selection provenance vocabulary.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize)]
pub(crate) enum SelectionSource {
    Flag,
    Environment,
    UserConfig,
    Marker,
    None,
}

impl SelectionSource {
    pub(crate) const fn spelling(self) -> &'static str {
        match self {
            Self::Flag => "flag",
            Self::Environment => "environment",
            Self::UserConfig => "user-config",
            Self::Marker => "marker",
            Self::None => "none",
        }
    }
}

impl From<Source> for SelectionSource {
    fn from(source: Source) -> Self {
        match source {
            Source::Cli => Self::Flag,
            Source::Environment => Self::Environment,
            Source::User => Self::UserConfig,
            // Neither layer can supply an account: the project layer is
            // restricted to the profile key, and the account binding names a
            // profile rather than an account.
            Source::Default | Source::Project | Source::Account => Self::None,
        }
    }
}

/// One immutable selection resolved before application context construction.
#[derive(Clone, Debug)]
pub(crate) struct AccountSelection {
    account: Option<Identifier>,
    source: SelectionSource,
}

impl AccountSelection {
    pub(crate) const fn none() -> Self {
        Self {
            account: None,
            source: SelectionSource::None,
        }
    }
    pub(crate) const fn new(account: Identifier, source: SelectionSource) -> Self {
        Self {
            account: Some(account),
            source,
        }
    }
    pub(crate) const fn account(&self) -> Option<&Identifier> {
        self.account.as_ref()
    }
    pub(crate) const fn source(&self) -> SelectionSource {
        self.source
    }
}

/// List-only mode projection. `Invalid` is never durable metadata.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ReportMode {
    Login,
    Token,
    Invalid,
}

impl ReportMode {
    pub(crate) const fn spelling(self) -> &'static str {
        match self {
            Self::Login => "login",
            Self::Token => "token",
            Self::Invalid => "invalid",
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct AccountFinding {
    pub(crate) name: Identifier,
    pub(crate) mode: ReportMode,
    pub(crate) usable: bool,
    pub(crate) selected: bool,
    /// The profile this account is bound to, absent when it has none.
    ///
    /// Local state alone, like every other field here: reading the binding is
    /// one file per account and never a child.
    pub(crate) profile: Option<Identifier>,
}

/// A credential precedence condition the wrapper reports and never acts on.
///
/// One type for three surfaces — the pre-launch line on standard error, the
/// `warnings` array in `account status`, and the mode-aware `doctor` results —
/// so a condition is worded once. Every variant describes something that
/// outranks or shadows the selected account's stored credential; none of them
/// changes it, because ambient authentication is the user's and a stored mode
/// is only ever changed by `account login`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Warning {
    /// An ambient mechanism outranks the selected subscription account.
    Ambient(AmbientCredential),
    /// A stored token shadows a saved login that also exists.
    TokenOverLogin,
    /// The child configuration this terminal would launch with cannot be read.
    ///
    /// Not "the first run is unrecorded": since [ADR-0105] moved the seed to
    /// launch, an unrecorded key is repaired on the way past and is nothing to
    /// warn about. What survives is the file the launch cannot read.
    ///
    /// Carries no name for the reason the one below carries none: every surface
    /// raising it has already said which account it is about.
    ///
    /// [ADR-0105]: ../../docs/decisions/ADR-0105-seed-a-session-at-launch.md
    FirstRunOnboarding,
    /// A token account declared no subscription plan.
    ///
    /// Carries no name for the reason the one above carries none, and applies
    /// to token mode alone: a saved login tells the child its own plan.
    PlanUndeclared,
    /// The account is bound to a profile that has no document.
    ///
    /// Carries no name, because the report already states which profile the
    /// account is bound to and a warning that repeated it would be the same
    /// fact twice.
    BoundProfileMissing,
}

impl Warning {
    /// Returns the one wording every surface uses.
    pub(crate) fn message(self) -> String {
        match self {
            Self::Ambient(credential) => format!(
                concat!(
                    "{} outranks the selected account, so the child will",
                    " authenticate with it; the stored mode is unchanged"
                ),
                credential.spelling()
            ),
            Self::TokenOverLogin => concat!(
                "the stored token outranks the saved login in this account's",
                " configuration directory, so the saved login is not used"
            )
            .to_owned(),
            Self::FirstRunOnboarding => concat!(
                "the claude configuration file this terminal would launch with could not",
                " be read, so claude will meet its first-run setup and ask to sign in",
                " again; move that file aside and the next launch writes a fresh one"
            )
            .to_owned(),
            Self::PlanUndeclared => concat!(
                "this account declared no subscription plan, so claude cannot tell",
                " which one the stored token belongs to; it will describe the session",
                " as an API one and pick the model it defaults to without a plan. Run",
                " claude-session account login for this account with --token and",
                " --plan to declare it"
            )
            .to_owned(),
            Self::BoundProfileMissing => concat!(
                "the profile this account is bound to has no document, so a launch",
                " under this account refuses until the profile exists or the",
                " account is bound to one that does"
            )
            .to_owned(),
        }
    }
}

/// A child credential mechanism that outranks a selected subscription account.
///
/// Membership is the whole carry, and it is here so the wrapper can say its own
/// account selection was defeated (ADR-0089). Which mechanism the child would
/// prefer among these is the child's to arbitrate, so no order is asserted.
/// None is ever wrapper-managed and none is ever stripped from a launch.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum AmbientCredential {
    AuthToken,
    ApiKey,
    Bedrock,
    Vertex,
    Foundry,
}

impl AmbientCredential {
    /// The environment variable that reveals this mechanism, which is also the
    /// only name any surface prints for it.
    pub(crate) const fn spelling(self) -> &'static str {
        match self {
            Self::AuthToken => "ANTHROPIC_AUTH_TOKEN",
            Self::ApiKey => "ANTHROPIC_API_KEY",
            Self::Bedrock => "CLAUDE_CODE_USE_BEDROCK",
            Self::Vertex => "CLAUDE_CODE_USE_VERTEX",
            Self::Foundry => "CLAUDE_CODE_USE_FOUNDRY",
        }
    }

    /// Every mechanism, in declaration order, which is a reading order and not
    /// a ranking.
    pub(crate) const ALL: [Self; 5] = [
        Self::AuthToken,
        Self::ApiKey,
        Self::Bedrock,
        Self::Vertex,
        Self::Foundry,
    ];
}

/// Whether the child answered, and how.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ProbeStatus {
    Ok,
    Failed,
    Unavailable,
}

impl ProbeStatus {
    pub(crate) const fn spelling(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Failed => "failed",
            Self::Unavailable => "unavailable",
        }
    }
}

/// One child probe's reportable result.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Probe {
    pub(crate) status: ProbeStatus,
    /// Present only when a child actually ran.
    pub(crate) exit_code: Option<u8>,
}

/// One account's full status projection.
///
/// The five always-present fields answer "which account, and can it be used".
/// Everything else is present only where it applies, and absent rather than
/// null where it does not, which is the same document rule every report
/// follows. `age_seconds` and `estimated_expiry` are additionally withheld when
/// `metadata_consistent` is false, because computing them from a mint time that
/// is not the token's would be inventing a fact.
#[derive(Clone, Debug)]
pub(crate) struct AccountStatus {
    pub(crate) account: Identifier,
    pub(crate) selected: bool,
    pub(crate) mode: ReportMode,
    pub(crate) usable: bool,
    pub(crate) warnings: Vec<Warning>,
    pub(crate) selection_source: Option<SelectionSource>,
    pub(crate) recorded_at: Option<RecordedAt>,
    pub(crate) age_seconds: Option<u64>,
    pub(crate) estimated_expiry: Option<String>,
    pub(crate) fingerprint: Option<Fingerprint>,
    pub(crate) metadata_consistent: Option<bool>,
    pub(crate) child_login_present: Option<bool>,
    pub(crate) child_probe: Option<Probe>,
    /// The profile this account is bound to.
    pub(crate) profile: Option<Identifier>,
    /// Whether that profile has a document.
    ///
    /// Absent when the account is unbound, where there is no profile for the
    /// question to be about.
    pub(crate) profile_present: Option<bool>,
    /// Which layer supplied the profile this run would launch under.
    ///
    /// Present only for the selected account, for the reason
    /// `selection_source` is: an account nobody selected is not the subject of
    /// this run's profile resolution.
    pub(crate) profile_source: Option<Source>,
    /// The profile this run would launch under, which is not always the bound
    /// one: a flag, the environment, and a project file each outrank it.
    ///
    /// Paired with `profile_source` and present under the same condition. The
    /// human report needs it to name the profile that wins rather than only
    /// asserting that a different one would; the machine document carries the
    /// binding and the layer, and its field set does not move here.
    pub(crate) effective_profile: Option<Identifier>,
}

/// What the commit that records one account's authentication mode did.
///
/// The retirement rides here rather than in the metadata because it is about
/// the mode being replaced rather than the one being recorded, and nothing
/// durable should remember it: the artifact is gone, so the fact has exactly
/// one reader, which is the report of the login that did it.
#[derive(Clone, Debug)]
pub(crate) struct ModeCommit {
    pub(crate) metadata: AuthModeMetadata,
    /// Whether this commit unlinked the artifact the replaced mode used.
    pub(crate) retired: bool,
}

/// What one removal did.
#[derive(Clone, Debug)]
pub(crate) struct Removal {
    pub(crate) account: Identifier,
    pub(crate) path: std::path::PathBuf,
    pub(crate) removed: bool,
    pub(crate) mode: Option<ReportMode>,
    pub(crate) marker_cleared: Option<bool>,
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn token_metadata_round_trips_its_fingerprint() {
        let document =
            r#"{"mode":"token","recorded_at":"2026-08-11T12:34:56Z","fingerprint":"3c469e9d"}"#;
        let value: AuthModeMetadata = serde_json::from_str(document).expect("token metadata");
        assert_eq!(value.mode, AuthMode::Token);
        assert_eq!(
            value.token_fingerprint().map(Fingerprint::as_str),
            Some("3c469e9d")
        );
        assert_eq!(
            serde_json::to_string(&value).expect("token metadata renders"),
            document
        );
    }

    /// Login metadata carries neither optional key at all rather than a null,
    /// which is the same absent-not-null rule every report document follows.
    #[test]
    fn login_metadata_omits_the_fingerprint_key() {
        let value = AuthModeMetadata {
            mode: AuthMode::Login,
            recorded_at: RecordedAt::new_unchecked("2026-08-11T12:34:56Z".into()),
            fingerprint: None,
            plan: None,
        };
        assert_eq!(
            serde_json::to_string(&value).expect("login metadata renders"),
            r#"{"mode":"login","recorded_at":"2026-08-11T12:34:56Z"}"#
        );
        assert!(value.token_fingerprint().is_none());
        assert!(value.declared_plan().is_none());
    }

    /// A plan under a login-mode account is not the launch's to read: the child
    /// answers the question from the credential it saved for itself there.
    #[test]
    fn a_login_mode_plan_is_not_reported() {
        let value = AuthModeMetadata {
            mode: AuthMode::Login,
            recorded_at: RecordedAt::new_unchecked("2026-08-11T12:34:56Z".into()),
            fingerprint: None,
            plan: Some(Plan::parse("max").expect("a plan")),
        };
        assert!(value.declared_plan().is_none());
    }

    /// Shape alone, and case normalized, because the wrapper is typing an
    /// environment value rather than judging which plans exist.
    #[test]
    fn a_plan_is_validated_for_shape_and_lowercased() {
        assert_eq!(
            Plan::parse("  Max \n").expect("a plan").as_str(),
            "max",
            "a trimmed, lowercased declaration"
        );
        assert_eq!(
            Plan::parse("enterprise_usage_based")
                .expect("a plan")
                .as_str(),
            "enterprise_usage_based",
            "a spelling the offer does not list is still recorded"
        );
        for refused in ["", "   ", "max plan", "max=1", &"m".repeat(33)] {
            assert!(
                Plan::parse(refused).is_err(),
                "{refused:?} is not usable as an environment value"
            );
        }
    }

    /// The declared plan rides in the same document as the fingerprint, so a
    /// rotation that does not re-declare one clears it.
    #[test]
    fn token_metadata_round_trips_the_declared_plan() {
        let document = concat!(
            r#"{"mode":"token","recorded_at":"2026-08-11T12:34:56Z","#,
            r#""fingerprint":"3c469e9d","plan":"max"}"#
        );
        let value: AuthModeMetadata = serde_json::from_str(document).expect("token metadata");
        assert_eq!(value.declared_plan().map(Plan::as_str), Some("max"));
        assert_eq!(
            serde_json::to_string(&value).expect("token metadata renders"),
            document
        );
    }

    /// Metadata written before the plan existed still reads, as an account that
    /// declared none rather than as a defect.
    #[test]
    fn token_metadata_without_a_plan_reads_as_undeclared() {
        let document = concat!(
            r#"{"mode":"token","recorded_at":"2026-08-11T12:34:56Z","#,
            r#""fingerprint":"3c469e9d"}"#
        );
        let value: AuthModeMetadata = serde_json::from_str(document).expect("token metadata");
        assert!(value.declared_plan().is_none());
    }

    #[test]
    fn malformed_metadata_is_refused() {
        for document in [
            r#"{"mode":"login"}"#,
            r#"{"mode":"other","recorded_at":"2026-08-11T12:34:56Z"}"#,
            r#"{"mode":"token","recorded_at":"2026-08-11T12:34:56Z","fingerprint":"NOTHEX"}"#,
        ] {
            assert!(
                serde_json::from_str::<AuthModeMetadata>(document).is_err(),
                "{document} must be refused"
            );
        }
    }

    /// A token document without a fingerprint parses, because the field is
    /// optional in the type, and is then refused by the reader that narrows it.
    #[test]
    fn a_token_document_without_a_fingerprint_has_none_to_report() {
        let value: AuthModeMetadata =
            serde_json::from_str(r#"{"mode":"token","recorded_at":"2026-08-11T12:34:56Z"}"#)
                .expect("the field is optional in the type");
        assert!(value.token_fingerprint().is_none());
    }

    #[test]
    fn invalid_is_the_pinned_list_only_spelling() {
        assert_eq!(ReportMode::Invalid.spelling(), "invalid");
    }

    #[test]
    fn sources_have_only_the_public_account_spellings() {
        assert_eq!(SelectionSource::Flag.spelling(), "flag");
        assert_eq!(SelectionSource::Environment.spelling(), "environment");
        assert_eq!(SelectionSource::UserConfig.spelling(), "user-config");
        assert_eq!(SelectionSource::Marker.spelling(), "marker");
        assert_eq!(SelectionSource::None.spelling(), "none");
        assert_eq!(
            SelectionSource::from(Source::Project),
            SelectionSource::None
        );
    }
}
