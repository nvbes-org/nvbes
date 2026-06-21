package provisioning

import (
	"crypto/sha256"
	"encoding/hex"
	"errors"
	"fmt"
	"strings"
)

type ServiceClass string

const (
	ServiceClassStarter    ServiceClass = "starter"
	ServiceClassBusiness   ServiceClass = "business"
	ServiceClassEnterprise ServiceClass = "enterprise"
)

type WorkspaceProvisioningRequest struct {
	WorkspaceID  string
	RegionID     string
	PlanCode     string
	ServiceClass ServiceClass
}

type ProvisioningStep struct {
	Name           string
	Target         string
	IdempotencyKey string
}

type ProvisioningPlan struct {
	WorkspaceID       string
	RegionID          string
	PlanCode          string
	ServiceClass      ServiceClass
	SLA               string
	IdempotencyKey    string
	RequiredAuditType string
	Steps             []ProvisioningStep
}

func BuildProvisioningPlan(request WorkspaceProvisioningRequest) (ProvisioningPlan, error) {
	if err := validateRequest(request); err != nil {
		return ProvisioningPlan{}, err
	}

	rootKey := idempotencyKey("cloud.provision", request.WorkspaceID, request.RegionID, request.PlanCode)
	steps := []ProvisioningStep{
		step("postgres-cell", "cloud.postgres.cell", rootKey),
		step("object-storage-bucket", "cloud.object-storage.bucket", rootKey),
		step("valkey-namespace", "cloud.valkey.namespace", rootKey),
		step("observability-slo", "cloud.observability.slo", rootKey),
	}

	return ProvisioningPlan{
		WorkspaceID:       request.WorkspaceID,
		RegionID:          request.RegionID,
		PlanCode:          request.PlanCode,
		ServiceClass:      request.ServiceClass,
		SLA:               slaFor(request.ServiceClass),
		IdempotencyKey:    rootKey,
		RequiredAuditType: "cloud.provisioning.plan_created",
		Steps:             steps,
	}, nil
}

func AuditEventTypes() []string {
	return []string{
		"cloud.provisioning.plan_created",
		"cloud.provisioning.step_applied",
		"cloud.provisioning.rollback_requested",
	}
}

func validateRequest(request WorkspaceProvisioningRequest) error {
	if strings.TrimSpace(request.WorkspaceID) == "" {
		return errors.New("workspace_id is required")
	}
	if strings.TrimSpace(request.RegionID) == "" {
		return errors.New("region_id is required")
	}
	if strings.TrimSpace(request.PlanCode) == "" {
		return errors.New("plan_code is required")
	}
	switch request.ServiceClass {
	case ServiceClassStarter, ServiceClassBusiness, ServiceClassEnterprise:
		return nil
	default:
		return fmt.Errorf("unsupported service_class %q", request.ServiceClass)
	}
}

func step(name string, target string, rootKey string) ProvisioningStep {
	return ProvisioningStep{
		Name:           name,
		Target:         target,
		IdempotencyKey: idempotencyKey(rootKey, name, target),
	}
}

func idempotencyKey(parts ...string) string {
	hash := sha256.Sum256([]byte(strings.Join(parts, ":")))
	return hex.EncodeToString(hash[:])
}

func slaFor(class ServiceClass) string {
	switch class {
	case ServiceClassEnterprise:
		return "99.95"
	case ServiceClassBusiness:
		return "99.9"
	default:
		return "best-effort"
	}
}
