-- Add network restrictions column to tenants
ALTER TABLE tenants ADD COLUMN allowed_ips TEXT[];
