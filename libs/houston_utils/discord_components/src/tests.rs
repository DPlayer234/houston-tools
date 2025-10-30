use super::*;

#[test]
fn components() {
    let comps = components![CreateTextDisplay::new("hello")];
    assert!(matches!(
        comps.as_slice(),
        [CreateComponent::TextDisplay(_)]
    ));
}

#[test]
fn components_array() {
    let comps = components_array![CreateTextDisplay::new("hello")];
    assert!(matches!(
        comps.as_slice(),
        [CreateComponent::TextDisplay(_)]
    ));
}

#[test]
fn section_components() {
    let comps = section_components![CreateTextDisplay::new("hello")];
    assert!(matches!(
        comps.as_slice(),
        [CreateSectionComponent::TextDisplay(_)]
    ));
}
