package provisioning

import "testing"

func TestBuildProvisioningPlanIsDeterministicAndAudited(t *testing.T) {
	request := WorkspaceProvisioningRequest{
		WorkspaceID:  "workspace-123",
		RegionID:     "eu-fr",
		PlanCode:     "business",
		ServiceClass: ServiceClassBusiness,
	}

	first, err := BuildProvisioningPlan(request)
	if err != nil {
		t.Fatalf("expected provisioning plan: %v", err)
	}
	second, err := BuildProvisioningPlan(request)
	if err != nil {
		t.Fatalf("expected second provisioning plan: %v", err)
	}

	if first.IdempotencyKey != second.IdempotencyKey {
		t.Fatalf("expected deterministic root idempotency key")
	}
	if first.RequiredAuditType != "cloud.provisioning.plan_created" {
		t.Fatalf("expected cloud provisioning audit event")
	}
	if first.SLA != "99.9" {
		t.Fatalf("expected business SLA, got %s", first.SLA)
	}
	if len(first.Steps) != 4 {
		t.Fatalf("expected four provisioning steps, got %d", len(first.Steps))
	}

	seen := map[string]bool{}
	for _, step := range first.Steps {
		if step.IdempotencyKey == "" {
			t.Fatalf("step %s missing idempotency key", step.Name)
		}
		if seen[step.IdempotencyKey] {
			t.Fatalf("duplicate step idempotency key for %s", step.Name)
		}
		seen[step.IdempotencyKey] = true
	}
}

func TestBuildProvisioningPlanRejectsInvalidRequests(t *testing.T) {
	invalid := []WorkspaceProvisioningRequest{
		{RegionID: "eu-fr", PlanCode: "starter", ServiceClass: ServiceClassStarter},
		{WorkspaceID: "workspace-123", PlanCode: "starter", ServiceClass: ServiceClassStarter},
		{WorkspaceID: "workspace-123", RegionID: "eu-fr", ServiceClass: ServiceClassStarter},
		{WorkspaceID: "workspace-123", RegionID: "eu-fr", PlanCode: "starter", ServiceClass: "custom"},
	}

	for _, request := range invalid {
		if _, err := BuildProvisioningPlan(request); err == nil {
			t.Fatalf("expected invalid request to fail: %#v", request)
		}
	}
}

func TestAuditEventTypesAreVersionableCloudEvents(t *testing.T) {
	events := AuditEventTypes()
	expected := []string{
		"cloud.provisioning.plan_created",
		"cloud.provisioning.step_applied",
		"cloud.provisioning.rollback_requested",
	}

	if len(events) != len(expected) {
		t.Fatalf("expected %d events, got %d", len(expected), len(events))
	}
	for index, event := range expected {
		if events[index] != event {
			t.Fatalf("event %d = %s, want %s", index, events[index], event)
		}
	}
}
