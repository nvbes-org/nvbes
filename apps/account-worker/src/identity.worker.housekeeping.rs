use std::time::{Duration, Instant};

use nvbes_observability::{
    WorkerMonitorSchedule, start_worker_monitor_check_in, worker_monitor_slug,
};

use crate::app::AppState;

const HOUSEKEEPING_INTERVAL: Duration = Duration::from_secs(3600);
const HOUSEKEEPING_MONITOR_SCHEDULE: WorkerMonitorSchedule = WorkerMonitorSchedule {
    interval_minutes: 60,
    checkin_margin_minutes: 15,
    max_runtime_minutes: 30,
};

pub async fn run_if_due(state: &AppState, last_run: &mut Instant) -> anyhow::Result<()> {
    if last_run.elapsed() < HOUSEKEEPING_INTERVAL {
        return Ok(());
    }

    let account_check_in = start_worker_monitor_check_in(
        &state.config.environment,
        &worker_monitor_slug("account-worker", "housekeeping-expired-unverified-accounts"),
        HOUSEKEEPING_MONITOR_SCHEDULE,
    );

    let result =
        nvbes_account_service::domains::auth::email_verification::cleanup_expired_unverified_accounts(
            &state.db,
            state.config.auth_unverified_account_ttl_days,
        )
        .await
        .map_err(|error| anyhow::anyhow!(error.message));

    let deleted = match result {
        Ok(deleted) => {
            account_check_in.finish_ok();
            deleted
        }
        Err(error) => {
            account_check_in.finish_error();
            return Err(error);
        }
    };

    if deleted > 0 {
        tracing::info!(deleted, "Expired unverified accounts cleaned up");
    }

    run_geo_housekeeping(state).await?;
    run_v2fly_geoip_import_if_due(state).await?;
    run_maxmind_geolite_import_if_due(state).await?;
    run_loyalsoldier_geoip_import_if_due(state).await?;

    *last_run = Instant::now();
    Ok(())
}

async fn run_maxmind_geolite_import_if_due(state: &AppState) -> anyhow::Result<()> {
    if !state.config.maxmind_geolite_database_enabled {
        return Ok(());
    }

    let check_in = start_worker_monitor_check_in(
        &state.config.environment,
        &worker_monitor_slug("account-worker", "housekeeping-maxmind-geolite-import"),
        HOUSEKEEPING_MONITOR_SCHEDULE,
    );

    let report = nvbes_region::geo::run_scheduled_maxmind_geolite_import(
        &state.db,
        maxmind_geolite_config(&state.config)?,
    )
    .await;
    let report = match report {
        Ok(report) => {
            check_in.finish_ok();
            report
        }
        Err(error) => {
            check_in.finish_error();
            return Err(anyhow::anyhow!(error));
        }
    };

    if report.downloaded || report.expired_ranges > 0 {
        tracing::info!(
            downloaded = report.downloaded,
            imported_ranges = report.imported_ranges,
            expired_ranges = report.expired_ranges,
            country_ranges = report.country_ranges,
            city_ranges = report.city_ranges,
            asn_ranges = report.asn_ranges,
            "MaxMind GeoLite import completed"
        );
    }

    Ok(())
}

fn maxmind_geolite_config(
    config: &nvbes_core::config::AppConfig,
) -> anyhow::Result<nvbes_region::geo::MaxMindGeoLiteConfig> {
    Ok(nvbes_region::geo::MaxMindGeoLiteConfig {
        account_id: config
            .maxmind_account_id
            .clone()
            .ok_or_else(|| anyhow::anyhow!("NVBES_MAXMIND_ACCOUNT_ID is required"))?,
        license_key: config
            .maxmind_license_key
            .clone()
            .ok_or_else(|| anyhow::anyhow!("NVBES_MAXMIND_LICENSE_KEY is required"))?,
        eula_accepted: config.maxmind_geolite_eula_accepted,
    })
}

async fn run_loyalsoldier_geoip_import_if_due(state: &AppState) -> anyhow::Result<()> {
    if !state.config.loyalsoldier_geoip_enabled {
        return Ok(());
    }

    let check_in = start_worker_monitor_check_in(
        &state.config.environment,
        &worker_monitor_slug("account-worker", "housekeeping-loyalsoldier-geoip-import"),
        HOUSEKEEPING_MONITOR_SCHEDULE,
    );

    let report = nvbes_region::geo::run_scheduled_loyalsoldier_geoip_import(&state.db).await;
    let report = match report {
        Ok(report) => {
            check_in.finish_ok();
            report
        }
        Err(error) => {
            check_in.finish_error();
            return Err(anyhow::anyhow!(error));
        }
    };

    if report.downloaded || report.expired_ranges > 0 {
        tracing::info!(
            downloaded = report.downloaded,
            imported_ranges = report.imported_ranges,
            expired_ranges = report.expired_ranges,
            country_ranges = report.country_ranges,
            category_ranges = report.category_ranges,
            "Loyalsoldier GeoIP import completed"
        );
    }

    Ok(())
}

async fn run_geo_housekeeping(state: &AppState) -> anyhow::Result<()> {
    let check_in = start_worker_monitor_check_in(
        &state.config.environment,
        &worker_monitor_slug("account-worker", "housekeeping-geo-lookup-cache"),
        HOUSEKEEPING_MONITOR_SCHEDULE,
    );

    let report = nvbes_region::geo::run_geo_maintenance(&state.db).await;
    let report = match report {
        Ok(report) => {
            check_in.finish_ok();
            report
        }
        Err(error) => {
            check_in.finish_error();
            return Err(anyhow::anyhow!(error));
        }
    };

    if report.expired_personal_ranges_disabled > 0
        || report.expired_unreferenced_relations_deleted > 0
        || report.expired_maxmind_evidence_scrubbed > 0
        || report.expired_maxmind_relations_deleted > 0
    {
        tracing::info!(
            expired_personal_ranges_disabled = report.expired_personal_ranges_disabled,
            expired_unreferenced_relations_deleted = report.expired_unreferenced_relations_deleted,
            expired_maxmind_evidence_scrubbed = report.expired_maxmind_evidence_scrubbed,
            expired_maxmind_relations_deleted = report.expired_maxmind_relations_deleted,
            "Geo lookup maintenance completed"
        );
    }

    Ok(())
}

async fn run_v2fly_geoip_import_if_due(state: &AppState) -> anyhow::Result<()> {
    let check_in = start_worker_monitor_check_in(
        &state.config.environment,
        &worker_monitor_slug("account-worker", "housekeeping-v2fly-geoip-import"),
        HOUSEKEEPING_MONITOR_SCHEDULE,
    );

    let report = nvbes_region::geo::run_scheduled_v2fly_geoip_import(&state.db).await;
    let report = match report {
        Ok(report) => {
            check_in.finish_ok();
            report
        }
        Err(error) => {
            check_in.finish_error();
            return Err(anyhow::anyhow!(error));
        }
    };

    if report.downloaded || report.expired_ranges > 0 {
        tracing::info!(
            downloaded = report.downloaded,
            imported_ranges = report.imported_ranges,
            expired_ranges = report.expired_ranges,
            "v2fly GeoIP import completed"
        );
    }

    Ok(())
}
