package controlplane

import "testing"

func TestHealthStatus(t *testing.T) {
	if HealthStatus() != "ok" {
		t.Fatalf("expected ok health status")
	}
}
