//! Expected values ported from Vixen.Tests/Effects/MarkCollectionSelectionServiceTests.cs.
use vixen_core::marks::MarkCollectionType::{self, Generic, Phoneme, State};
use vixen_core::{MarkCollection, Uuid};
use vixen_effects::framework::normalized_mark_selection;

fn collection(kind: MarkCollectionType) -> MarkCollection {
    MarkCollection { collection_type: kind, linked_mark_collection_id: Some(Uuid::new_v4()), ..Default::default() }
}

#[test]
fn preferred_and_fallback_policies_match_upstream_examples() {
    let marks = [collection(Generic), collection(State), collection(State)];
    for (preferred, fallback, expected) in [
        (None, true, marks[0].id),
        (Some(State), true, marks[1].id),
        (Some(State), false, marks[1].id),
        (Some(Phoneme), true, marks[0].id),
        (Some(Phoneme), false, Uuid::nil()),
        (None, false, Uuid::nil()),
    ] {
        assert_eq!(normalized_mark_selection(Uuid::nil(), &marks, preferred, fallback), expected);
        assert_eq!(normalized_mark_selection(Uuid::new_v4(), &marks, preferred, fallback), expected);
    }
}

#[test]
fn existing_selection_wins_even_when_it_does_not_match_the_preferred_type() {
    let marks = [collection(State), collection(Generic)];
    for (preferred, fallback) in [(None, true), (Some(State), true), (Some(Phoneme), false)] {
        assert_eq!(normalized_mark_selection(marks[1].id, &marks, preferred, fallback), marks[1].id);
    }
}

#[test]
fn no_collection_clears_an_active_selection_and_empty_id_is_never_a_match() {
    assert_eq!(normalized_mark_selection(Uuid::new_v4(), &[], None, true), Uuid::nil());
    let marks = [MarkCollection { id: Uuid::nil(), ..collection(Generic) }, collection(State)];
    assert_eq!(normalized_mark_selection(Uuid::nil(), &marks, Some(State), true), marks[1].id);
}

#[test]
fn normalizing_does_not_change_shared_collections_or_links() {
    let marks = [collection(Generic), collection(State)];
    let before = marks.clone();
    normalized_mark_selection(Uuid::nil(), &marks, Some(State), true);
    assert_eq!(marks, before);
}
