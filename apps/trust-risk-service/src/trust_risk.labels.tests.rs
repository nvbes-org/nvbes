use nvbes_trust_risk::proto::nvbes::trust_risk::v1::LabelSourceClass;

use super::authority_rank;

#[test]
fn canonical_precedence_excludes_heuristics() {
    assert!(
        authority_rank(LabelSourceClass::Human)
            > authority_rank(LabelSourceClass::AuthoritativeExternal)
    );
    assert!(
        authority_rank(LabelSourceClass::AuthoritativeExternal)
            > authority_rank(LabelSourceClass::VerifiedProduct)
    );
    assert_eq!(authority_rank(LabelSourceClass::Heuristic), None);
}
