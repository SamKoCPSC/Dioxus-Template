use super::*;
#[test]
fn mutations_require_the_initiating_verified_account() {
    let alice = Uuid::from_u128(1);
    let bob = Uuid::from_u128(2);
    assert_eq!(
        check_account_binding(Some(alice), Some(alice)).unwrap(),
        alice
    );
    assert!(check_account_binding(Some(alice), Some(bob)).is_err());
    assert!(check_account_binding(None, Some(alice)).is_err());
    assert!(check_account_binding(Some(alice), None).is_err());
}
#[test]
fn form_binding_is_not_reassigned_after_an_account_switch() {
    let mut binding = Some(Uuid::from_u128(1));
    latch_form_account(&mut binding, Some(Uuid::from_u128(2)));
    assert_eq!(binding, Some(Uuid::from_u128(1)));
}

#[test]
fn stale_completion_cannot_update_a_different_accounts_context() {
    let alice = Uuid::from_u128(1);
    let bob = Uuid::from_u128(2);
    assert!(account_completion_matches(Some(alice), Some(alice), alice));
    assert!(!account_completion_matches(Some(alice), Some(bob), alice));
    assert!(!account_completion_matches(Some(alice), None, alice));
}
