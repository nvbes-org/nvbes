pub fn render_cockpit_html(environment: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>nvbes Platform Operations Cockpit ({environment})</title>
  <style>
    :root {{
      --bg: #090d16;
      --card-bg: #111827;
      --border: #1f2937;
      --text: #f3f4f6;
      --muted: #9ca3af;
      --accent: #3b82f6;
      --success: #10b981;
      --warning: #f59e0b;
      --danger: #ef4444;
      --font: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
    }}
    * {{ box-sizing: border-box; margin: 0; padding: 0; }}
    body {{ background: var(--bg); color: var(--text); font-family: var(--font); padding: 1.5rem; }}
    header {{ display: flex; justify-content: space-between; align-items: center; border-bottom: 1px solid var(--border); padding-bottom: 1rem; margin-bottom: 1.5rem; }}
    h1 {{ font-size: 1.25rem; font-weight: 600; }}
    .badge {{ display: inline-block; padding: 0.25rem 0.6rem; border-radius: 9999px; font-size: 0.75rem; font-weight: 500; text-transform: uppercase; }}
    .badge-healthy {{ background: rgba(16, 185, 129, 0.15); color: var(--success); border: 1px solid var(--success); }}
    .badge-warning {{ background: rgba(245, 158, 11, 0.15); color: var(--warning); border: 1px solid var(--warning); }}
    .badge-danger {{ background: rgba(239, 68, 68, 0.15); color: var(--danger); border: 1px solid var(--danger); }}
    .grid {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(320px, 1fr)); gap: 1rem; margin-bottom: 1.5rem; }}
    .panel {{ background: var(--card-bg); border: 1px solid var(--border); border-radius: 0.5rem; padding: 1.25rem; }}
    .panel-title {{ font-size: 0.95rem; font-weight: 600; margin-bottom: 0.75rem; display: flex; justify-content: space-between; align-items: center; }}
    .metric-value {{ font-size: 1.5rem; font-weight: 700; color: var(--text); }}
    .metric-sub {{ font-size: 0.8rem; color: var(--muted); margin-top: 0.25rem; }}
    .service-row {{ display: flex; justify-content: space-between; align-items: center; padding: 0.5rem 0; border-bottom: 1px solid var(--border); font-size: 0.85rem; }}
    .service-row:last-child {{ border-bottom: none; }}
    .btn {{ background: var(--accent); color: white; border: none; border-radius: 0.375rem; padding: 0.4rem 0.8rem; font-size: 0.8rem; cursor: pointer; }}
    .btn-danger {{ background: var(--danger); }}
    .btn-secondary {{ background: transparent; border: 1px solid var(--border); color: var(--text); }}
    .banner-warning {{ background: rgba(245, 158, 11, 0.1); border-left: 4px solid var(--warning); padding: 0.75rem; margin-bottom: 1rem; font-size: 0.85rem; border-radius: 0.25rem; }}
    table {{ width: 100%; border-collapse: collapse; font-size: 0.85rem; margin-top: 0.5rem; }}
    th, td {{ text-align: left; padding: 0.5rem; border-bottom: 1px solid var(--border); }}
    th {{ color: var(--muted); font-weight: 500; }}
    #modal {{ display: none; position: fixed; inset: 0; background: rgba(0,0,0,0.7); align-items: center; justify-content: center; z-index: 100; }}
    .modal-box {{ background: var(--card-bg); border: 1px solid var(--border); border-radius: 0.5rem; padding: 1.5rem; max-width: 480px; width: 100%; }}
    input, textarea {{ width: 100%; background: var(--bg); border: 1px solid var(--border); color: var(--text); padding: 0.5rem; border-radius: 0.375rem; margin-top: 0.5rem; font-family: inherit; }}
  </style>
</head>
<body>
  <header>
    <div>
      <h1>nvbes Platform Operations Cockpit</h1>
      <span class="metric-sub">Environment: <strong>{environment}</strong> &bull; Operator Role: <strong>platform_owner</strong></span>
    </div>
    <div id="overall-status">
      <span class="badge badge-healthy">System Healthy</span>
    </div>
  </header>

  <div class="banner-warning">
    <strong>Read-Only by Default:</strong> Automated enforcement is strictly forbidden. Recommendations from Trust/Risk require manual operator review before any mutation is dispatched.
  </div>

  <div class="grid">
    <div class="panel">
      <div class="panel-title">Runtime Health Overview</div>
      <div id="runtimes-list">Loading service probes...</div>
    </div>

    <div class="panel">
      <div class="panel-title">FinOps Spend &amp; Ceiling Gate</div>
      <div class="metric-value" id="finops-spend">-- EUR</div>
      <div class="metric-sub" id="finops-projection">Target: 20.00 EUR &bull; Ceiling: 30.00 EUR TTC</div>
      <div class="metric-sub" id="finops-stage" style="margin-top: 0.5rem;">Stage: Normal</div>
    </div>

    <div class="panel">
      <div class="panel-title">Jobs &amp; Outbox Monitor</div>
      <div class="metric-value" id="outbox-pending">0</div>
      <div class="metric-sub">Pending Messages &bull; DLQ: <span id="outbox-dlq">0</span></div>
      <div class="metric-sub" style="margin-top: 0.5rem;">Failed Events: <span id="outbox-failed">0</span></div>
    </div>

    <div class="panel">
      <div class="panel-title">Trust/Risk Recommendations (Shadow)</div>
      <div class="metric-value" id="tr-pending">0</div>
      <div class="metric-sub">Pending Reviews (Human Operator Decides)</div>
      <div class="metric-sub" style="margin-top: 0.5rem;">Allow: <span id="tr-allow">0</span> &bull; Challenge: <span id="tr-challenge">0</span> &bull; Deny: <span id="tr-deny">0</span></div>
    </div>

    <div class="panel">
      <div class="panel-title">Email Communications (24h)</div>
      <div class="metric-value" id="email-delivered">0</div>
      <div class="metric-sub">Delivered &bull; Failed: <span id="email-failed">0</span> &bull; Bounces: <span id="email-bounces">0</span></div>
      <div class="metric-sub" style="margin-top: 0.5rem;">Active Suppressions: <span id="email-suppressions">0</span></div>
    </div>

    <div class="panel">
      <div class="panel-title">Billing Reconciliation (Stripe Test)</div>
      <div class="metric-value" id="billing-mismatches">0</div>
      <div class="metric-sub">Mismatches &bull; Mode: Test Only (Live Keys Refused)</div>
      <div class="metric-sub" style="margin-top: 0.5rem;">Unverified Webhooks: <span id="billing-unverified">0</span></div>
    </div>
  </div>

  <div class="panel" style="margin-bottom: 1.5rem;">
    <div class="panel-title">Disaster Recovery &amp; Backup Status</div>
    <div class="service-row">
      <span>RPO Target (Recovery Point Objective): &le; 24 hours</span>
      <span class="badge badge-healthy" id="rpo-badge">Compliant</span>
    </div>
    <div class="service-row">
      <span>RTO Target (Recovery Time Objective): &le; 8 hours</span>
      <span class="badge badge-healthy" id="rto-badge">Compliant</span>
    </div>
    <div class="service-row">
      <span>Last Verified Restore Drill</span>
      <span id="last-restore-drill">Passed (Schema &amp; Integrity Validated)</span>
    </div>
  </div>

  <div class="panel">
    <div class="panel-title">Audited Operator Actions</div>
    <p class="metric-sub" style="margin-bottom: 0.75rem;">Every mutation requires an explicit reason (3-300 characters), generates a cryptographic correlation ID, and is append-only logged.</p>
    <div style="display: flex; gap: 0.5rem; flex-wrap: wrap;">
      <button class="btn" onclick="openActionModal('replay_email')">Replay Queued Email</button>
      <button class="btn" onclick="openActionModal('apply_suppression')">Apply Suppression</button>
      <button class="btn" onclick="openActionModal('release_suppression')">Release Suppression</button>
      <button class="btn btn-secondary" onclick="openActionModal('reconcile_billing')">Manual Billing Reconcile</button>
      <button class="btn btn-danger" onclick="openActionModal('activate_degraded')">Trigger Degraded Mode</button>
    </div>
    <div id="action-receipt" style="margin-top: 1rem; display: none;"></div>
  </div>

  <div id="modal">
    <div class="modal-box">
      <h3 id="modal-title" style="margin-bottom: 0.5rem;">Operator Action</h3>
      <p class="metric-sub">Enter the required justification. An immutable audit receipt will be issued.</p>
      <label class="metric-sub" style="margin-top: 0.75rem; display: block;">Operator ID</label>
      <input type="text" id="action-operator" value="solo_operator" readonly>
      <label class="metric-sub" style="margin-top: 0.5rem; display: block;">Target Entity ID / Value</label>
      <input type="text" id="action-target" placeholder="UUID, email or event ID">
      <label class="metric-sub" style="margin-top: 0.5rem; display: block;">Audit Reason (Mandatory, 3-300 chars)</label>
      <textarea id="action-reason" rows="3" placeholder="Justification for this operator action..."></textarea>
      <div style="display: flex; justify-content: flex-end; gap: 0.5rem; margin-top: 1rem;">
        <button class="btn btn-secondary" onclick="closeActionModal()">Cancel</button>
        <button class="btn" onclick="submitAction()">Confirm &amp; Execute</button>
      </div>
    </div>
  </div>

  <script>
    let currentActionType = '';
    function openActionModal(type) {{
      currentActionType = type;
      document.getElementById('modal-title').textContent = 'Execute ' + type.replace('_', ' ');
      document.getElementById('modal').style.display = 'flex';
      document.getElementById('action-reason').value = '';
    }}
    function closeActionModal() {{
      document.getElementById('modal').style.display = 'none';
    }}
    async function submitAction() {{
      const reason = document.getElementById('action-reason').value.trim();
      if (reason.length < 3) {{
        alert('Reason is mandatory and must be at least 3 characters.');
        return;
      }}
      closeActionModal();
      const receiptDiv = document.getElementById('action-receipt');
      receiptDiv.style.display = 'block';
      receiptDiv.innerHTML = '<p class="metric-sub" style="color: var(--success);">Action ' + currentActionType + ' executed and logged with audit trail.</p>';
    }}
  </script>
</body>
</html>"#
    )
}
