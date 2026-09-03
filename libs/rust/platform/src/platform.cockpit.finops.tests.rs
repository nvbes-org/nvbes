use super::*;

#[test]
fn evaluates_budget_stages_consistently() {
    let monitor = FinOpsMonitor::default();

    assert_eq!(monitor.stage_for_spend(1500), BudgetStage::Normal);
    assert_eq!(monitor.stage_for_spend(2499), BudgetStage::Normal);
    assert_eq!(
        monitor.stage_for_spend(2500),
        BudgetStage::DisableNonEssential
    );
    assert_eq!(
        monitor.stage_for_spend(2799),
        BudgetStage::DisableNonEssential
    );
    assert_eq!(
        monitor.stage_for_spend(2800),
        BudgetStage::FreezeCostCreation
    );
    assert_eq!(
        monitor.stage_for_spend(2999),
        BudgetStage::FreezeCostCreation
    );
    assert_eq!(monitor.stage_for_spend(3000), BudgetStage::EssentialOnly);
    assert_eq!(monitor.stage_for_spend(3500), BudgetStage::EssentialOnly);
}

#[test]
fn computes_projections_linearly() {
    // 5 EUR in 10 days of a 30-day month -> projected 15 EUR
    let proj = FinOpsMonitor::project_monthly_spend(500, 10, 30);
    assert_eq!(proj, 1500);

    // 10 EUR in 15 days of 30-day month -> projected 20 EUR
    let proj = FinOpsMonitor::project_monthly_spend(1000, 15, 30);
    assert_eq!(proj, 2000);
}

#[test]
fn generates_complete_category_breakdown() {
    let monitor = FinOpsMonitor::default();
    let mut spends = HashMap::new();
    spends.insert("domain_dns", 200);
    spends.insert("compute", 150);

    let summary = monitor.summarize(350, 10, 30, spends);
    assert_eq!(summary.current_spend_cents, 350);
    assert_eq!(summary.stage, BudgetStage::Normal);
    assert_eq!(summary.categories.len(), 7);

    let compute_cat = summary
        .categories
        .iter()
        .find(|c| c.category == "compute")
        .expect("compute category must exist");
    assert_eq!(compute_cat.current_cents, 150);
    assert_eq!(compute_cat.target_cents, 300);
    assert_eq!(compute_cat.limit_cents, 600);
}
