use crate::error::WorkflowResult;
use crate::expression::evaluate_expression_str;
use serde_json::Value;
use serverless_workflow_core::models::authentication::ReferenceableAuthenticationPolicy;

type VarsMap = std::collections::HashMap<String, Value>;
type AuthDefs = std::collections::HashMap<String, ReferenceableAuthenticationPolicy>;

/// Digest auth credentials extracted for the two-step flow
pub(crate) struct DigestAuthInfo {
    pub username: String,
    pub password: String,
}

/// Parameters for computing a digest auth response hash per RFC 2617
pub(crate) struct DigestAuthParams<'a> {
    pub username: &'a str,
    pub password: &'a str,
    pub realm: &'a str,
    pub method: &'a str,
    pub uri: &'a str,
    pub nonce: &'a str,
    pub qop: Option<&'a str>,
    pub nc: &'a str,
    pub cnonce: &'a str,
    pub algorithm: &'a str,
}

/// Parameters for building the Authorization: Digest header value
pub(crate) struct DigestHeaderParams<'a> {
    pub username: &'a str,
    pub realm: &'a str,
    pub nonce: &'a str,
    pub uri: &'a str,
    pub response: &'a str,
    pub opaque: Option<&'a str>,
    pub qop: Option<&'a str>,
    pub nc: &'a str,
    pub cnonce: &'a str,
}

/// Parses a WWW-Authenticate: Digest header into its components
pub(crate) struct DigestChallenge {
    pub realm: String,
    pub nonce: String,
    pub opaque: Option<String>,
    pub algorithm: String,
    pub qop: Option<String>,
}

/// Extracts digest authentication info if digest auth is configured.
/// Returns Ok(None) if no digest auth is configured, Ok(Some(info)) if it is,
/// or an error if digest auth is configured but invalid.
pub(crate) fn extract_digest_info(
    policy: &ReferenceableAuthenticationPolicy,
    auth_definitions: Option<&AuthDefs>,
    input: &Value,
    vars: &VarsMap,
    task_name: &str,
) -> WorkflowResult<Option<DigestAuthInfo>> {
    let resolved_policy = match policy {
        ReferenceableAuthenticationPolicy::Policy(def) => def,
        ReferenceableAuthenticationPolicy::Reference(ref_ref) => match auth_definitions {
            Some(defs) => match defs.get(&ref_ref.use_) {
                Some(ReferenceableAuthenticationPolicy::Policy(def)) => def,
                _ => return Ok(None),
            },
            None => return Ok(None),
        },
    };

    if let Some(ref digest) = resolved_policy.digest {
        if let Some(ref username_expr) = digest.username {
            let username = evaluate_expression_str(username_expr, input, vars, task_name)?;
            let password = digest
                .password
                .as_deref()
                .map(|p| evaluate_expression_str(p, input, vars, task_name))
                .transpose()?
                .unwrap_or_default();
            return Ok(Some(DigestAuthInfo { username, password }));
        }
    }

    Ok(None)
}

pub(crate) fn parse_digest_challenge(www_auth: &str) -> Option<DigestChallenge> {
    let header = www_auth.strip_prefix("Digest")?.trim();

    let mut realm = None;
    let mut nonce = None;
    let mut opaque = None;
    let mut algorithm = "MD5".to_string();
    let mut qop = None;

    // Parse key="value" pairs
    let re = regex_lazy();
    for cap in re.captures_iter(header) {
        let key = match cap.get(1) {
            Some(m) => m.as_str(),
            None => continue,
        };
        let value = match cap.get(2) {
            Some(m) => m.as_str(),
            None => continue,
        };
        match key {
            "realm" => realm = Some(value.to_string()),
            "nonce" => nonce = Some(value.to_string()),
            "opaque" => opaque = Some(value.to_string()),
            "algorithm" => algorithm = value.to_string(),
            "qop" => qop = Some(value.to_string()),
            _ => {}
        }
    }

    Some(DigestChallenge {
        realm: realm?,
        nonce: nonce?,
        opaque,
        algorithm,
        qop,
    })
}

/// Lazy static regex for parsing WWW-Authenticate header
fn regex_lazy() -> std::sync::MutexGuard<'static, regex::Regex> {
    use regex::Regex;
    use std::sync::Mutex;
    static RE: std::sync::OnceLock<Mutex<Regex>> = std::sync::OnceLock::new();
    let guard = RE
        .get_or_init(|| {
            Mutex::new(Regex::new(r#"([A-Za-z]+)="([^"]*)""#).expect("static regex is valid"))
        })
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    guard
}

/// Generates a random nonce for digest auth (cnonce)
pub(crate) fn rand_nonce() -> u32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    // Simple XOR-shift PRNG for cnonce generation
    let mut x = seed.wrapping_add(1);
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    (x & 0xFFFFFFFF) as u32
}

/// Computes the MD5 hex digest of a string
pub(crate) fn md5_hex(input: &str) -> String {
    use md5::{Digest, Md5};
    let mut hasher = Md5::new();
    hasher.update(input.as_bytes());
    let result = hasher.finalize();
    result.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Computes the digest auth response hash per RFC 2617
pub(crate) fn compute_digest_response(params: &DigestAuthParams) -> String {
    // HA1 = MD5(username:realm:password)
    let ha1 = md5_hex(&format!(
        "{}:{}:{}",
        params.username, params.realm, params.password
    ));

    // For MD5-sess: HA1 = MD5(HA1:nonce:cnonce)
    let ha1 = if params.algorithm.eq_ignore_ascii_case("MD5-sess") {
        md5_hex(&format!("{}:{}:{}", ha1, params.nonce, params.cnonce))
    } else {
        ha1
    };

    // HA2 = MD5(method:uri)
    let ha2 = md5_hex(&format!("{}:{}", params.method, params.uri));

    // response = MD5(HA1:nonce:nc:cnonce:qop:HA2) or MD5(HA1:nonce:HA2)
    match params.qop {
        Some(qop_val) => md5_hex(&format!(
            "{}:{}:{}:{}:{}:{}",
            ha1, params.nonce, params.nc, params.cnonce, qop_val, ha2
        )),
        None => md5_hex(&format!("{}:{}:{}", ha1, params.nonce, ha2)),
    }
}

/// Builds the Authorization: Digest header value
pub(crate) fn build_digest_auth_header(params: &DigestHeaderParams) -> String {
    let mut parts = vec![
        format!("username=\"{}\"", params.username),
        format!("realm=\"{}\"", params.realm),
        format!("nonce=\"{}\"", params.nonce),
        format!("uri=\"{}\"", params.uri),
        format!("response=\"{}\"", params.response),
    ];

    if let Some(q) = params.qop {
        parts.push(format!("qop={}", q));
        parts.push(format!("nc={}", params.nc));
        parts.push(format!("cnonce=\"{}\"", params.cnonce));
    }

    if let Some(op) = params.opaque {
        parts.push(format!("opaque=\"{}\"", op));
    }

    format!("Digest {}", parts.join(", "))
}
