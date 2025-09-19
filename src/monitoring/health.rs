//! Health Monitoring
//!
//! Health checks and system health monitoring

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, RwLock};
use std::time::Instant;

/// Health monitoring system
pub struct HealthMonitor {
    checks: Arc<RwLock<HashMap<String, HealthCheck>>>,
    check_interval: std::time::Duration,
}

impl HealthMonitor {
    pub fn new(interval: std::time::Duration) -> Self {
        Self {
            checks: Arc::new(RwLock::new(HashMap::new())),
            check_interval: interval,
        }
    }

    pub fn record_check(&self, component: &str, healthy: bool, details: Option<String>) {
        let mut checks = self.checks.write().unwrap();
        let check = checks.entry(component.to_string()).or_insert_with(|| {
            HealthCheck::new(component.to_string())
        });

        check.record_result(healthy, details);
    }

    pub fn get_overall_health(&self) -> HealthStatus {
        let checks = self.checks.read().unwrap();

        if checks.is_empty() {
            return HealthStatus::Unknown;
        }

        let unhealthy_count = checks.values()
            .filter(|check| !check.is_healthy())
            .count();

        match unhealthy_count {
            0 => HealthStatus::Healthy,
            n if n < checks.len() / 2 => HealthStatus::Degraded,
            _ => HealthStatus::Unhealthy,
        }
    }

    pub fn get_component_health(&self, component: &str) -> Option<HealthStatus> {
        let checks = self.checks.read().unwrap();
        checks.get(component).map(|check| {
            if check.is_healthy() {
                HealthStatus::Healthy
            } else {
                HealthStatus::Unhealthy
            }
        })
    }
}

/// Individual health check
#[derive(Debug, Clone)]
pub struct HealthCheck {
    pub component: String,
    pub last_check: Option<Instant>,
    pub is_healthy: bool,
    pub consecutive_failures: u32,
    pub details: Option<String>,
    pub history: VecDeque<HealthCheckResult>,
}

impl HealthCheck {
    pub fn new(component: String) -> Self {
        Self {
            component,
            last_check: None,
            is_healthy: true,
            consecutive_failures: 0,
            details: None,
            history: VecDeque::with_capacity(100),
        }
    }

    pub fn record_result(&mut self, healthy: bool, details: Option<String>) {
        self.last_check = Some(Instant::now());
        self.is_healthy = healthy;
        self.details = details.clone();

        if healthy {
            self.consecutive_failures = 0;
        } else {
            self.consecutive_failures += 1;
        }

        if self.history.len() >= 100 {
            self.history.pop_front();
        }

        self.history.push_back(HealthCheckResult {
            timestamp: Instant::now(),
            healthy,
            details,
        });
    }

    pub fn is_healthy(&self) -> bool {
        self.is_healthy && self.consecutive_failures < 3
    }
}

#[derive(Debug, Clone)]
pub struct HealthCheckResult {
    pub timestamp: Instant,
    pub healthy: bool,
    pub details: Option<String>,
}

/// Health status enumeration
#[derive(Debug, Clone, PartialEq)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}
