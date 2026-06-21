package identitysdk

import "testing"

func TestHealthPath(t *testing.T) {
	client := NewClient("https://identity.example.test")
	if got := client.HealthPath(); got != "https://identity.example.test/health" {
		t.Fatalf("HealthPath() = %q", got)
	}
}
