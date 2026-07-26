-- Indexes used by the expired geo-relation cleanup.
CREATE INDEX IF NOT EXISTS idx_geo_ip_network_relations_expired
  ON geo_ip_network_relations (expires_at, id)
  WHERE expires_at IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_geo_lookup_evidence_network_relation
  ON geo_lookup_evidence (network_relation_id)
  WHERE network_relation_id IS NOT NULL;
