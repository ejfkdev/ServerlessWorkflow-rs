use super::*;

    #[tokio::test]
    async fn test_runner_call_http_get() {

        let hello = warp::path!("pets" / i32).map(|id| {
            warp::reply::json(&serde_json::json!({
                "id": id,
                "name": "Buddy",
                "status": "available"
            }))
        });

        let output = run_workflow_with_mock_server("call_http_get.yaml", hello, json!({})).await;
        assert_eq!(output["id"], json!(1));
        assert_eq!(output["name"], json!("Buddy"));
    }

    // === HTTP Call: POST ===

    #[tokio::test]
    async fn test_runner_call_http_post() {

        let create_pet = warp::post()
            .and(warp::path("pets"))
            .and(warp::body::json())
            .map(|body: serde_json::Value| {
                let mut response = body.clone();
                response["id"] = json!(42);
                warp::reply::json(&response)
            });

        let output = run_workflow_with_mock_server("call_http_post.yaml", create_pet, json!({"petName": "Rex"})).await;
        assert_eq!(output["name"], json!("Rex"));
        assert_eq!(output["status"], json!("available"));
        assert_eq!(output["id"], json!(42));
    }

    // === HTTP Call: GET with output.as ===

    #[tokio::test]
    async fn test_runner_call_http_get_output_as() {

        let hello = warp::path!("pets" / i32).map(|id| {
            warp::reply::json(&serde_json::json!({
                "id": id,
                "name": "Buddy",
                "status": "available"
            }))
        });

        let output = run_workflow_with_mock_server("call_http_get_output_as.yaml", hello, json!({})).await;
        // output.as: .name should extract just the name
        assert_eq!(output, json!("Buddy"));
    }

    // === HTTP Call: Try-catch 404 ===

    #[tokio::test]
    async fn test_runner_call_http_try_catch_404() {

        let not_found = warp::path("notfound")
            .map(|| warp::reply::with_status("Not Found", warp::http::StatusCode::NOT_FOUND));

        let output = run_workflow_with_mock_server("call_http_try_catch_404.yaml", not_found, json!({})).await;
        assert_eq!(output["recovered"], json!(true));
    }

    // === HTTP Call: PUT ===

    #[tokio::test]
    async fn test_runner_call_http_put() {

        let update_pet = warp::put()
            .and(warp::path!("pets" / i32))
            .and(warp::body::json())
            .map(|_id: i32, body: serde_json::Value| {
                let mut response = body.clone();
                response["updated"] = json!(true);
                warp::reply::json(&response)
            });

        let output = run_workflow_with_mock_server("call_http_put.yaml", update_pet, json!({"newName": "Max"})).await;
        assert_eq!(output["name"], json!("Max"));
        assert_eq!(output["status"], json!("sold"));
        assert_eq!(output["updated"], json!(true));
    }

    // === HTTP Call: DELETE ===

    #[tokio::test]
    async fn test_runner_call_http_delete() {

        let delete_pet = warp::delete()
            .and(warp::path!("pets" / i32))
            .map(|id: i32| {
                warp::reply::json(&serde_json::json!({
                    "deleted": true,
                    "id": id
                }))
            });

        let output = run_workflow_with_mock_server("call_http_delete.yaml", delete_pet, json!({})).await;
        assert_eq!(output["deleted"], json!(true));
        assert_eq!(output["id"], json!(1));
    }

    // === Export with conditional expression ===

    #[tokio::test]
    async fn test_runner_call_http_query_params() {

        let find_pets = warp::get()
            .and(warp::path("pets"))
            .and(warp::query::<HashMap<String, String>>())
            .map(|params: HashMap<String, String>| {
                let status = params.get("status").cloned().unwrap_or_default();
                warp::reply::json(&serde_json::json!({
                    "pets": [{"id": 1, "status": status}],
                    "count": 1
                }))
            });

        let output = run_workflow_with_mock_server("call_http_query_params.yaml", find_pets, json!({"status": "sold"})).await;
        assert_eq!(output["count"], json!(1));
    }

    // === Schedule: after delay ===

    #[tokio::test]
    async fn test_runner_call_http_head() {

        let head_handler = warp::head()
            .and(warp::path!("users" / i32))
            .map(|_id: i32| warp::reply::with_header(warp::reply(), "content-length", "42"));

        let output = run_workflow_with_mock_server("call_http_head.yaml", head_handler, json!({})).await;
        // HEAD returns no body, but should not error
        assert!(output.is_null() || output.is_object() || output.is_string());
    }

    // === HTTP Call: PATCH ===

    #[tokio::test]
    async fn test_runner_call_http_patch() {

        let patch_user = warp::patch()
            .and(warp::path!("users" / i32))
            .and(warp::body::json())
            .map(|_id: i32, body: serde_json::Value| {
                let mut response = body.clone();
                response["patched"] = json!(true);
                warp::reply::json(&response)
            });

        let output = run_workflow_with_mock_server("call_http_patch.yaml", patch_user, json!({"newStatus": "inactive"})).await;
        assert_eq!(output["status"], json!("inactive"));
        assert_eq!(output["patched"], json!(true));
    }

    // === HTTP Call: OPTIONS ===

    #[tokio::test]
    async fn test_runner_call_http_options() {

        let options_handler = warp::options()
            .and(warp::path!("users" / i32))
            .map(|_id: i32| {
                warp::reply::with_header(
                    warp::reply(),
                    "allow",
                    "GET, PATCH, DELETE, OPTIONS, HEAD",
                )
            });

        let output = run_workflow_with_mock_server("call_http_options.yaml", options_handler, json!({})).await;
        // OPTIONS returns headers but likely no JSON body
        assert!(output.is_null() || output.is_object() || output.is_string());
    }

    // === HTTP Call: Endpoint interpolation ===

    #[tokio::test]
    async fn test_runner_call_http_endpoint_interpolation() {

        let get_pet = warp::path!("pets" / i32).map(|id: i32| {
            warp::reply::json(&serde_json::json!({
                "id": id,
                "name": "Rex"
            }))
        });

        let output = run_workflow_with_mock_server("call_http_endpoint_interpolation.yaml", get_pet, json!({"petId": 5})).await;
        assert_eq!(output["id"], json!(5));
        assert_eq!(output["name"], json!("Rex"));
    }

    // === HTTP Call: POST with body expression and output.as ===

    #[tokio::test]
    async fn test_runner_call_http_post_expr() {

        let create_user = warp::post()
            .and(warp::path("users"))
            .and(warp::body::json())
            .map(|body: serde_json::Value| warp::reply::json(&body));

        let output = run_workflow_with_mock_server("call_http_post_expr.yaml", create_user, json!({"name": "John", "surname": "Doe"})).await;
        // output.as: .firstName should extract just the firstName value
        assert_eq!(output, json!("John"));
    }

    // === HTTP Call: redirect false ===

    #[tokio::test]
    async fn test_runner_call_http_redirect_false() {

        let create_user = warp::post()
            .and(warp::path("users"))
            .and(warp::body::json())
            .map(|body: serde_json::Value| warp::reply::json(&body));

        let output = run_workflow_with_mock_server("call_http_redirect_false.yaml", create_user, json!({"firstName": "John", "lastName": "Doe"})).await;
        assert_eq!(output["firstName"], json!("John"));
        assert_eq!(output["lastName"], json!("Doe"));
    }

    // === HTTP Call: redirect true (follow redirects) ===

    #[tokio::test]
    async fn test_runner_call_http_redirect_true() {
        use warp::http::StatusCode;

        let redirect_handler = warp::get().and(warp::path("old-path")).map(|| {
            warp::http::Response::builder()
                .status(StatusCode::PERMANENT_REDIRECT)
                .header("Location", "/new-path")
                .body("".to_string())
                .unwrap()
        });

        let target_handler = warp::get()
            .and(warp::path("new-path"))
            .map(|| warp::reply::json(&serde_json::json!({"redirected": true})));

        let routes = redirect_handler.or(target_handler);
        let output = run_workflow_with_mock_server("call_http_redirect_true.yaml", routes, json!({})).await;
        assert_eq!(output["redirected"], json!(true));
    }

    // === HTTP Call: output response format ===

    #[tokio::test]
    async fn test_runner_call_http_output_response() {

        let get_pet = warp::path!("pets" / i32).map(|id: i32| {
            warp::reply::json(&serde_json::json!({
                "id": id,
                "name": "Buddy"
            }))
        });

        let output = run_workflow_with_mock_server("call_http_output_response.yaml", get_pet, json!({})).await;
        // output:response should return {statusCode, headers, body}
        assert!(output.get("statusCode").is_some());
        assert!(output.get("headers").is_some());
        assert!(output.get("body").is_some());
        assert_eq!(output["statusCode"], json!(200));
        assert_eq!(output["body"]["id"], json!(1));
        assert_eq!(output["body"]["name"], json!("Buddy"));
    }

    // === Listen: to any (stub) ===

    #[tokio::test]
    async fn test_runner_call_http_find_by_status() {

        // Server that returns 404 for the find endpoint
        let not_found = warp::path!("v2" / "pet" / "findByStatus")
            .map(|| warp::reply::with_status("Pet not found", warp::http::StatusCode::NOT_FOUND));

        let output = run_workflow_with_mock_server("call_http_find_by_status.yaml", not_found, json!({})).await;
        // try-catch catches the communication error successfully
        // Output may be null if no explicit recovery task is defined
        let _ = output;
    }

    // === Workflow Output Schema: valid ===

    #[tokio::test]
    async fn test_runner_call_http_with_headers() {

        let headers_echo = warp::path("headers").map(|| {
            // Echo back some data to confirm the request was received
            warp::reply::json(&serde_json::json!({
                "received": true
            }))
        });

        let output = run_workflow_with_mock_server("call_http_with_headers.yaml", headers_echo, json!({})).await;
        assert_eq!(output["received"], json!(true));
    }

    // === Schedule: cron (parse + run once) ===

    #[tokio::test]
    async fn test_runner_call_http_basic_auth() {
        use crate::secret::MapSecretManager;

        // Server that echoes back the Authorization header for verification
        let protected = warp::path("protected")
            .and(warp::header::optional("Authorization"))
            .map(|auth: Option<String>| {
                match auth {
                    Some(val) if val.starts_with("Basic ") => {
                        // "admin:secret123" base64-encoded is "YWRtaW46c2VjcmV0MTIz"
                        if val == "Basic YWRtaW46c2VjcmV0MTIz" {
                            warp::reply::json(&serde_json::json!({"access": "granted"}))
                        } else {
                            warp::reply::json(&serde_json::json!({"access": "denied", "got": val}))
                        }
                    }
                    _ => {
                        warp::reply::json(&serde_json::json!({"access": "denied", "auth": "none"}))
                    }
                }
            });

        let port = start_mock_server(protected);

        let secret_mgr = Arc::new(MapSecretManager::new().with_secret(
            "mySecret",
            json!({
                "username": "admin",
                "password": "secret123"
            }),
        ));

        let yaml_str = std::fs::read_to_string(testdata("call_http_basic_auth.yaml")).unwrap();
        let yaml_str = yaml_str.replace("9876", &port.to_string());
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow)
            .unwrap()
            .with_secret_manager(secret_mgr);

        let output = runner.run(json!({})).await.unwrap();
        assert_eq!(output["access"], json!("granted"));
    }

    // === HTTP Call: Basic Auth with basic.use secret reference + $authorization export ===

    #[tokio::test]
    async fn test_runner_call_http_basic_secret_auth() {
        use crate::secret::MapSecretManager;

        // Server that checks Basic auth and returns JSON
        let protected = warp::path("protected")
            .and(warp::header::optional("Authorization"))
            .map(|auth: Option<String>| match auth {
                Some(val) if val == "Basic YWRtaW46c2VjcmV0MTIz" => {
                    warp::reply::json(&serde_json::json!({"access": "granted"}))
                }
                _ => warp::reply::json(&serde_json::json!({"access": "denied"})),
            });

        let port = start_mock_server(protected);

        let secret_mgr = Arc::new(MapSecretManager::new().with_secret(
            "mySecret",
            json!({
                "username": "admin",
                "password": "secret123"
            }),
        ));

        let yaml_str =
            std::fs::read_to_string(testdata("call_http_basic_secret_auth.yaml")).unwrap();
        let yaml_str = yaml_str.replace("9876", &port.to_string());
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow)
            .unwrap()
            .with_secret_manager(secret_mgr);

        let output = runner.run(json!({})).await.unwrap();
        assert_eq!(output["access"], json!("granted"));
    }

    // === HTTP Call: Bearer Auth with bearer.use secret reference ===

    #[tokio::test]
    async fn test_runner_call_http_bearer_secret_auth() {
        use crate::secret::MapSecretManager;

        // Server that checks Bearer auth
        let protected = warp::path("api")
            .and(warp::path("data"))
            .and(warp::header::optional("Authorization"))
            .map(|auth: Option<String>| match auth {
                Some(val) if val == "Bearer my-secret-token-123" => {
                    warp::reply::json(&serde_json::json!({"status": "ok"}))
                }
                _ => warp::reply::json(&serde_json::json!({"status": "denied"})),
            });

        let port = start_mock_server(protected);

        let secret_mgr = Arc::new(MapSecretManager::new().with_secret(
            "apiToken",
            json!({
                "token": "my-secret-token-123"
            }),
        ));

        let yaml_str =
            std::fs::read_to_string(testdata("call_http_bearer_secret_auth.yaml")).unwrap();
        let yaml_str = yaml_str.replace("9876", &port.to_string());
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow)
            .unwrap()
            .with_secret_manager(secret_mgr);

        let output = runner.run(json!({})).await.unwrap();
        assert_eq!(output["status"], json!("ok"));
    }

    // === HTTP Call: Basic Auth with secret + $authorization export.as ===

    #[tokio::test]
    async fn test_runner_call_http_auth_export() {
        use crate::secret::MapSecretManager;

        let protected = warp::path("protected")
            .and(warp::header::optional("Authorization"))
            .map(|auth: Option<String>| match auth {
                Some(val) if val == "Basic YWRtaW46c2VjcmV0MTIz" => {
                    warp::reply::json(&serde_json::json!({"access": "granted"}))
                }
                _ => warp::reply::json(&serde_json::json!({"access": "denied"})),
            });

        let port = start_mock_server(protected);

        let secret_mgr = Arc::new(MapSecretManager::new().with_secret(
            "mySecret",
            json!({
                "username": "admin",
                "password": "secret123"
            }),
        ));

        let yaml_str = std::fs::read_to_string(testdata("call_http_auth_export.yaml")).unwrap();
        let yaml_str = yaml_str.replace("9876", &port.to_string());
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow)
            .unwrap()
            .with_secret_manager(secret_mgr);

        let output = runner.run(json!({})).await.unwrap();
        // Verify HTTP call succeeded and export.as with $authorization works
        assert_eq!(output["authScheme"], json!("Basic"));
        // Parameter is Base64-encoded credentials, not plaintext
        assert_eq!(output["authParam"], json!(base64::engine::general_purpose::STANDARD.encode("admin:secret123")));
    }

    // === Call HTTP: authentication reference (use.authentications) ===

    #[tokio::test]
    async fn test_runner_call_http_auth_reference() {

        let protected = warp::path("protected")
            .and(warp::header::optional("Authorization"))
            .map(|auth: Option<String>| match auth {
                Some(val) if val == "Basic YWRtaW46c2VjcmV0MTIz" => {
                    warp::reply::json(&serde_json::json!({"access": "granted"}))
                }
                _ => warp::reply::json(&serde_json::json!({"access": "denied"})),
            });

        let output = run_workflow_with_mock_server("call_http_auth_reference.yaml", protected, json!({})).await;
        assert_eq!(output["access"], json!("granted"));
    }

    // === Call: HTTP Digest Auth ===

    #[tokio::test]
    async fn test_runner_call_http_digest_auth() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;
        use warp::Reply;

        let request_was_digest = Arc::new(AtomicBool::new(false));
        let request_was_digest_clone = request_was_digest.clone();

        let digest_endpoint = warp::path("digest-protected")
            .and(warp::header::optional("Authorization"))
            .map(move |auth: Option<String>| {
                match auth {
                    Some(val) if val.starts_with("Digest") => {
                        request_was_digest_clone.store(true, Ordering::SeqCst);
                        warp::reply::json(&serde_json::json!({"access": "granted"})).into_response()
                    }
                    _ => {
                        // Return 401 with Digest challenge
                        let body = warp::reply::json(&serde_json::json!({"error": "unauthorized"}));
                        let mut response = warp::reply::with_status(body, warp::http::StatusCode::UNAUTHORIZED).into_response();
                        response.headers_mut().insert(
                            "WWW-Authenticate",
                            warp::http::HeaderValue::from_static(
                                r#"Digest realm="testrealm", nonce="dcd98b7102dd2f0e8b11d0f600bfb0c093", qop="auth""#,
                            ),
                        );
                        response
                    }
                }
            });

        let port = start_mock_server(digest_endpoint);

        let yaml_str = std::fs::read_to_string(testdata("call_http_digest.yaml")).unwrap();
        let yaml_str = yaml_str.replace("9876", &port.to_string());
        let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
        assert_eq!(output["access"], json!("granted"));
        assert!(
            request_was_digest.load(Ordering::SeqCst),
            "Expected Digest auth header to be sent"
        );
    }

    // === HTTP Call: OAuth2 client_credentials ===

    #[tokio::test]
    async fn test_runner_call_http_oauth2_client_credentials() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;
        use warp::Reply;

        let token_issued = Arc::new(AtomicBool::new(false));
        let token_issued_clone = token_issued.clone();

        // Token endpoint: accepts POST with grant_type=client_credentials, returns access_token
        let token_endpoint = warp::path("oauth2")
            .and(warp::path("token"))
            .and(warp::post())
            .and(warp::body::form::<std::collections::HashMap<String, String>>())
            .map(move |params: std::collections::HashMap<String, String>| {
                let grant_type = params.get("grant_type").map(|s| s.as_str()).unwrap_or("");
                let client_id = params.get("client_id").map(|s| s.as_str()).unwrap_or("");
                let client_secret = params
                    .get("client_secret")
                    .map(|s| s.as_str())
                    .unwrap_or("");
                let scope = params.get("scope").map(|s| s.as_str()).unwrap_or("");

                if grant_type == "client_credentials"
                    && client_id == "test-client"
                    && client_secret == "test-secret"
                {
                    token_issued_clone.store(true, Ordering::SeqCst);
                    warp::reply::json(&serde_json::json!({
                        "access_token": "test-access-token-123",
                        "token_type": "Bearer",
                        "expires_in": 3600,
                        "scope": scope
                    }))
                } else {
                    warp::reply::json(&serde_json::json!({
                        "error": "invalid_client"
                    }))
                }
            });

        // Protected endpoint: requires Bearer token
        let protected_endpoint = warp::path("protected")
            .and(warp::header::optional("Authorization"))
            .map(|auth: Option<String>| match auth {
                Some(val) if val == "Bearer test-access-token-123" => {
                    warp::reply::json(&serde_json::json!({"data": "secret", "authenticated": true}))
                        .into_response()
                }
                _ => warp::reply::with_status(
                    warp::reply::json(&serde_json::json!({"error": "unauthorized"})),
                    warp::http::StatusCode::UNAUTHORIZED,
                )
                .into_response(),
            });

        let routes = token_endpoint.or(protected_endpoint);
        let port = start_mock_server(routes);

        let yaml_str = std::fs::read_to_string(testdata("call_http_oauth2.yaml")).unwrap();
        let yaml_str = yaml_str.replace("9876", &port.to_string());
        let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
        assert_eq!(output["data"], json!("secret"));
        assert_eq!(output["authenticated"], json!(true));
        assert!(
            token_issued.load(Ordering::SeqCst),
            "Expected OAuth2 token to be fetched"
        );
    }

    // === HTTP Call: OAuth2 password grant ===

    #[tokio::test]
    async fn test_runner_call_http_oauth2_password_grant() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;
        use warp::Reply;

        let token_issued = Arc::new(AtomicBool::new(false));
        let token_issued_clone = token_issued.clone();

        // Token endpoint: accepts POST with grant_type=password
        let token_endpoint = warp::path("oauth2")
            .and(warp::path("token"))
            .and(warp::post())
            .and(warp::body::form::<std::collections::HashMap<String, String>>())
            .map(move |params: std::collections::HashMap<String, String>| {
                let grant_type = params.get("grant_type").map(|s| s.as_str()).unwrap_or("");
                let username = params.get("username").map(|s| s.as_str()).unwrap_or("");
                let password = params.get("password").map(|s| s.as_str()).unwrap_or("");

                if grant_type == "password" && username == "testuser" && password == "testpass" {
                    token_issued_clone.store(true, Ordering::SeqCst);
                    warp::reply::json(&serde_json::json!({
                        "access_token": "password-grant-token-456",
                        "token_type": "Bearer"
                    }))
                } else {
                    warp::reply::json(&serde_json::json!({"error": "invalid_grant"}))
                }
            });

        // Protected endpoint
        let protected_endpoint = warp::path("protected")
            .and(warp::header::optional("Authorization"))
            .map(|auth: Option<String>| match auth {
                Some(val) if val == "Bearer password-grant-token-456" => {
                    warp::reply::json(&serde_json::json!({"status": "ok"})).into_response()
                }
                _ => warp::reply::with_status(
                    warp::reply::json(&serde_json::json!({"error": "unauthorized"})),
                    warp::http::StatusCode::UNAUTHORIZED,
                )
                .into_response(),
            });

        let routes = token_endpoint.or(protected_endpoint);
        let port = start_mock_server(routes);

        let yaml = format!(
            r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: oauth2-password
  version: '0.1.0'
do:
  - getProtected:
      call: http
      with:
        method: get
        endpoint:
          uri: http://localhost:{port}/protected
          authentication:
            oauth2:
              authority: http://localhost:{port}
              grant: password
              username: testuser
              password: testpass
"#
        );
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();

        let output = runner.run(json!({})).await.unwrap();
        assert_eq!(output["status"], json!("ok"));
        assert!(
            token_issued.load(Ordering::SeqCst),
            "Expected OAuth2 password grant token to be fetched"
        );
    }

    // === HTTP Call: OAuth2 client_secret_basic auth method ===

    #[tokio::test]
    async fn test_runner_call_http_oauth2_client_secret_basic() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;
        use warp::Reply;

        let token_issued = Arc::new(AtomicBool::new(false));
        let token_issued_clone = token_issued.clone();

        // Token endpoint: checks for Basic auth header with client_id:client_secret
        let token_endpoint = warp::path("oauth2")
            .and(warp::path("token"))
            .and(warp::post())
            .and(warp::header::optional("Authorization"))
            .and(warp::body::form::<std::collections::HashMap<String, String>>())
            .map(
                move |auth: Option<String>, params: std::collections::HashMap<String, String>| {
                    let grant_type = params.get("grant_type").map(|s| s.as_str()).unwrap_or("");
                    let has_basic_auth = auth.map(|a| a.starts_with("Basic")).unwrap_or(false);

                    if grant_type == "client_credentials" && has_basic_auth {
                        token_issued_clone.store(true, Ordering::SeqCst);
                        warp::reply::json(&serde_json::json!({
                            "access_token": "basic-auth-token-789",
                            "token_type": "Bearer"
                        }))
                    } else {
                        warp::reply::json(&serde_json::json!({"error": "invalid_client"}))
                    }
                },
            );

        let protected_endpoint = warp::path("protected")
            .and(warp::header::optional("Authorization"))
            .map(|auth: Option<String>| match auth {
                Some(val) if val == "Bearer basic-auth-token-789" => {
                    warp::reply::json(&serde_json::json!({"status": "ok"})).into_response()
                }
                _ => warp::reply::with_status(
                    warp::reply::json(&serde_json::json!({"error": "unauthorized"})),
                    warp::http::StatusCode::UNAUTHORIZED,
                )
                .into_response(),
            });

        let routes = token_endpoint.or(protected_endpoint);
        let port = start_mock_server(routes);

        let yaml = format!(
            r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: oauth2-client-secret-basic
  version: '0.1.0'
do:
  - getProtected:
      call: http
      with:
        method: get
        endpoint:
          uri: http://localhost:{port}/protected
          authentication:
            oauth2:
              authority: http://localhost:{port}
              grant: client_credentials
              client:
                id: test-client
                secret: test-secret
                authentication: client_secret_basic
"#
        );
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();

        let output = runner.run(json!({})).await.unwrap();
        assert_eq!(output["status"], json!("ok"));
        assert!(
            token_issued.load(Ordering::SeqCst),
            "Expected OAuth2 token with client_secret_basic auth"
        );
    }

    // === HTTP Call: OAuth2 JSON encoding ===

    #[tokio::test]
    async fn test_runner_call_http_oauth2_json_encoding() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;
        use warp::Reply;

        let token_issued = Arc::new(AtomicBool::new(false));
        let token_issued_clone = token_issued.clone();

        // Token endpoint: accepts JSON body
        let token_endpoint = warp::path("oauth2")
            .and(warp::path("token"))
            .and(warp::post())
            .and(warp::body::json::<std::collections::HashMap<String, String>>())
            .map(move |params: std::collections::HashMap<String, String>| {
                let grant_type = params.get("grant_type").map(|s| s.as_str()).unwrap_or("");
                let client_id = params.get("client_id").map(|s| s.as_str()).unwrap_or("");

                if grant_type == "client_credentials" && client_id == "json-client" {
                    token_issued_clone.store(true, Ordering::SeqCst);
                    warp::reply::json(&serde_json::json!({
                        "access_token": "json-token-abc",
                        "token_type": "Bearer"
                    }))
                } else {
                    warp::reply::json(&serde_json::json!({"error": "invalid_client"}))
                }
            });

        let protected_endpoint = warp::path("protected")
            .and(warp::header::optional("Authorization"))
            .map(|auth: Option<String>| match auth {
                Some(val) if val == "Bearer json-token-abc" => {
                    warp::reply::json(&serde_json::json!({"encoding": "json"})).into_response()
                }
                _ => warp::reply::with_status(
                    warp::reply::json(&serde_json::json!({"error": "unauthorized"})),
                    warp::http::StatusCode::UNAUTHORIZED,
                )
                .into_response(),
            });

        let routes = token_endpoint.or(protected_endpoint);
        let port = start_mock_server(routes);

        let yaml = format!(
            r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: oauth2-json-encoding
  version: '0.1.0'
do:
  - getProtected:
      call: http
      with:
        method: get
        endpoint:
          uri: http://localhost:{port}/protected
          authentication:
            oauth2:
              authority: http://localhost:{port}
              grant: client_credentials
              request:
                encoding: application/json
              client:
                id: json-client
                secret: json-secret
"#
        );
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();

        let output = runner.run(json!({})).await.unwrap();
        assert_eq!(output["encoding"], json!("json"));
        assert!(
            token_issued.load(Ordering::SeqCst),
            "Expected OAuth2 token with JSON encoding"
        );
    }

    // === HTTP Call: OAuth2 issuer validation ===

    #[tokio::test]
    async fn test_runner_call_http_oauth2_issuer_validation_rejects() {

        // Token endpoint returning a token with wrong issuer
        let token_endpoint = warp::path("oauth2")
            .and(warp::path("token"))
            .and(warp::post())
            .and(warp::body::form::<std::collections::HashMap<String, String>>())
            .map(|params: std::collections::HashMap<String, String>| {
                let _ = params;
                warp::reply::json(&serde_json::json!({
                    "access_token": "bad-issuer-token",
                    "token_type": "Bearer",
                    "iss": "https://evil-issuer.com"
                }))
            });

        let port = start_mock_server(token_endpoint);

        let yaml = format!(
            r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: oauth2-issuer-validation
  version: '0.1.0'
do:
  - getProtected:
      call: http
      with:
        method: get
        endpoint:
          uri: http://localhost:{port}/protected
          authentication:
            oauth2:
              authority: http://localhost:{port}
              grant: client_credentials
              client:
                id: test-client
                secret: test-secret
              issuers:
                - https://trusted-issuer.com
"#
        );
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();

        let result = runner.run(json!({})).await;
        assert!(result.is_err(), "Expected error due to issuer validation");
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("issuer") || err.contains("not in allowed"),
            "Expected issuer validation error, got: {}",
            err
        );
    }

    // === HTTP Call: redirect false ===

    #[tokio::test]
    async fn test_runner_call_http_redirect_no_follow() {
        use warp::Reply;

        // /old returns 302 redirect to /new
        let redirect_endpoint = warp::path("old").and(warp::get()).map(|| {
            let mut response = warp::reply::with_status(
                warp::reply::json(&serde_json::json!({"message": "redirect"})),
                warp::http::StatusCode::from_u16(302).unwrap(),
            )
            .into_response();
            response
                .headers_mut()
                .insert("location", warp::http::HeaderValue::from_static("/new"));
            response
        });

        let target_endpoint = warp::path("new")
            .and(warp::get())
            .map(|| warp::reply::json(&serde_json::json!({"message": "target"})));

        let routes = redirect_endpoint.or(target_endpoint);
        let port = start_mock_server(routes);

        // With redirect:false, the 302 response is returned as-is (not followed)
        // Since 302 is not a client/server error, it's returned as the output
        let yaml = format!(
            r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: call-http-redirect-false
  version: '0.1.0'
do:
  - getOld:
      call: http
      with:
        method: get
        redirect: false
        endpoint:
          uri: http://localhost:{port}/old
        output: response
"#
        );
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();

        let output = runner.run(json!({})).await.unwrap();
        // With redirect:false, we get the 302 response, not the redirect target
        assert_eq!(output["statusCode"], json!(302));
        // Should NOT have followed the redirect to get "target" message
        assert_ne!(output["body"]["message"], json!("target"));
    }

    // === Try-Catch-Retry: inline exponential backoff ===

    #[tokio::test]
    async fn test_runner_call_http_oidc_client_credentials() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;
        use warp::Reply;

        let token_issued = Arc::new(AtomicBool::new(false));
        let token_issued_clone = token_issued.clone();

        // OIDC token endpoint: the authority IS the full token URL (no /oauth2/token path appended)
        let token_endpoint = warp::path("realms")
            .and(warp::path("test"))
            .and(warp::path("token"))
            .and(warp::post())
            .and(warp::body::form::<std::collections::HashMap<String, String>>())
            .map(move |params: std::collections::HashMap<String, String>| {
                let grant_type = params.get("grant_type").map(|s| s.as_str()).unwrap_or("");
                let client_id = params.get("client_id").map(|s| s.as_str()).unwrap_or("");

                if grant_type == "client_credentials" && client_id == "oidc-client" {
                    token_issued_clone.store(true, Ordering::SeqCst);
                    warp::reply::json(&serde_json::json!({
                        "access_token": "oidc-token-xyz",
                        "token_type": "Bearer",
                        "iss": "http://localhost/realms/test"
                    }))
                } else {
                    warp::reply::json(&serde_json::json!({"error": "invalid_client"}))
                }
            });

        let protected_endpoint = warp::path("protected")
            .and(warp::header::optional("Authorization"))
            .map(|auth: Option<String>| match auth {
                Some(val) if val == "Bearer oidc-token-xyz" => warp::reply::json(
                    &serde_json::json!({"data": "oidc-protected", "authenticated": true}),
                )
                .into_response(),
                _ => warp::reply::with_status(
                    warp::reply::json(&serde_json::json!({"error": "unauthorized"})),
                    warp::http::StatusCode::UNAUTHORIZED,
                )
                .into_response(),
            });

        let routes = token_endpoint.or(protected_endpoint);
        let port = start_mock_server(routes);

        // OIDC authority is the full token endpoint URL
        let yaml = format!(
            r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: oidc-client-credentials
  version: '0.1.0'
do:
  - getProtected:
      call: http
      with:
        method: get
        endpoint:
          uri: http://localhost:{port}/protected
          authentication:
            oidc:
              authority: http://localhost:{port}/realms/test/token
              grant: client_credentials
              client:
                id: oidc-client
                secret: oidc-secret
              issuers:
                - http://localhost/realms/test
"#
        );
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();

        let output = runner.run(json!({})).await.unwrap();
        assert_eq!(output["data"], json!("oidc-protected"));
        assert_eq!(output["authenticated"], json!(true));
        assert!(
            token_issued.load(Ordering::SeqCst),
            "Expected OIDC token to be fetched"
        );
    }

    // === HTTP Call: OAuth2 no endpoints (default /oauth2/token path) ===

    #[tokio::test]
    async fn test_runner_call_http_oauth2_no_endpoints() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;
        use warp::Reply;

        let token_issued = Arc::new(AtomicBool::new(false));
        let token_issued_clone = token_issued.clone();

        // Token endpoint at default /oauth2/token path
        let token_endpoint = warp::path("oauth2")
            .and(warp::path("token"))
            .and(warp::post())
            .and(warp::body::form::<std::collections::HashMap<String, String>>())
            .map(move |params: std::collections::HashMap<String, String>| {
                let grant_type = params.get("grant_type").map(|s| s.as_str()).unwrap_or("");
                if grant_type == "client_credentials" {
                    token_issued_clone.store(true, Ordering::SeqCst);
                    warp::reply::json(&serde_json::json!({
                        "access_token": "default-path-token",
                        "token_type": "Bearer"
                    }))
                } else {
                    warp::reply::json(&serde_json::json!({"error": "invalid_grant"}))
                }
            });

        let protected_endpoint = warp::path("protected")
            .and(warp::header::optional("Authorization"))
            .map(|auth: Option<String>| match auth {
                Some(val) if val == "Bearer default-path-token" => {
                    warp::reply::json(&serde_json::json!({"result": "ok"})).into_response()
                }
                _ => warp::reply::with_status(
                    warp::reply::json(&serde_json::json!({"error": "unauthorized"})),
                    warp::http::StatusCode::UNAUTHORIZED,
                )
                .into_response(),
            });

        let routes = token_endpoint.or(protected_endpoint);
        let port = start_mock_server(routes);

        // No endpoints config — should use default /oauth2/token
        let yaml = format!(
            r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: oauth2-no-endpoints
  version: '0.1.0'
do:
  - getProtected:
      call: http
      with:
        method: get
        endpoint:
          uri: http://localhost:{port}/protected
          authentication:
            oauth2:
              authority: http://localhost:{port}
              grant: client_credentials
              client:
                id: test-client
                secret: test-secret
"#
        );
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();

        let output = runner.run(json!({})).await.unwrap();
        assert_eq!(output["result"], json!("ok"));
        assert!(
            token_issued.load(Ordering::SeqCst),
            "Expected OAuth2 token with default /oauth2/token path"
        );
    }

    // === HTTP Call: OAuth2 with expression parameters ===

    #[tokio::test]
    async fn test_runner_call_http_bearer_auth() {
        use crate::secret::MapSecretManager;

        // Server that checks bearer token
        let api_data = warp::path("api")
            .and(warp::path("data"))
            .and(warp::header::optional("Authorization"))
            .map(|auth: Option<String>| match auth {
                Some(val) if val == "Bearer my-secret-token-123" => {
                    warp::reply::json(&serde_json::json!({"status": "ok", "data": [1, 2, 3]}))
                }
                _ => warp::reply::json(&serde_json::json!({"status": "unauthorized"})),
            });

        let port = start_mock_server(api_data);

        let secret_mgr = Arc::new(MapSecretManager::new().with_secret(
            "apiToken",
            json!({
                "token": "my-secret-token-123"
            }),
        ));

        let yaml_str = std::fs::read_to_string(testdata("call_http_bearer_auth.yaml")).unwrap();
        let yaml_str = yaml_str.replace("9876", &port.to_string());
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow)
            .unwrap()
            .with_secret_manager(secret_mgr);

        let output = runner.run(json!({})).await.unwrap();
        assert_eq!(output["status"], json!("ok"));
    }

    // === For Loop: custom each and at variables ===

    #[tokio::test]
    async fn test_runner_call_http_output_response_as() {

        let get_pet = warp::path!("pets" / i32).map(|id: i32| {
            warp::reply::json(&serde_json::json!({
                "id": id,
                "name": "Buddy"
            }))
        });

        let output = run_workflow_with_mock_server("call_http_output_response_as.yaml", get_pet, json!({})).await;
        // output.as: .statusCode should extract just the status code
        assert_eq!(output, json!(200));
    }

    // === Call function with output.as ===

    #[tokio::test]
    async fn test_runner_call_http_404_in_try() {

        let not_found = warp::path("missing")
            .map(|| warp::reply::with_status("Not Found", warp::http::StatusCode::NOT_FOUND));

        let port = start_mock_server(not_found);

        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: http-404-try
  version: '0.1.0'
do:
  - safeCall:
      try:
        - callMissing:
            call: http
            with:
              method: get
              endpoint:
                uri: http://localhost:PORT/missing
      catch:
        errors:
          with:
            type: communication
        as: err
        do:
          - logError:
              set:
                errorStatus: ${ $err.status }
"#
        .replace("PORT", &port.to_string());
        let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
        assert_eq!(output["errorStatus"], json!(404));
    }

    // === Fork: non-compete with branch names in output ===

    #[tokio::test]
    async fn test_runner_call_http_post_body_expr_output_as() {

        let create_user = warp::post()
            .and(warp::path("users"))
            .and(warp::body::json())
            .map(|body: serde_json::Value| {
                let mut response = body.clone();
                response["id"] = json!(42);
                warp::reply::json(&response)
            });

        let port = start_mock_server(create_user);

        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: call-http-post-body-expr-output-as
  version: '0.1.0'
do:
  - postUser:
      call: http
      with:
        method: post
        endpoint:
          uri: http://localhost:PORT/users
        body: "${ {firstName: .name, lastName: .surname} }"
      output:
        as: .firstName
"#
        .replace("PORT", &port.to_string());
        let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

        let output = runner
            .run(json!({"name": "John", "surname": "Doe"}))
            .await
            .unwrap();
        assert_eq!(output, json!("John"));
    }

    // === HTTP Call: response output with output.as extracting body ===

    #[tokio::test]
    async fn test_runner_call_http_response_output_as_body() {

        let get_pet = warp::path!("pets" / i32).map(|id: i32| {
            warp::reply::json(&serde_json::json!({
                "id": id,
                "name": "Buddy",
                "status": "available"
            }))
        });

        let port = start_mock_server(get_pet);

        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: call-http-response-body
  version: '0.1.0'
do:
  - getPetBody:
      call: http
      with:
        method: get
        output: response
        endpoint:
          uri: http://localhost:PORT/pets/1
      output:
        as: .body
"#
        .replace("PORT", &port.to_string());
        let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
        assert_eq!(output["id"], json!(1));
        assert_eq!(output["name"], json!("Buddy"));
    }

    // === For loop: sum with output.as extracting counter ===

    #[tokio::test]
    async fn test_runner_call_http_post_json_body() {

        let echo = warp::post()
            .and(warp::path("data"))
            .and(warp::body::json())
            .map(|body: serde_json::Value| warp::reply::json(&body));

        let port = start_mock_server(echo);

        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: call-http-post-json-body
  version: '0.1.0'
do:
  - postData:
      call: http
      with:
        method: post
        endpoint:
          uri: http://localhost:PORT/data
        body: "${ {firstName: .first, lastName: .last, fullName: (.first + \" \" + .last)} }"
"#
        .replace("PORT", &port.to_string());
        let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

        let output = runner
            .run(json!({"first": "Jane", "last": "Smith"}))
            .await
            .unwrap();
        assert_eq!(output["firstName"], json!("Jane"));
        assert_eq!(output["lastName"], json!("Smith"));
        assert_eq!(output["fullName"], json!("Jane Smith"));
    }

    // === Emit with structured event data ===

    #[tokio::test]
    async fn test_runner_call_http_response_output_as_headers() {

        let get_pet = warp::path("pets")
            .and(warp::path::param::<i32>())
            .map(|id: i32| {
                warp::reply::with_header(
                    warp::reply::json(&serde_json::json!({"id": id, "name": "Rex"})),
                    "x-custom-header",
                    "custom-value",
                )
            });

        let port = start_mock_server(get_pet);

        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: call-http-response-headers
  version: '0.1.0'
do:
  - getPet:
      call: http
      with:
        method: get
        output: response
        endpoint:
          uri: http://localhost:PORT/pets/1
      output:
        as: "${ {status: .statusCode, petName: .body.name} }"
"#
        .replace("PORT", &port.to_string());
        let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
        assert_eq!(output["status"], json!(200));
        assert_eq!(output["petName"], json!("Rex"));
    }

    // === Nested for loop with outer variable reference ===

    #[tokio::test]
    async fn test_runner_secret_nested_access() {
        // Matches Java SDK's secret-expression.yaml pattern
        // Uses nested $secret.superman.name, $secret.superman.enemy.name, $secret.superman.enemy.isHuman
        use crate::secret::SecretManager;
        struct TestSecretManager;
        #[async_trait::async_trait]
        impl SecretManager for TestSecretManager {
            fn get_secret(&self, key: &str) -> Option<Value> {
                match key {
                    "mySecret" => Some(json!({
                        "superman": {
                            "name": "Clark Kent",
                            "enemy": {
                                "name": "Lex Luthor",
                                "isHuman": true
                            }
                        }
                    })),
                    _ => None,
                }
            }

            fn get_all_secrets(&self) -> Value {
                json!({
                    "mySecret": {
                        "superman": {
                            "name": "Clark Kent",
                            "enemy": {
                                "name": "Lex Luthor",
                                "isHuman": true
                            }
                        }
                    }
                })
            }
        }
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: secret-expression
  version: '0.1.0'
use:
  secrets:
    - mySecret
do:
  - useExpression:
      set:
        superSecret: ${$secret.mySecret.superman.name}
        theEnemy: ${$secret.mySecret.superman.enemy.name}
        humanEnemy: ${$secret.mySecret.superman.enemy.isHuman}
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow)
            .unwrap()
            .with_secret_manager(std::sync::Arc::new(TestSecretManager));
        let output = runner.run(json!({})).await.unwrap();
        assert_eq!(output["superSecret"], json!("Clark Kent"));
        assert_eq!(output["theEnemy"], json!("Lex Luthor"));
        assert_eq!(output["humanEnemy"], json!(true));
    }

    // === Batch 18: Java/Go SDK pattern alignment ===

    #[tokio::test]
    async fn test_runner_call_http_response_output_as_status() {

        let post_user = warp::path("users")
            .and(warp::post())
            .and(warp::body::json())
            .map(|body: serde_json::Value| warp::reply::json(&body));

        let port = start_mock_server(post_user);

        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: http-call-with-response-output-expr
  version: '0.1.0'
do:
  - postPet:
      call: http
      with:
        redirect: true
        headers:
          content-type: application/json
        method: post
        output: response
        endpoint:
          uri: http://localhost:PORT/users
        body: "${{firstName: .firstName, lastName: .lastName, id: .id, bookId: .bookId}}"
      output:
        as: .statusCode
"#
        .replace("PORT", &port.to_string());
        let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

        let output = runner
            .run(json!({
                "firstName": "John",
                "lastName": "Doe",
                "id": 1,
                "bookId": 42
            }))
            .await
            .unwrap();
        // output.as: .statusCode extracts the status code from the full response object
        assert_eq!(output, json!(200));
    }

    // === Set: complex nested structures with static and dynamic values ===
    // Matches Go SDK's TestSetTaskExecutor_ComplexNestedStructures

    #[tokio::test]
    async fn test_runner_secret_missing_error() {
        // Java SDK's SecretExpressionTest.testMissing - missing secret should cause error
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: secret-missing
  version: '0.1.0'
do:
  - useSecret:
      set:
        value: '${ $secret.mySecret.name }'
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        // No secret manager set - should fail
        let runner = WorkflowRunner::new(workflow).unwrap();
        let result = runner.run(json!({})).await;
        assert!(result.is_err(), "Missing secret should cause error");
    }

    // === Custom secret manager with nested access ===

    #[tokio::test]
    async fn test_runner_secret_custom_manager_nested() {
        // Java SDK's SecretExpressionTest.testCustom - custom secret manager with nested access
        use crate::secret::MapSecretManager;
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: secret-custom-nested
  version: '0.1.0'
do:
  - useSecret:
      set:
        superSecret: '${ $secret.mySecret.name }'
        theEnemy: '${ $secret.mySecret.enemy.name }'
        humanEnemy: '${ $secret.mySecret.enemy.isHuman }'
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let secret_mgr = MapSecretManager::new().with_secret(
            "mySecret",
            json!({
                "name": "ClarkKent",
                "enemy": {
                    "name": "Lex Luthor",
                    "isHuman": true
                }
            }),
        );
        let runner = WorkflowRunner::new(workflow)
            .unwrap()
            .with_secret_manager(std::sync::Arc::new(secret_mgr));
        let output = runner.run(json!({})).await.unwrap();
        assert_eq!(output["superSecret"], json!("ClarkKent"));
        assert_eq!(output["theEnemy"], json!("Lex Luthor"));
        assert_eq!(output["humanEnemy"], json!(true));
    }

    // === For loop with at (index) and output collection ===

    #[tokio::test]
    async fn test_e2e_secret_http_basic_auth() {
        use warp::Reply;

        // Endpoint that checks Authorization header
        let protected = warp::path("api")
            .and(warp::path("secure"))
            .and(warp::header::optional("Authorization"))
            .map(|auth: Option<String>| match auth {
                Some(header) if header == "Basic YWRtaW46c2VjcmV0" => {
                    warp::reply::json(&serde_json::json!({
                        "authenticated": true,
                        "user": "admin",
                        "data": "secret-value"
                    }))
                    .into_response()
                }
                _ => warp::reply::with_status(
                    warp::reply::json(&serde_json::json!({"error": "unauthorized"})),
                    warp::http::StatusCode::UNAUTHORIZED,
                )
                .into_response(),
            });

        let port = start_mock_server(protected);

        // Use endpoint.authentication.basic with $secret expressions (same pattern as call_http_basic_auth.yaml)
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: secret-http-auth
  version: '0.1.0'
use:
  secrets:
    - mySecret
do:
  - callSecure:
      call: http
      with:
        method: get
        endpoint:
          uri: http://localhost:PORT/api/secure
          authentication:
            basic:
              username: '${ $secret.mySecret.username }'
              password: '${ $secret.mySecret.password }'
"#
        .replace("PORT", &port.to_string());

        let secret_mgr = Arc::new(crate::secret::MapSecretManager::new().with_secret(
            "mySecret",
            json!({
                "username": "admin",
                "password": "secret"
            }),
        ));

        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow)
            .unwrap()
            .with_secret_manager(secret_mgr);
        let output = runner.run(json!({})).await.unwrap();
        assert_eq!(output["authenticated"], json!(true));
        assert_eq!(output["user"], json!("admin"));
        assert_eq!(output["data"], json!("secret-value"));
    }

    // === E2E: Multi-task ETL — data extraction, transform, load ===

    #[tokio::test]
    async fn test_e2e_secret_http_auth() {
        use warp::Reply;

        let protected = warp::header::optional("Authorization")
            .and(warp::path("protected"))
            .map(|auth: Option<String>| match auth {
                Some(val) if val == "Basic dXNlcjpwYXNz" => {
                    warp::reply::json(&serde_json::json!({"access": "granted", "user": "admin"}))
                        .into_response()
                }
                _ => warp::reply::with_status("Unauthorized", warp::http::StatusCode::UNAUTHORIZED)
                    .into_response(),
            });

        let port = start_mock_server(protected);

        let yaml_str = std::fs::read_to_string(testdata("e2e_secret_http_auth.yaml")).unwrap();
        let yaml_str = yaml_str.replace("PORT", &port.to_string());
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let secret_mgr = crate::secret::MapSecretManager::new()
            .with_secret("authHeader", json!("Basic dXNlcjpwYXNz"));

        let runner = WorkflowRunner::new(workflow)
            .unwrap()
            .with_secret_manager(std::sync::Arc::new(secret_mgr));

        let output = runner.run(json!({})).await.unwrap();
        assert_eq!(output["access"], json!("granted"));
        assert_eq!(output["user"], json!("admin"));
    }

    // === E2E-8: Multi-task ETL workflow ===

    #[tokio::test]
    async fn test_runner_call_http_put_output_as_transforms_response() {

        let route = warp::path!("api" / "v1" / "authors" / ..)
            .and(warp::put())
            .and(warp::body::json())
            .map(|body: Value| {
                warp::reply::json(
                    &json!({"id": 1, "firstName": body["firstName"], "updated": true}),
                )
            });

        let port = start_mock_server(route);

        // Java SDK pattern: output.as extracts specific field from response
        let yaml = format!(
            r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: call-http-output-as
  version: '0.1.0'
do:
  - updateAuthor:
      call: http
      with:
        method: put
        endpoint:
          uri: http://localhost:{port}/api/v1/authors/1
        body:
          firstName: Jane
        output: response
      output:
        as: .body.firstName
"#
        );
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();
        let output = runner.run(json!({})).await.unwrap();
        // output.as extracts just the firstName field
        assert_eq!(output, json!("Jane"));
    }

    #[tokio::test]
    async fn test_bearer_auth_http_call() {

        let handler = warp::path("hello")
            .and(warp::get())
            .and(warp::header::optional("authorization"))
            .map(|auth: Option<String>| match auth {
                Some(token) if token == "Bearer my-token-123" => warp::reply::json(
                    &serde_json::json!({"authenticated": true, "method": "bearer"}),
                ),
                _ => warp::reply::json(&serde_json::json!({"authenticated": false})),
            });

        let port = start_mock_server(handler);

        let workflow: WorkflowDefinition = serde_yaml::from_str(&format!(
            r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: bearer-auth-test
  version: '0.1.0'
do:
  - callApi:
      call: http
      with:
        method: get
        endpoint:
          uri: http://localhost:{port}/hello
          authentication:
            bearer:
              token: my-token-123
"#
        ))
        .unwrap();

        let runner = WorkflowRunner::new(workflow).unwrap();
        let output = runner.run(json!({})).await.unwrap();
        assert_eq!(output["authenticated"], json!(true));
        assert_eq!(output["method"], json!("bearer"));
    }

    #[tokio::test]
    async fn test_basic_auth_http_call() {

        let handler = warp::path("hello")
            .and(warp::get())
            .and(warp::header::optional("authorization"))
            .map(|auth: Option<String>| match auth {
                Some(token) if token.starts_with("Basic ") => {
                    warp::reply::json(&serde_json::json!({"authenticated": true}))
                }
                _ => warp::reply::json(&serde_json::json!({"authenticated": false})),
            });

        let port = start_mock_server(handler);

        let workflow: WorkflowDefinition = serde_yaml::from_str(&format!(
            r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: basic-auth-test
  version: '0.1.0'
do:
  - callApi:
      call: http
      with:
        method: get
        endpoint:
          uri: http://localhost:{port}/hello
          authentication:
            basic:
              username: admin
              password: secret
"#
        ))
        .unwrap();

        let runner = WorkflowRunner::new(workflow).unwrap();
        let output = runner.run(json!({})).await.unwrap();
        assert_eq!(output["authenticated"], json!(true));
    }

    // === HTTP Call: Basic Auth with $secret expression (username: ${$secret.mySecret.username}) ===
    // Matches Java SDK's basic-properties-auth.yaml pattern

    #[tokio::test]
    async fn test_runner_call_http_output_as_expr() {

        let api = warp::path("pets").and(warp::path::param()).map(|id: u32| {
            warp::reply::json(&serde_json::json!({
                "id": id,
                "name": format!("Pet {}", id),
                "status": "available"
            }))
        });

        let output = run_workflow_with_mock_server("call_http_output_as_expr.yaml", api, json!({})).await;
        assert_eq!(output["id"], json!(42));
        assert_eq!(output["petName"], json!("Pet 42"));
    }
