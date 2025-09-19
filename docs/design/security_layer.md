# Security Layer Implementation Design

## 🎯 **Objective**

Implement a comprehensive security layer with rate limiting, input validation, threat detection, authentication, and CSRF protection.

## 📊 **Current State**

### **What's Working**

- ✅ Security configuration structure
- ✅ Basic security manager framework
- ✅ Error handling for security violations
- ✅ Threat level definitions

### **What's Missing**

- ❌ Actual rate limiting implementation
- ❌ Input validation logic
- ❌ Threat detection algorithms
- ❌ Authentication mechanisms
- ❌ CSRF protection
- ❌ Security monitoring and logging

## 🏗 **Architecture Design**

### **Core Components**

```
SecurityManager
├── RateLimiter (request rate limiting)
├── InputValidator (payload validation)
├── ThreatDetector (malicious content detection)
├── Authenticator (JWT and session management)
├── CsrfProtector (CSRF token validation)
└── SecurityMonitor (security event logging)
```

### **Security Flow**

```
Request → Rate Limit Check → Authentication → Input Validation → Threat Detection → Processing
    ↑           ↓                ↓               ↓                ↓               ↓
    └───────────┴────────────────┴───────────────┴────────────────┴───────────────┘
```

## 🛠 **Implementation Plan**

### **Phase 1: Rate Limiting Implementation**

#### **1.1 Token Bucket Rate Limiter**

```rust
pub struct RateLimiter {
    buckets: Arc<Mutex<HashMap<String, TokenBucket>>>,
    default_capacity: u32,
    default_refill_rate: u32,
    refill_interval: Duration,
}

pub struct TokenBucket {
    capacity: u32,
    tokens: u32,
    last_refill: Instant,
    refill_rate: u32,
}

impl RateLimiter {
    pub fn new(default_capacity: u32, default_refill_rate: u32, refill_interval: Duration) -> Self {
        Self {
            buckets: Arc::new(Mutex::new(HashMap::new())),
            default_capacity,
            default_refill_rate,
            refill_interval,
        }
    }

    pub fn check_rate_limit(&mut self, client_id: &str) -> Result<(), RateLimitError> {
        let mut buckets = self.buckets.lock().unwrap();

        // Get or create bucket for client
        let bucket = buckets.entry(client_id.to_string()).or_insert_with(|| {
            TokenBucket {
                capacity: self.default_capacity,
                tokens: self.default_capacity,
                last_refill: Instant::now(),
                refill_rate: self.default_refill_rate,
            }
        });

        // Refill tokens based on time elapsed
        self.refill_tokens(bucket);

        // Check if request can be processed
        if bucket.tokens > 0 {
            bucket.tokens -= 1;
            Ok(())
        } else {
            Err(RateLimitError::RateLimitExceeded {
                client_id: client_id.to_string(),
                retry_after: self.calculate_retry_after(bucket),
            })
        }
    }

    fn refill_tokens(&self, bucket: &mut TokenBucket) {
        let now = Instant::now();
        let time_elapsed = now.duration_since(bucket.last_refill);

        if time_elapsed >= self.refill_interval {
            let intervals_elapsed = time_elapsed.as_millis() / self.refill_interval.as_millis();
            let tokens_to_add = (intervals_elapsed as u32) * bucket.refill_rate;

            bucket.tokens = (bucket.tokens + tokens_to_add).min(bucket.capacity);
            bucket.last_refill = now;
        }
    }

    fn calculate_retry_after(&self, bucket: &TokenBucket) -> Duration {
        if bucket.tokens == 0 {
            let tokens_needed = 1;
            let intervals_needed = (tokens_needed + bucket.refill_rate - 1) / bucket.refill_rate;
            Duration::from_millis(intervals_needed as u64 * self.refill_interval.as_millis())
        } else {
            Duration::from_secs(0)
        }
    }
}
```

#### **1.2 Sliding Window Rate Limiter**

```rust
pub struct SlidingWindowRateLimiter {
    windows: Arc<Mutex<HashMap<String, SlidingWindow>>>,
    window_size: Duration,
    max_requests: u32,
}

pub struct SlidingWindow {
    requests: VecDeque<Instant>,
    max_requests: u32,
    window_size: Duration,
}

impl SlidingWindowRateLimiter {
    pub fn new(window_size: Duration, max_requests: u32) -> Self {
        Self {
            windows: Arc::new(Mutex::new(HashMap::new())),
            window_size,
            max_requests,
        }
    }

    pub fn check_rate_limit(&mut self, client_id: &str) -> Result<(), RateLimitError> {
        let mut windows = self.windows.lock().unwrap();

        let window = windows.entry(client_id.to_string()).or_insert_with(|| {
            SlidingWindow {
                requests: VecDeque::new(),
                max_requests: self.max_requests,
                window_size: self.window_size,
            }
        });

        let now = Instant::now();

        // Remove old requests outside the window
        while let Some(&oldest_request) = window.requests.front() {
            if now.duration_since(oldest_request) > window.window_size {
                window.requests.pop_front();
            } else {
                break;
            }
        }

        // Check if we can add a new request
        if window.requests.len() < window.max_requests as usize {
            window.requests.push_back(now);
            Ok(())
        } else {
            Err(RateLimitError::RateLimitExceeded {
                client_id: client_id.to_string(),
                retry_after: self.calculate_retry_after(window),
            })
        }
    }

    fn calculate_retry_after(&self, window: &SlidingWindow) -> Duration {
        if let Some(&oldest_request) = window.requests.front() {
            let oldest_request_age = Instant::now().duration_since(oldest_request);
            window.window_size - oldest_request_age
        } else {
            Duration::from_secs(0)
        }
    }
}
```

### **Phase 2: Input Validation**

#### **2.1 Comprehensive Input Validator**

```rust
pub struct InputValidator {
    max_message_size: usize,
    allowed_content_types: HashSet<String>,
    forbidden_patterns: Vec<regex::Regex>,
    max_nesting_depth: usize,
    max_array_length: usize,
    max_string_length: usize,
}

impl InputValidator {
    pub fn new(max_message_size: usize) -> Self {
        let mut forbidden_patterns = Vec::new();

        // Add common attack patterns
        forbidden_patterns.push(regex::Regex::new(r"<script[^>]*>.*?</script>").unwrap());
        forbidden_patterns.push(regex::Regex::new(r"javascript:").unwrap());
        forbidden_patterns.push(regex::Regex::new(r"on\w+\s*=").unwrap());
        forbidden_patterns.push(regex::Regex::new(r"eval\s*\(").unwrap());
        forbidden_patterns.push(regex::Regex::new(r"expression\s*\(").unwrap());

        Self {
            max_message_size,
            allowed_content_types: HashSet::from([
                "application/json".to_string(),
                "text/plain".to_string(),
                "application/octet-stream".to_string(),
            ]),
            forbidden_patterns,
            max_nesting_depth: 10,
            max_array_length: 1000,
            max_string_length: 10000,
        }
    }

    pub fn validate_input(&self, payload: &[u8]) -> Result<(), ValidationError> {
        // Check payload size
        if payload.len() > self.max_message_size {
            return Err(ValidationError::PayloadTooLarge {
                size: payload.len(),
                max_size: self.max_message_size,
            });
        }

        // Try to parse as JSON
        if let Ok(json_value) = serde_json::from_slice::<serde_json::Value>(payload) {
            self.validate_json_structure(&json_value, 0)?;
        } else {
            // Validate as plain text
            self.validate_text_content(payload)?;
        }

        Ok(())
    }

    fn validate_json_structure(&self, value: &serde_json::Value, depth: usize) -> Result<(), ValidationError> {
        if depth > self.max_nesting_depth {
            return Err(ValidationError::NestingTooDeep { depth });
        }

        match value {
            serde_json::Value::String(s) => {
                if s.len() > self.max_string_length {
                    return Err(ValidationError::StringTooLong {
                        length: s.len(),
                        max_length: self.max_string_length,
                    });
                }
                self.validate_string_content(s)?;
            }
            serde_json::Value::Array(arr) => {
                if arr.len() > self.max_array_length {
                    return Err(ValidationError::ArrayTooLong {
                        length: arr.len(),
                        max_length: self.max_array_length,
                    });
                }
                for item in arr {
                    self.validate_json_structure(item, depth + 1)?;
                }
            }
            serde_json::Value::Object(obj) => {
                for (key, value) in obj {
                    self.validate_string_content(key)?;
                    self.validate_json_structure(value, depth + 1)?;
                }
            }
            _ => {
                // Numbers, booleans, null are generally safe
            }
        }

        Ok(())
    }

    fn validate_text_content(&self, content: &[u8]) -> Result<(), ValidationError> {
        if let Ok(text) = std::str::from_utf8(content) {
            self.validate_string_content(text)?;
        }
        Ok(())
    }

    fn validate_string_content(&self, content: &str) -> Result<(), ValidationError> {
        // Check for forbidden patterns
        for pattern in &self.forbidden_patterns {
            if pattern.is_match(content) {
                return Err(ValidationError::ForbiddenPattern {
                    pattern: pattern.as_str().to_string(),
                    content: content.to_string(),
                });
            }
        }

        // Check for suspicious content
        if self.is_suspicious_content(content) {
            return Err(ValidationError::SuspiciousContent {
                content: content.to_string(),
            });
        }

        Ok(())
    }

    fn is_suspicious_content(&self, content: &str) -> bool {
        // Check for common attack patterns
        let suspicious_patterns = [
            "union select",
            "drop table",
            "delete from",
            "insert into",
            "update set",
            "exec(",
            "system(",
            "cmd.exe",
            "/bin/sh",
            "wget ",
            "curl ",
        ];

        let content_lower = content.to_lowercase();
        suspicious_patterns.iter().any(|pattern| content_lower.contains(pattern))
    }
}
```

### **Phase 3: Threat Detection**

#### **3.1 Multi-Layer Threat Detector**

```rust
pub struct ThreatDetector {
    threat_patterns: Vec<ThreatPattern>,
    behavior_analyzer: BehaviorAnalyzer,
    anomaly_detector: AnomalyDetector,
    threat_score_threshold: f64,
}

#[derive(Debug, Clone)]
pub struct ThreatPattern {
    pub name: String,
    pub pattern: regex::Regex,
    pub severity: ThreatLevel,
    pub weight: f64,
}

pub struct BehaviorAnalyzer {
    client_behaviors: Arc<Mutex<HashMap<String, ClientBehavior>>>,
    normal_patterns: Vec<BehaviorPattern>,
}

pub struct ClientBehavior {
    pub request_frequency: f64,
    pub average_payload_size: f64,
    pub error_rate: f64,
    pub unique_endpoints: HashSet<String>,
    pub last_seen: Instant,
}

impl ThreatDetector {
    pub fn new() -> Self {
        let mut threat_patterns = Vec::new();

        // SQL Injection patterns
        threat_patterns.push(ThreatPattern {
            name: "SQL Injection".to_string(),
            pattern: regex::Regex::new(r"(?i)(union|select|insert|update|delete|drop|create|alter|exec|execute)").unwrap(),
            severity: ThreatLevel::High,
            weight: 0.8,
        });

        // XSS patterns
        threat_patterns.push(ThreatPattern {
            name: "XSS Attack".to_string(),
            pattern: regex::Regex::new(r"(?i)(<script|javascript:|on\w+\s*=)").unwrap(),
            severity: ThreatLevel::High,
            weight: 0.7,
        });

        // Command Injection patterns
        threat_patterns.push(ThreatPattern {
            name: "Command Injection".to_string(),
            pattern: regex::Regex::new(r"(?i)(;|\||&|\$\(|`|cmd\.exe|/bin/sh)").unwrap(),
            severity: ThreatLevel::Critical,
            weight: 0.9,
        });

        Self {
            threat_patterns,
            behavior_analyzer: BehaviorAnalyzer::new(),
            anomaly_detector: AnomalyDetector::new(),
            threat_score_threshold: 0.7,
        }
    }

    pub fn analyze_request(&mut self, request: &SecurityRequest) -> Result<ThreatLevel, ThreatDetectionError> {
        let mut threat_score = 0.0;
        let mut detected_threats = Vec::new();

        // Pattern-based detection
        for pattern in &self.threat_patterns {
            if pattern.pattern.is_match(&request.payload) {
                threat_score += pattern.weight;
                detected_threats.push(pattern.name.clone());
            }
        }

        // Behavior-based detection
        let behavior_score = self.behavior_analyzer.analyze_behavior(request)?;
        threat_score += behavior_score;

        // Anomaly detection
        let anomaly_score = self.anomaly_detector.detect_anomalies(request)?;
        threat_score += anomaly_score;

        // Determine threat level
        let threat_level = if threat_score >= 0.9 {
            ThreatLevel::Critical
        } else if threat_score >= 0.7 {
            ThreatLevel::High
        } else if threat_score >= 0.5 {
            ThreatLevel::Medium
        } else if threat_score >= 0.3 {
            ThreatLevel::Low
        } else {
            ThreatLevel::None
        };

        // Log threat detection
        if threat_level != ThreatLevel::None {
            self.log_threat_detection(request, threat_level, detected_threats, threat_score);
        }

        Ok(threat_level)
    }

    fn log_threat_detection(
        &self,
        request: &SecurityRequest,
        threat_level: ThreatLevel,
        detected_threats: Vec<String>,
        threat_score: f64,
    ) {
        eprintln!(
            "Threat detected: Level={:?}, Score={:.2}, Threats={:?}, Client={:?}",
            threat_level, threat_score, detected_threats, request.client_id
        );
    }
}
```

### **Phase 4: Authentication System**

#### **4.1 JWT Authentication**

```rust
pub struct Authenticator {
    jwt_secret: Option<String>,
    token_blacklist: Arc<Mutex<HashSet<String>>>,
    session_store: Arc<Mutex<HashMap<String, Session>>>,
}

pub struct Session {
    pub user_id: String,
    pub created_at: Instant,
    pub expires_at: Instant,
    pub permissions: Vec<String>,
}

impl Authenticator {
    pub fn new(jwt_secret: Option<String>) -> Self {
        Self {
            jwt_secret,
            token_blacklist: Arc::new(Mutex::new(HashSet::new())),
            session_store: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn authenticate(&self, auth_token: &str) -> Result<Session, AuthenticationError> {
        // Check if token is blacklisted
        if self.token_blacklist.lock().unwrap().contains(auth_token) {
            return Err(AuthenticationError::TokenBlacklisted);
        }

        // Verify JWT token
        if let Some(secret) = &self.jwt_secret {
            match self.verify_jwt_token(auth_token, secret) {
                Ok(claims) => {
                    // Check if session exists
                    if let Some(session) = self.session_store.lock().unwrap().get(auth_token) {
                        if session.expires_at > Instant::now() {
                            Ok(session.clone())
                        } else {
                            Err(AuthenticationError::TokenExpired)
                        }
                    } else {
                        Err(AuthenticationError::SessionNotFound)
                    }
                }
                Err(e) => Err(AuthenticationError::InvalidToken(e.to_string())),
            }
        } else {
            // No JWT secret configured, use simple token validation
            self.validate_simple_token(auth_token)
        }
    }

    fn verify_jwt_token(&self, token: &str, secret: &str) -> Result<serde_json::Value, jsonwebtoken::errors::Error> {
        use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};

        let key = DecodingKey::from_secret(secret.as_ref());
        let validation = Validation::new(Algorithm::HS256);

        let token_data = decode::<serde_json::Value>(token, &key, &validation)?;
        Ok(token_data.claims)
    }

    fn validate_simple_token(&self, token: &str) -> Result<Session, AuthenticationError> {
        // Simple token validation for development
        if token.starts_with("dev_token_") {
            Ok(Session {
                user_id: "dev_user".to_string(),
                created_at: Instant::now(),
                expires_at: Instant::now() + Duration::from_hours(24),
                permissions: vec!["read".to_string(), "write".to_string()],
            })
        } else {
            Err(AuthenticationError::InvalidToken("Invalid token format".to_string()))
        }
    }

    pub fn create_session(&self, user_id: String, permissions: Vec<String>) -> Result<String, AuthenticationError> {
        let session_id = uuid::Uuid::new_v4().to_string();
        let now = Instant::now();

        let session = Session {
            user_id,
            created_at: now,
            expires_at: now + Duration::from_hours(24),
            permissions,
        };

        self.session_store.lock().unwrap().insert(session_id.clone(), session);
        Ok(session_id)
    }

    pub fn revoke_token(&self, token: &str) {
        self.token_blacklist.lock().unwrap().insert(token.to_string());
        self.session_store.lock().unwrap().remove(token);
    }
}
```

## 🧪 **Testing Strategy**

### **Unit Tests**

- Rate limiting with different algorithms
- Input validation with various payloads
- Threat detection with attack patterns
- Authentication with valid/invalid tokens

### **Security Tests**

- Penetration testing scenarios
- Rate limit bypass attempts
- Input validation bypass attempts
- Authentication bypass attempts

### **Performance Tests**

- Rate limiter performance under load
- Input validator throughput
- Threat detector latency
- Authentication system scalability

## ✅ **Success Criteria**

### **Functionality**

- ✅ Effective rate limiting with multiple algorithms
- ✅ Comprehensive input validation
- ✅ Accurate threat detection
- ✅ Secure authentication system
- ✅ CSRF protection

### **Performance**

- ✅ < 1ms rate limit check
- ✅ < 5ms input validation
- ✅ < 10ms threat detection
- ✅ < 2ms authentication
- ✅ < 1MB memory usage per 1000 clients

### **Security**

- ✅ Blocks common attack patterns
- ✅ Prevents rate limit bypass
- ✅ Detects suspicious behavior
- ✅ Maintains secure sessions
- ✅ Logs security events

## 🚀 **Implementation Timeline**

- **Day 1-2**: Rate limiting implementation
- **Day 3-4**: Input validation system
- **Day 5-6**: Threat detection engine
- **Day 7**: Authentication system
- **Day 8**: Testing and validation

---

**Priority: HIGH - Security is critical for production deployment.**
