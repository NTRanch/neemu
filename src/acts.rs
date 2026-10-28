//! The ACT registry seed (SPEC §6) and the S2 vocabularies.
//!
//! Provenance discipline: every code below is grounded in a named
//! estate source record; nothing is invented in this file.
//!
//! - Act codes are the B-canon of `ntr-corpus-skeleton/BDSM-TYPES.md`
//!   (B-01..B-80), plus the three lab-local codes the W6 lab assigned
//!   to [ACT]-only catalogue entries that carry no B-code: `KISS`,
//!   `M-01`, `SOLO` (`adult-audio-research-20261026/TORORI-CATALOGUE.md`
//!   [ACT] entries; `night-20261026/w6/W6-SPEC.md` §1.1.1).
//! - W6 group / disposition / physics class per code are the W6
//!   lab's own classification (W6-SPEC §1.1.1 families, §1.1.2
//!   physics classes, §1.1.3 dispositions for the codes that are not
//!   standalone families).
//! - A stratum/primitive mapping is recorded ONLY where a source
//!   states it (ATLAS.md §4): B-48 -> S-18, M-01 -> S-17 + S-18,
//!   B-60 -> S-12. Every other entry's stratum mapping is honestly
//!   `unmapped` — an honest state, not a defect (SPEC §6).
//! - The seven [ACT]-only items that are neither families nor coded
//!   (layers / umbrella / foley, W6-SPEC §1.1.3) ride as uncoded
//!   candidates: present, labelled, never silently dropped.
//!
//! The registry is versioned and append-only in spirit: the seed is
//! data, extraction mints candidate records for codes it does not
//! know (see tools), and canon changes are Owner-ratified edits to
//! this table, never runtime mutations.

use serde_json::{json, Value};

/// Version stamp of this registry seed (SPEC §6: versioned file).
pub const ACT_REGISTRY_VERSION: &str = "act-registry/1 (seed: W6-SPEC §1 + B-canon)";

/// One coded registry entry.
pub struct ActEntry {
    pub code: &'static str,
    /// Behaviour-level name, from the source record.
    pub name: &'static str,
    /// W6 family group (PEN / ORAL / DEV / IMP / RST / CONTACT) for
    /// the 37 standalone families; `None` for non-family codes.
    pub group: Option<&'static str>,
    /// `family`, or the W6-SPEC §1.1.3 disposition for non-families.
    pub disposition: &'static str,
    /// W6 physics-applicability class (P-A / P-B / P-C) for families.
    pub physics_class: Option<&'static str>,
    /// Source-stated W6 strata (ATLAS.md §4); empty when none stated.
    pub strata: &'static [&'static str],
    /// Source-stated W6 primitives; empty when none stated.
    pub primitives: &'static [&'static str],
    /// The source records this entry is grounded in.
    pub source: &'static str,
}

const SRC_FAMILY: &str =
    "ntr-corpus-skeleton/BDSM-TYPES.md (B-canon); night-20261026/w6/W6-SPEC.md 1.1.1";
const SRC_DISPOSITION: &str =
    "ntr-corpus-skeleton/BDSM-TYPES.md (B-canon); night-20261026/w6/W6-SPEC.md 1.1.3";
const SRC_LAB_CODE: &str = "adult-audio-research-20261026/TORORI-CATALOGUE.md [ACT] (no B-code); night-20261026/w6/W6-SPEC.md 1.1.1 (lab-local code)";
const SRC_B48: &str = "ntr-corpus-skeleton/BDSM-TYPES.md (B-canon); night-20261026/w6/W6-SPEC.md 1.1.1; night-20261026/w6/ATLAS.md S-18 (B-48 adjacency stratum)";
const SRC_M01: &str = "adult-audio-research-20261026/TORORI-CATALOGUE.md [ACT] (no B-code); night-20261026/w6/W6-SPEC.md 1.1.1 (lab-local code); night-20261026/w6/ATLAS.md S-17/S-18 + LM row";
const SRC_B60: &str = "ntr-corpus-skeleton/BDSM-TYPES.md (B-canon); night-20261026/w6/W6-SPEC.md 1.1.3; night-20261026/w6/ATLAS.md S-12 (internal finish, B-60 form)";

/// The seed entries: 80 B-codes + KISS / M-01 / SOLO = 83.
pub const ACT_REGISTRY: &[ActEntry] = &[
    // ---- PEN — penetrative / insertion (W6-SPEC §1.1.1) ----
    ActEntry {
        code: "B-05",
        name: "DP dildos, pussy + mouth",
        group: Some("PEN"),
        disposition: "family",
        physics_class: Some("P-A"),
        strata: &[],
        primitives: &[],
        source: SRC_FAMILY,
    },
    ActEntry {
        code: "B-06",
        name: "DP dildos, pussy + ass",
        group: Some("PEN"),
        disposition: "family",
        physics_class: Some("P-A"),
        strata: &[],
        primitives: &[],
        source: SRC_FAMILY,
    },
    ActEntry {
        code: "B-07",
        name: "Triple dildo, pussy + mouth + ass",
        group: Some("PEN"),
        disposition: "family",
        physics_class: Some("P-A"),
        strata: &[],
        primitives: &[],
        source: SRC_FAMILY,
    },
    ActEntry {
        code: "B-16",
        name: "Missionary, silhouette partner",
        group: Some("PEN"),
        disposition: "family",
        physics_class: Some("P-A"),
        strata: &[],
        primitives: &[],
        source: SRC_FAMILY,
    },
    ActEntry {
        code: "B-17",
        name: "Doggy, silhouette partner",
        group: Some("PEN"),
        disposition: "family",
        physics_class: Some("P-A"),
        strata: &[],
        primitives: &[],
        source: SRC_FAMILY,
    },
    ActEntry {
        code: "B-18",
        name: "Cowgirl, silhouette partner",
        group: Some("PEN"),
        disposition: "family",
        physics_class: Some("P-A"),
        strata: &[],
        primitives: &[],
        source: SRC_FAMILY,
    },
    ActEntry {
        code: "B-19",
        name: "Anal, silhouette partner",
        group: Some("PEN"),
        disposition: "family",
        physics_class: Some("P-A"),
        strata: &[],
        primitives: &[],
        source: SRC_FAMILY,
    },
    ActEntry {
        code: "B-32",
        name: "Yuri",
        group: Some("PEN"),
        disposition: "family",
        physics_class: Some("P-A"),
        strata: &[],
        primitives: &[],
        source: SRC_FAMILY,
    },
    ActEntry {
        code: "B-36",
        name: "Large toys, swelling, giant orgasm",
        group: Some("PEN"),
        disposition: "family",
        physics_class: Some("P-A"),
        strata: &[],
        primitives: &[],
        source: SRC_FAMILY,
    },
    ActEntry {
        code: "B-37",
        name: "Anal beads",
        group: Some("PEN"),
        disposition: "family",
        physics_class: Some("P-C"),
        strata: &[],
        primitives: &[],
        source: SRC_FAMILY,
    },
    // ---- ORAL — mouth / breast contact ----
    ActEntry {
        code: "B-15",
        name: "Fellatio, silhouette partner",
        group: Some("ORAL"),
        disposition: "family",
        physics_class: Some("P-A"),
        strata: &[],
        primitives: &[],
        source: SRC_FAMILY,
    },
    ActEntry {
        code: "B-43",
        name: "Deep throating",
        group: Some("ORAL"),
        disposition: "family",
        physics_class: Some("P-A"),
        strata: &[],
        primitives: &[],
        source: SRC_FAMILY,
    },
    ActEntry {
        code: "B-33",
        name: "Paizuri",
        group: Some("ORAL"),
        disposition: "family",
        physics_class: Some("P-A"),
        strata: &[],
        primitives: &[],
        source: SRC_FAMILY,
    },
    ActEntry {
        code: "KISS",
        name: "Kissing",
        group: Some("ORAL"),
        disposition: "family",
        physics_class: Some("P-C"),
        strata: &[],
        primitives: &[],
        source: SRC_LAB_CODE,
    },
    // ---- DEV — device-driven ----
    ActEntry {
        code: "B-03",
        name: "Sybian machine",
        group: Some("DEV"),
        disposition: "family",
        physics_class: Some("P-C"),
        strata: &[],
        primitives: &[],
        source: SRC_FAMILY,
    },
    ActEntry {
        code: "B-04",
        name: "Hitachi wand",
        group: Some("DEV"),
        disposition: "family",
        physics_class: Some("P-C"),
        strata: &[],
        primitives: &[],
        source: SRC_FAMILY,
    },
    ActEntry {
        code: "B-22",
        name: "Fucking machine",
        group: Some("DEV"),
        disposition: "family",
        physics_class: Some("P-C"),
        strata: &[],
        primitives: &[],
        source: SRC_FAMILY,
    },
    ActEntry {
        code: "B-51",
        name: "Remote control devices",
        group: Some("DEV"),
        disposition: "family",
        physics_class: Some("P-C"),
        strata: &[],
        primitives: &[],
        source: SRC_FAMILY,
    },
    // ---- IMP — impact / punctate contact ----
    ActEntry {
        code: "B-08",
        name: "Spanking, bare hand",
        group: Some("IMP"),
        disposition: "family",
        physics_class: Some("P-C"),
        strata: &[],
        primitives: &[],
        source: SRC_FAMILY,
    },
    ActEntry {
        code: "B-09",
        name: "Spanking, instruments",
        group: Some("IMP"),
        disposition: "family",
        physics_class: Some("P-C"),
        strata: &[],
        primitives: &[],
        source: SRC_FAMILY,
    },
    ActEntry {
        code: "B-59",
        name: "Over-the-knee spanking",
        group: Some("IMP"),
        disposition: "family",
        physics_class: Some("P-C"),
        strata: &[],
        primitives: &[],
        source: SRC_FAMILY,
    },
    ActEntry {
        code: "B-42",
        name: "Biting",
        group: Some("IMP"),
        disposition: "family",
        physics_class: Some("P-C"),
        strata: &[],
        primitives: &[],
        source: SRC_FAMILY,
    },
    ActEntry {
        code: "B-46",
        name: "Hair pulling",
        group: Some("IMP"),
        disposition: "family",
        physics_class: Some("P-C"),
        strata: &[],
        primitives: &[],
        source: SRC_FAMILY,
    },
    ActEntry {
        code: "B-48",
        name: "Nipple play",
        group: Some("IMP"),
        disposition: "family",
        physics_class: Some("P-B"),
        strata: &["S-18"],
        primitives: &["mv.limb.grasp", "mv.tors.breast-bounce", "mv.flud.dribble"],
        source: SRC_B48,
    },
    // ---- RST — restraint / held / locomotion ----
    ActEntry {
        code: "B-02",
        name: "Handcuffs",
        group: Some("RST"),
        disposition: "family",
        physics_class: Some("P-C"),
        strata: &[],
        primitives: &[],
        source: SRC_FAMILY,
    },
    ActEntry {
        code: "B-11",
        name: "Walked on a collar",
        group: Some("RST"),
        disposition: "family",
        physics_class: Some("P-C"),
        strata: &[],
        primitives: &[],
        source: SRC_FAMILY,
    },
    ActEntry {
        code: "B-14",
        name: "Full-body shibari",
        group: Some("RST"),
        disposition: "family",
        physics_class: Some("P-C"),
        strata: &[],
        primitives: &[],
        source: SRC_FAMILY,
    },
    ActEntry {
        code: "B-20",
        name: "Tied to a bed",
        group: Some("RST"),
        disposition: "family",
        physics_class: Some("P-C"),
        strata: &[],
        primitives: &[],
        source: SRC_FAMILY,
    },
    ActEntry {
        code: "B-21",
        name: "Tied to a chair",
        group: Some("RST"),
        disposition: "family",
        physics_class: Some("P-C"),
        strata: &[],
        primitives: &[],
        source: SRC_FAMILY,
    },
    ActEntry {
        code: "B-29",
        name: "Hands and knees, leash",
        group: Some("RST"),
        disposition: "family",
        physics_class: Some("P-C"),
        strata: &[],
        primitives: &[],
        source: SRC_FAMILY,
    },
    ActEntry {
        code: "B-55",
        name: "Being overpowered",
        group: Some("RST"),
        disposition: "family",
        physics_class: Some("P-C"),
        strata: &[],
        primitives: &[],
        source: SRC_FAMILY,
    },
    ActEntry {
        code: "B-78",
        name: "Over-the-shoulder carry",
        group: Some("RST"),
        disposition: "family",
        physics_class: Some("P-C"),
        strata: &[],
        primitives: &[],
        source: SRC_FAMILY,
    },
    // ---- CONTACT — sustained contact / solo / milk ----
    ActEntry {
        code: "B-25",
        name: "Master worship",
        group: Some("CONTACT"),
        disposition: "family",
        physics_class: Some("P-C"),
        strata: &[],
        primitives: &[],
        source: SRC_FAMILY,
    },
    ActEntry {
        code: "B-26",
        name: "Body writing",
        group: Some("CONTACT"),
        disposition: "family",
        physics_class: Some("P-C"),
        strata: &[],
        primitives: &[],
        source: SRC_FAMILY,
    },
    ActEntry {
        code: "B-79",
        name: "Foot massage as caretaking dominance",
        group: Some("CONTACT"),
        disposition: "family",
        physics_class: Some("P-C"),
        strata: &[],
        primitives: &[],
        source: SRC_FAMILY,
    },
    ActEntry {
        code: "SOLO",
        name: "Masturbation (gentle + feverish merged)",
        group: Some("CONTACT"),
        disposition: "family",
        physics_class: Some("P-C"),
        strata: &[],
        primitives: &[],
        source: SRC_LAB_CODE,
    },
    ActEntry {
        code: "M-01",
        name: "Lactation / milking",
        group: Some("CONTACT"),
        disposition: "family",
        physics_class: Some("P-B"),
        strata: &["S-17", "S-18"],
        primitives: &["mv.flud.spurt-to-dribble"],
        source: SRC_M01,
    },
    // ---- CONTEXT — setting / role / frame (never a geometry) ----
    ActEntry {
        code: "B-01",
        name: "Total power exchange",
        group: None,
        disposition: "context",
        physics_class: None,
        strata: &[],
        primitives: &[],
        source: SRC_DISPOSITION,
    },
    ActEntry {
        code: "B-12",
        name: "Exhibitionism",
        group: None,
        disposition: "context",
        physics_class: None,
        strata: &[],
        primitives: &[],
        source: SRC_DISPOSITION,
    },
    ActEntry {
        code: "B-28",
        name: "Public play",
        group: None,
        disposition: "context",
        physics_class: None,
        strata: &[],
        primitives: &[],
        source: SRC_DISPOSITION,
    },
    ActEntry {
        code: "B-31",
        name: "1950s household / stepfordization",
        group: None,
        disposition: "context",
        physics_class: None,
        strata: &[],
        primitives: &[],
        source: SRC_DISPOSITION,
    },
    ActEntry {
        code: "B-35",
        name: "Consensual punishment",
        group: None,
        disposition: "context",
        physics_class: None,
        strata: &[],
        primitives: &[],
        source: SRC_DISPOSITION,
    },
    ActEntry {
        code: "B-39",
        name: "Consensual slavery",
        group: None,
        disposition: "context",
        physics_class: None,
        strata: &[],
        primitives: &[],
        source: SRC_DISPOSITION,
    },
    ActEntry {
        code: "B-41",
        name: "Boss / secretary",
        group: None,
        disposition: "context",
        physics_class: None,
        strata: &[],
        primitives: &[],
        source: SRC_DISPOSITION,
    },
    ActEntry {
        code: "B-47",
        name: "High protocol",
        group: None,
        disposition: "context",
        physics_class: None,
        strata: &[],
        primitives: &[],
        source: SRC_DISPOSITION,
    },
    ActEntry {
        code: "B-53",
        name: "Service-oriented submission",
        group: None,
        disposition: "context",
        physics_class: None,
        strata: &[],
        primitives: &[],
        source: SRC_DISPOSITION,
    },
    ActEntry {
        code: "B-58",
        name: "Public sex, uncaught",
        group: None,
        disposition: "context",
        physics_class: None,
        strata: &[],
        primitives: &[],
        source: SRC_DISPOSITION,
    },
    ActEntry {
        code: "B-61",
        name: "Maid uniforms",
        group: None,
        disposition: "context",
        physics_class: None,
        strata: &[],
        primitives: &[],
        source: SRC_DISPOSITION,
    },
    ActEntry {
        code: "B-62",
        name: "Interrogation play",
        group: None,
        disposition: "context",
        physics_class: None,
        strata: &[],
        primitives: &[],
        source: SRC_DISPOSITION,
    },
    ActEntry {
        code: "B-64",
        name: "Domestic servitude",
        group: None,
        disposition: "context",
        physics_class: None,
        strata: &[],
        primitives: &[],
        source: SRC_DISPOSITION,
    },
    ActEntry {
        code: "B-76",
        name: "Voyeur-room performance",
        group: None,
        disposition: "context",
        physics_class: None,
        strata: &[],
        primitives: &[],
        source: SRC_DISPOSITION,
    },
    ActEntry {
        code: "B-77",
        name: "Charity auction",
        group: None,
        disposition: "context",
        physics_class: None,
        strata: &[],
        primitives: &[],
        source: SRC_DISPOSITION,
    },
    // ---- REGISTER — speech / tone / dynamic ----
    ActEntry {
        code: "B-24",
        name: "Obedience training",
        group: None,
        disposition: "register",
        physics_class: None,
        strata: &[],
        primitives: &[],
        source: SRC_DISPOSITION,
    },
    ActEntry {
        code: "B-30",
        name: "Begging",
        group: None,
        disposition: "register",
        physics_class: None,
        strata: &[],
        primitives: &[],
        source: SRC_DISPOSITION,
    },
    ActEntry {
        code: "B-49",
        name: "Play punishment",
        group: None,
        disposition: "register",
        physics_class: None,
        strata: &[],
        primitives: &[],
        source: SRC_DISPOSITION,
    },
    ActEntry {
        code: "B-50",
        name: "Consensual psychosexual mind play",
        group: None,
        disposition: "register",
        physics_class: None,
        strata: &[],
        primitives: &[],
        source: SRC_DISPOSITION,
    },
    ActEntry {
        code: "B-67",
        name: "Rewarded good girl",
        group: None,
        disposition: "register",
        physics_class: None,
        strata: &[],
        primitives: &[],
        source: SRC_DISPOSITION,
    },
    ActEntry {
        code: "B-68",
        name: "Brat taming",
        group: None,
        disposition: "register",
        physics_class: None,
        strata: &[],
        primitives: &[],
        source: SRC_DISPOSITION,
    },
    ActEntry {
        code: "B-69",
        name: "Begging for permission to cum",
        group: None,
        disposition: "register",
        physics_class: None,
        strata: &[],
        primitives: &[],
        source: SRC_DISPOSITION,
    },
    ActEntry {
        code: "B-70",
        name: "Affectionate domination",
        group: None,
        disposition: "register",
        physics_class: None,
        strata: &[],
        primitives: &[],
        source: SRC_DISPOSITION,
    },
    ActEntry {
        code: "B-71",
        name: "Yes daddy",
        group: None,
        disposition: "register",
        physics_class: None,
        strata: &[],
        primitives: &[],
        source: SRC_DISPOSITION,
    },
    ActEntry {
        code: "B-72",
        name: "Yes master",
        group: None,
        disposition: "register",
        physics_class: None,
        strata: &[],
        primitives: &[],
        source: SRC_DISPOSITION,
    },
    ActEntry {
        code: "B-74",
        name: "Praise kink as primary stimulus",
        group: None,
        disposition: "register",
        physics_class: None,
        strata: &[],
        primitives: &[],
        source: SRC_DISPOSITION,
    },
    // ---- REGIME — control / temporal pattern over a host act ----
    ActEntry {
        code: "B-23",
        name: "Orgasm overstimulation",
        group: None,
        disposition: "regime",
        physics_class: None,
        strata: &[],
        primitives: &[],
        source: SRC_DISPOSITION,
    },
    ActEntry {
        code: "B-34",
        name: "Orgasm denial",
        group: None,
        disposition: "regime",
        physics_class: None,
        strata: &[],
        primitives: &[],
        source: SRC_DISPOSITION,
    },
    ActEntry {
        code: "B-40",
        name: "Consensual corruption",
        group: None,
        disposition: "regime",
        physics_class: None,
        strata: &[],
        primitives: &[],
        source: SRC_DISPOSITION,
    },
    ActEntry {
        code: "B-44",
        name: "Edging",
        group: None,
        disposition: "regime",
        physics_class: None,
        strata: &[],
        primitives: &[],
        source: SRC_DISPOSITION,
    },
    ActEntry {
        code: "B-45",
        name: "Gooning",
        group: None,
        disposition: "regime",
        physics_class: None,
        strata: &[],
        primitives: &[],
        source: SRC_DISPOSITION,
    },
    ActEntry {
        code: "B-73",
        name: "Multiple orgasms as punishment",
        group: None,
        disposition: "regime",
        physics_class: None,
        strata: &[],
        primitives: &[],
        source: SRC_DISPOSITION,
    },
    // ---- REACTION OVERLAY — state or trigger-direction ----
    ActEntry {
        code: "B-38",
        name: "Ben wa balls",
        group: None,
        disposition: "reaction_overlay",
        physics_class: None,
        strata: &[],
        primitives: &[],
        source: SRC_DISPOSITION,
    },
    ActEntry {
        code: "B-54",
        name: "Deep subspace",
        group: None,
        disposition: "reaction_overlay",
        physics_class: None,
        strata: &[],
        primitives: &[],
        source: SRC_DISPOSITION,
    },
    ActEntry {
        code: "B-63",
        name: "Pleasure from pleasing",
        group: None,
        disposition: "reaction_overlay",
        physics_class: None,
        strata: &[],
        primitives: &[],
        source: SRC_DISPOSITION,
    },
    ActEntry {
        code: "B-75",
        name: "Voyeurism, watching side",
        group: None,
        disposition: "reaction_overlay",
        physics_class: None,
        strata: &[],
        primitives: &[],
        source: SRC_DISPOSITION,
    },
    // ---- MODIFIER — prop / costume / display / concealed variant ----
    ActEntry {
        code: "B-10",
        name: "Collar portrait",
        group: None,
        disposition: "modifier",
        physics_class: None,
        strata: &[],
        primitives: &[],
        source: SRC_DISPOSITION,
    },
    ActEntry {
        code: "B-13",
        name: "Blindfold",
        group: None,
        disposition: "modifier",
        physics_class: None,
        strata: &[],
        primitives: &[],
        source: SRC_DISPOSITION,
    },
    ActEntry {
        code: "B-27",
        name: "Pierced nipples",
        group: None,
        disposition: "modifier",
        physics_class: None,
        strata: &[],
        primitives: &[],
        source: SRC_DISPOSITION,
    },
    ActEntry {
        code: "B-56",
        name: "Lingerie",
        group: None,
        disposition: "modifier",
        physics_class: None,
        strata: &[],
        primitives: &[],
        source: SRC_DISPOSITION,
    },
    ActEntry {
        code: "B-57",
        name: "Vintage lingerie",
        group: None,
        disposition: "modifier",
        physics_class: None,
        strata: &[],
        primitives: &[],
        source: SRC_DISPOSITION,
    },
    ActEntry {
        code: "B-65",
        name: "Covert bondage",
        group: None,
        disposition: "modifier",
        physics_class: None,
        strata: &[],
        primitives: &[],
        source: SRC_DISPOSITION,
    },
    ActEntry {
        code: "B-66",
        name: "Slave collars",
        group: None,
        disposition: "modifier",
        physics_class: None,
        strata: &[],
        primitives: &[],
        source: SRC_DISPOSITION,
    },
    // ---- SECTION — prelude / finish / coda ----
    ActEntry {
        code: "B-52",
        name: "Seduction",
        group: None,
        disposition: "section",
        physics_class: None,
        strata: &[],
        primitives: &[],
        source: SRC_DISPOSITION,
    },
    ActEntry {
        code: "B-60",
        name: "Impregnation",
        group: None,
        disposition: "section",
        physics_class: None,
        strata: &["S-12"],
        primitives: &[],
        source: SRC_B60,
    },
    ActEntry {
        code: "B-80",
        name: "Aftercare as its own scene",
        group: None,
        disposition: "section",
        physics_class: None,
        strata: &[],
        primitives: &[],
        source: SRC_DISPOSITION,
    },
];

/// An [ACT]-only catalogue item with no code and no family: a layer,
/// umbrella, or foley item (W6-SPEC §1.1.3). Carried as a candidate
/// so it is never silently dropped.
pub struct ActCandidate {
    pub name: &'static str,
    pub disposition: &'static str,
    pub source: &'static str,
}

const SRC_CANDIDATE: &str =
    "adult-audio-research-20261026/TORORI-CATALOGUE.md [ACT] (no B-code); night-20261026/w6/W6-SPEC.md 1.1.3 (layer/umbrella/foley)";

/// The seven uncoded candidates (six layer/umbrella/foley entries +
/// the second masturbation entry W6 merged into SOLO).
pub const ACT_CANDIDATES: &[ActCandidate] = &[
    ActCandidate {
        name: "JOI (spoken instruction format)",
        disposition: "layer",
        source: SRC_CANDIDATE,
    },
    ActCandidate {
        name: "Liquid events (drip, pour, spray, trickle)",
        disposition: "layer",
        source: SRC_CANDIDATE,
    },
    ActCandidate {
        name: "Sex-toy play, general",
        disposition: "umbrella",
        source: SRC_CANDIDATE,
    },
    ActCandidate {
        name: "Thrusting / friction loop",
        disposition: "layer",
        source: SRC_CANDIDATE,
    },
    ActCandidate {
        name: "Vore / swallowing foley",
        disposition: "foley",
        source: SRC_CANDIDATE,
    },
    ActCandidate {
        name: "Wet friction / contact layer",
        disposition: "layer",
        source: SRC_CANDIDATE,
    },
    ActCandidate {
        name: "Masturbation (second tempo entry, merged into SOLO)",
        disposition: "merged_duplicate",
        source: SRC_CANDIDATE,
    },
];

/// Look up a coded entry by its act code.
pub fn act_entry(code: &str) -> Option<&'static ActEntry> {
    ACT_REGISTRY.iter().find(|entry| entry.code == code)
}

/// Whether an entry carries a source-stated W6 mapping: a family
/// group (W6-SPEC §1.1.1) or a stated stratum (ATLAS §4). Anything
/// else is `unmapped` (SPEC §6).
pub fn mapping_status(entry: &ActEntry) -> &'static str {
    if entry.group.is_some() || !entry.strata.is_empty() {
        "mapped"
    } else {
        "unmapped"
    }
}

/// The JSON snapshot of an entry, embedded in ActRecords so a record
/// carries the registry state it was minted against.
pub fn entry_json(entry: &ActEntry) -> Value {
    json!({
        "act_code": entry.code,
        "name": entry.name,
        "disposition": entry.disposition,
        "w6_group": entry.group,
        "physics_class": entry.physics_class,
        "w6_strata": entry.strata,
        "w6_primitives": entry.primitives,
        "mapping": mapping_status(entry),
        "registry_status": "canon",
        "source": entry.source,
        "registry_version": ACT_REGISTRY_VERSION,
    })
}

/// The JSON snapshot for a code the registry does not know: it enters
/// as a candidate (SPEC §6), labelled `unmapped`, never dropped and
/// never presented as canon.
pub fn candidate_json(code: &str) -> Value {
    json!({
        "act_code": code,
        "name": null,
        "disposition": "candidate",
        "w6_group": null,
        "physics_class": null,
        "w6_strata": [],
        "w6_primitives": [],
        "mapping": "unmapped",
        "registry_status": "candidate",
        "source": "not in the registry seed; entered as a candidate at extraction (SPEC 6)",
        "registry_version": ACT_REGISTRY_VERSION,
    })
}

// ---- S2 vocabularies (closed sets, each grounded in its record) ----

/// Collider kinds for contact pairs: the D-076 spike brief's
/// candidates — XPBD surface samples, authored sphere/capsule
/// collider sets per body region, wrist-anchored spheres
/// (`portrait-transpose-study-20261027/build/CONTACT-SPIKE-BRIEF.md`).
pub const COLLIDER_KINDS: [&str; 3] = ["sphere", "capsule", "surface"];

/// Compliance classes for contact pairs, from the D-076 spike's
/// XPBD constraint classes (`contact-spike-20261027/SPIKE-REPORT.md`
/// §4–§5 + DECISIONS D-076): hard non-penetration projection
/// (alpha = 0), neighbour-distance compliance (alpha_n),
/// attachment compliance (alpha_a), and the falloff field reused as
/// attachment-compliance weighting.
pub const COMPLIANCE_CLASSES: [&str; 4] = [
    "hard_non_penetration",
    "neighbour",
    "attachment",
    "falloff_weighted",
];

/// The §3.6 beat vocabulary — a closed estate set.
pub const BEAT_TYPES: [&str; 10] = [
    "setup",
    "induction",
    "deepening",
    "trigger_installation",
    "trigger_use",
    "testing",
    "act",
    "escalation",
    "aftermath",
    "transition",
];

/// The §3.6 machinery vocabulary: the hypnosis/control mechanism in
/// play, as evidenced.
pub const MECHANISMS: [&str; 7] = [
    "device",
    "app",
    "eye_contact",
    "drug",
    "magic",
    "voice",
    "other",
];

/// Grouping bases for enrichment (delta 21).
pub const GROUPING_BASES: [&str; 3] = ["official_chain", "heuristic_match", "standalone"];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seed_counts_match_sources() {
        // 80 B-codes + KISS / M-01 / SOLO (W6-SPEC §1.1 source arithmetic).
        assert_eq!(ACT_REGISTRY.len(), 83);
        let families = ACT_REGISTRY
            .iter()
            .filter(|e| e.disposition == "family")
            .count();
        assert_eq!(families, 37);
        assert_eq!(ACT_CANDIDATES.len(), 7);
    }

    #[test]
    fn seed_codes_unique_and_grounded() {
        let mut codes: Vec<&str> = ACT_REGISTRY.iter().map(|e| e.code).collect();
        codes.sort();
        codes.dedup();
        assert_eq!(codes.len(), ACT_REGISTRY.len());
        for entry in ACT_REGISTRY {
            assert!(!entry.name.is_empty(), "{}", entry.code);
            assert!(!entry.source.is_empty(), "{}", entry.code);
        }
    }

    #[test]
    fn stated_strata_are_the_only_strata() {
        // ATLAS §4 states exactly three code->stratum links.
        let linked: Vec<&str> = ACT_REGISTRY
            .iter()
            .filter(|e| !e.strata.is_empty())
            .map(|e| e.code)
            .collect();
        assert_eq!(linked, vec!["B-48", "M-01", "B-60"]);
    }
}
