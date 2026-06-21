use nvbes_billing::catalog::PlanVersion;

pub fn catalog_plan_code(plan: &PlanVersion) -> &str {
    &plan.plan_code
}
