use super::{is_allowed, validate_permissions, PermissionRule};

fn rule(subject: &str, actions: &[&str]) -> PermissionRule {
    PermissionRule {
        subject: subject.to_owned(),
        action: actions.iter().map(|action| (*action).to_owned()).collect(),
    }
}

#[test]
fn exact_subject_and_action_is_allowed() {
    let rules = vec![rule("Channel", &["create"])];
    assert!(is_allowed(&rules, "Channel", "create"));
}

#[test]
fn other_subjects_and_actions_are_denied() {
    let rules = vec![rule("Channel", &["create"])];
    assert!(!is_allowed(&rules, "Invite", "create"));
    assert!(!is_allowed(&rules, "Channel", "delete"));
}

#[test]
fn empty_permissions_deny_everything() {
    assert!(!is_allowed(&[], "Channel", "read"));
}

// `manage` is the widest action: holding it satisfies every narrower one.
// Checking for a narrower action alone is the mistake this guards against.
#[test]
fn manage_satisfies_every_action() {
    let rules = vec![rule("ServerRole", &["manage"])];
    for action in ["delete", "create", "read", "update", "manage"] {
        assert!(is_allowed(&rules, "ServerRole", action), "{action}");
    }
}

// The `all` subject is the widest subject, and is just as easy to forget.
#[test]
fn the_all_subject_matches_every_subject() {
    let rules = vec![rule("all", &["read"])];
    assert!(is_allowed(&rules, "Channel", "read"));
    assert!(is_allowed(&rules, "InstanceRole", "read"));
    assert!(!is_allowed(&rules, "Channel", "delete"));
}

#[test]
fn all_combined_with_manage_grants_everything() {
    let rules = vec![rule("all", &["manage"])];
    assert!(is_allowed(&rules, "Invite", "delete"));
    assert!(is_allowed(&rules, "Server", "create"));
}

#[test]
fn a_grant_is_found_across_separate_rules() {
    let rules = vec![rule("Channel", &["read"]), rule("Invite", &["manage"])];
    assert!(is_allowed(&rules, "Invite", "create"));
    assert!(!is_allowed(&rules, "Channel", "create"));
}

#[test]
fn validate_permissions_rejects_unknown_subjects_and_actions() {
    let subjects = &["Channel", "all"];
    assert!(
        validate_permissions(&[rule("Channel", &["read"])], subjects).is_ok()
    );
    assert!(validate_permissions(&[rule("Nope", &["read"])], subjects).is_err());
    assert!(
        validate_permissions(&[rule("Channel", &["fly"])], subjects).is_err()
    );
    assert!(validate_permissions(&[rule("Channel", &[])], subjects).is_err());
}
