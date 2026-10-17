//! Fresh-account regression tests. No provider account or real credentials.
use super::*;
use std::{
    io::{Read, Write},
    net::TcpListener,
};
fn pending() -> Pending {
    Pending {
        profile: Profile::default(),
        verifier: URL_SAFE_NO_PAD.encode([7u8; 32]),
        serial: "synthetic-device-serial".into(),
        created: now(),
        generation: 1,
    }
}
fn response() -> Value {
    json!({"response":{"success":{"extensions":{"device_info":{"device_type":"A2CZJZGLK2JJVM"}},"tokens":{"bearer":{"access_token":"synthetic-access-only","refresh_token":"synthetic-refresh-only","expires_in":"3600.0"}}}}})
}
#[test]
fn account_namespaces_follow_provider_identity_without_exposing_it() {
    let mut a = response();
    a["response"]["success"]["extensions"]["customer_info"] = json!({"user_id":"synthetic-account-A"});
    let first = registration_auth(&a, Profile::Ios).unwrap().namespace();
    a["response"]["success"]["tokens"]["bearer"]["refresh_token"] = json!("different-registration");
    assert_eq!(registration_auth(&a, Profile::Ios).unwrap().namespace(), first);
    a["response"]["success"]["extensions"]["customer_info"]["user_id"] = json!("synthetic-account-B");
    assert_ne!(registration_auth(&a, Profile::Ios).unwrap().namespace(), first);
    assert_eq!(first.len(), 64);
    assert!(!first.contains("synthetic"));
    let old = registration_auth(&response(), Profile::Ios).unwrap();
    assert_ne!(old.namespace(), first);
    let state = MetadataState::with_auth(Some(old));
    assert!(state.guard_generation(0).is_ok());
    state.generation.fetch_add(1, Ordering::SeqCst);
    assert!(state.guard_generation(0).is_err());
}
#[test]
fn token_lifetime_accepts_provider_numeric_formats_and_rejects_invalid_values() {
    for value in [
        json!(3600),
        json!(3600.0),
        json!("3600"),
        json!("3600.0"),
        json!("3599.75"),
    ] {
        let seconds = expiry(&json!({"expires_in":value})).unwrap();
        assert!((3599..=3600).contains(&seconds));
    }
    for value in [
        json!(null),
        json!(true),
        json!("NaN"),
        json!("inf"),
        json!("-1"),
        json!(0),
        json!(31_536_000),
        json!([]),
    ] {
        assert!(expiry(&json!({"expires_in":value})).is_err());
    }
}

#[test]
fn signout_revokes_access_immediately_even_while_tokens_are_still_in_memory() {
    let auth = registration_auth(&response(), Profile::Ios).unwrap();
    let state = MetadataState::with_auth(Some(auth));
    assert!(state.connected());
    // Disconnect invalidates the generation before waiting for any API lock.
    state.generation.fetch_add(1, Ordering::SeqCst);
    assert!(!state.connected());
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    runtime.block_on(async {
        assert!(state.auth.lock().await.is_some());
        let status = state.connection().await.unwrap();
        assert!(!status.connected);
        assert!(status.profile.is_none());
    });
}

#[test]
fn a_registration_finishing_after_signout_cannot_reopen_account_access() {
    let state = MetadataState::with_auth(None);
    state.generation.store(1, Ordering::SeqCst);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    runtime.block_on(async {
        let auth = registration_auth(&response(), Profile::Ios).unwrap();
        // Simulate sign-out during the protected save after the initial check.
        state
            .finish_registration(1, auth, |_| {
                state.generation.fetch_add(1, Ordering::SeqCst);
                Ok(())
            })
            .await
            .unwrap_err();
        assert!(!state.connected());
        assert!(!state.connection().await.unwrap().connected);
    });
}
#[test]
fn fresh_authorization_exchange_encrypted_save_reload_and_status() {
    let root =
        std::env::temp_dir().join(format!("audible-login-flow-test-{}", rand::random::<u64>()));
    let store = crate::storage::Storage::initialize(&root, false).unwrap();
    let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).unwrap();
    let endpoint = format!("http://{}/auth/register", listener.local_addr().unwrap());
    let server = std::thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut request = Vec::new();
        let (header_end, length) = loop {
            let mut buffer = [0; 2048];
            let n = socket.read(&mut buffer).unwrap();
            assert!(n > 0);
            request.extend_from_slice(&buffer[..n]);
            assert!(request.len() < 65536);
            if let Some(end) = request.windows(4).position(|p| p == b"\r\n\r\n") {
                let headers = String::from_utf8_lossy(&request[..end]);
                let length = headers
                    .lines()
                    .find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        name.eq_ignore_ascii_case("content-length")
                            .then(|| value.trim().parse::<usize>().unwrap())
                    })
                    .unwrap();
                break (end + 4, length);
            }
        };
        while request.len() < header_end + length {
            let mut buffer = [0; 2048];
            let n = socket.read(&mut buffer).unwrap();
            assert!(n > 0);
            request.extend_from_slice(&buffer[..n]);
        }
        let body: Value =
            serde_json::from_slice(&request[header_end..header_end + length]).unwrap();
        let data = serde_json::to_vec(&response()).unwrap();
        write!(socket,"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",data.len()).unwrap();
        socket.write_all(&data).unwrap();
        body
    });
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async{
  let p=pending();let state=MetadataState::with_auth(None);state.generation.store(p.generation,Ordering::SeqCst);
  assert!(!state.connection().await.unwrap().connected);
  let url=authorize(&p);assert_eq!(url.query_pairs().find(|(k,_)|k=="openid.assoc_handle").unwrap().1,"amzn_audible_ios_us");
  let code=callback_code(&url::Url::parse("https://www.amazon.com/ap/maplanding?openid.oa2.authorization_code=synthetic-approved-code").unwrap()).unwrap();
  let client=reqwest::Client::builder().no_proxy().timeout(Duration::from_secs(5)).build().unwrap();
  let auth=exchange_registration(&client,&endpoint,&p,&code).await.unwrap();
  state.finish_registration(p.generation,auth,|auth|{
   let raw=zeroize::Zeroizing::new(serde_json::to_vec(auth).unwrap());store.save_account_bytes(&raw)
  }).await.unwrap();
  assert!(state.connection().await.unwrap().connected);
  let sealed=std::fs::read(store.dir.join("account.sealed")).unwrap();assert!(!sealed.windows(b"synthetic-access-only".len()).any(|p|p==b"synthetic-access-only"));
  drop(state);
  let raw=store.load_account_bytes().unwrap().unwrap();let restored=MetadataState::with_auth(Some(serde_json::from_slice(&raw).unwrap()));
  let status=restored.connection().await.unwrap();assert!(status.connected);assert_eq!(status.phase,"connected");
  let public=serde_json::to_string(&status).unwrap();assert!(!public.contains("synthetic-access"));assert!(!public.contains("synthetic-refresh"));
 });
    let request = server.join().unwrap();
    assert_eq!(
        request["registration_data"]["device_type"],
        "A2CZJZGLK2JJVM"
    );
    assert_eq!(request["auth_data"]["code_algorithm"], "SHA-256");
    assert_eq!(
        request["auth_data"]["authorization_code"],
        "synthetic-approved-code"
    );
    assert_eq!(request["requested_token_type"], json!(["bearer"]));
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn failed_protected_save_or_cancelled_exchange_cannot_connect() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let state = MetadataState::with_auth(None);
        state.generation.store(1, Ordering::SeqCst);
        let auth = registration_auth(&response(), Profile::Ios).unwrap();
        assert!(state
            .finish_registration(1, auth, |_| Err("Synthetic secure-store failure".into()))
            .await
            .is_err());
        assert!(!state.connection().await.unwrap().connected);
        state.generation.store(2, Ordering::SeqCst);
        let auth = registration_auth(&response(), Profile::Ios).unwrap();
        assert!(state
            .finish_registration(1, auth, |_| panic!(
                "Cancelled exchange must not save credentials"
            ))
            .await
            .is_err());
        assert!(!state.connection().await.unwrap().connected);
    });
}

#[test]
fn stale_note_session_is_rejected_even_after_another_account_connects(){
    let runtime=tokio::runtime::Builder::new_current_thread().build().unwrap();
    runtime.block_on(async{
        let state=MetadataState::with_auth(Some(registration_auth(&response(),Profile::Ios).unwrap()));
        assert!(state.guard_generation(0).is_ok());
        state.generation.store(1,Ordering::SeqCst);
        assert!(state.guard_generation(0).is_err());
        state.finish_registration(1,registration_auth(&response(),Profile::Ios).unwrap(), |_|Ok(())).await.unwrap();
        assert!(state.connected());
        assert!(state.guard_generation(0).is_err());
        assert!(state.guard_generation(1).is_ok());
    });
}

#[test]
fn registration_diagnostic_reports_shape_without_provider_values() {
    let mut value = response();
    value["response"]["error"] =
        json!({"message":"private-provider-message", "code":"private-provider-code"});
    let diagnostic = registration_shape(&value);
    assert!(diagnostic.contains(
        "success=true error=true bearer=true access=true refresh=true expiry=true device=true"
    ));
    for secret in [
        "synthetic-access-only",
        "synthetic-refresh-only",
        "private-provider-message",
        "private-provider-code",
        "A2CZJZGLK2JJVM",
    ] {
        assert!(!diagnostic.contains(secret));
    }
    assert!(registration_shape(&json!({"response":{"error":{"message":"private"}}}))
        .contains("success=false error=true bearer=false access=false refresh=false expiry=false device=false"));
}

#[test]
fn connection_diagnostic_is_bounded_and_identifies_native_build() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    runtime.block_on(async {
        let state = MetadataState::with_auth(None);
        for n in 0..30 {
            state.trace(format!("synthetic stage {n}"));
        }
        let status = state.connection().await.unwrap();
        assert_eq!(status.version, env!("CARGO_PKG_VERSION"));
        assert_eq!(status.diagnostics.len(), 24);
        assert_eq!(status.diagnostics[0], "synthetic stage 6");
        assert!(!status.connected);
    });
}

#[test]
fn packaged_first_build_android_contract_changes_only_duplicate_device_name() {
    // Source checkpoint e21c0ea492beea3cef2d58cc85ec378d11f456dc,
    // the first actual NSIS-enabled 0.1.0 build, not the earlier iOS prototype.
    let mut p = pending();
    p.profile = Profile::Android;
    let url = authorize(&p);
    let pairs = url
        .query_pairs()
        .collect::<std::collections::HashMap<_, _>>();
    assert_eq!(
        pairs.get("openid.assoc_handle").unwrap(),
        "amzn_audible_android_experiment_us"
    );
    assert_eq!(
        pairs.get("pageId").unwrap(),
        "amzn_audible_android_aui_v2_dark_us"
    );
    assert_eq!(
        pairs.get("openid.return_to").unwrap(),
        "https://www.amazon.com/ap/maplanding"
    );
    let mut body = registration_body(&p, "synthetic-code");
    assert!(body["registration_data"]["device_name"]
        .as_str()
        .unwrap()
        .starts_with("Listeners Library "));
    assert_ne!(
        body["registration_data"]["device_name"],
        "Audible Desktop Lab"
    );
    // Compare the original wire fields while explicitly allowing the corrected name.
    body["registration_data"]["device_name"] = json!("Audible Desktop Lab");
    assert_eq!(
        body["registration_data"],
        json!({
            "domain":"DeviceLegacy","app_version":"160008","device_type":"A10KISP2GWF0E4",
            "device_name":"Audible Desktop Lab","os_version":"30","device_serial":p.serial,
            "device_model":"OnePlus8","app_name":"com.audible.application","software_version":"130050002"
        })
    );
    assert_eq!(body["auth_data"]["client_domain"], "DeviceLegacy");
    assert_eq!(body["auth_data"]["code_algorithm"], "SHA-256");
    assert_eq!(body["requested_token_type"], json!(["bearer"]));
    assert_eq!(
        body["requested_extensions"],
        json!(["device_info", "customer_info"])
    );
    assert_eq!(
        Profile::Android.refresh_app(),
        ("com.audible.application", "160008")
    );
}

#[test]
fn registrations_have_distinct_names_that_stay_stable_within_an_attempt() {
    for profile in [Profile::Ios, Profile::Android] {
        let mut names = std::collections::HashSet::new();
        for n in 0..256 {
            let serial = format!("synthetic-device-serial-{n}");
            let registration = profile.registration(&serial);
            let name = registration["device_name"].as_str().unwrap();
            assert_eq!(registration["device_serial"], serial);
            assert_eq!(registration["device_type"], profile.device_type());
            assert_eq!(registration, profile.registration(&serial));
            assert_eq!(name.len(), "Listeners Library ".len() + 32);
            assert!(name
                .strip_prefix("Listeners Library ")
                .unwrap()
                .bytes()
                .all(|b| b.is_ascii_hexdigit()));
            assert!(!name.contains("synthetic-device-serial"));
            assert!(names.insert(name.to_owned()));
        }
    }
}

#[test]
fn registration_rejections_expose_only_known_codes_and_fixed_categories() {
    for (code, message, category) in [
        (
            "InvalidToken",
            "Auth time of the token is expired. private-value",
            "expired authorization",
        ),
        (
            "InvalidRequest",
            "Invalid code_verifier private-value",
            "code verification problem",
        ),
        (
            "InvalidGrant",
            "authorization code already used private-value",
            "already used",
        ),
        (
            "InvalidClient",
            "bad client_id private-value",
            "client identifier problem",
        ),
        (
            "InvalidInput",
            "bad device_type private-value",
            "device registration problem",
        ),
    ] {
        let safe = registration_rejection(
            &json!({"response":{"error":{"code":code,"message":message,"access_token":"private-token"}}}),
        );
        assert!(safe.contains(code));
        assert!(safe.contains(category));
        assert!(!safe.contains("private") || safe.contains("private server message omitted"));
        assert!(!safe.contains("private-value"));
        assert!(!safe.contains("private-token"));
    }
    let safe = registration_rejection(
        &json!({"response":{"error":{"code":"private-code","message":"private-email@example.invalid"}}}),
    );
    assert_eq!(safe, "unrecognized code · private server message omitted");
}

fn rejection_fixture(status: u16, data: Vec<u8>) -> (String, std::thread::JoinHandle<()>) {
    let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let server = std::thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut request = Vec::new();
        while !request.windows(4).any(|w| w == b"\r\n\r\n") {
            let mut buf = [0; 1024];
            let n = socket.read(&mut buf).unwrap();
            assert!(n > 0);
            request.extend_from_slice(&buf[..n]);
            assert!(request.len() < 8192);
        }
        write!(socket, "HTTP/1.1 {status} Error\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", data.len()).unwrap();
        let _ = socket.write_all(&data);
    });
    (endpoint, server)
}

#[test]
fn http_400_keeps_rejection_reason_but_never_private_body_in_status_or_trace() {
    let data = serde_json::to_vec(&json!({"response":{"error":{"code":"InvalidToken","message":"Auth time of the token is expired. private-account@example.invalid","refresh_token":"private-token"}}})).unwrap();
    let (endpoint, server) = rejection_fixture(400, data);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let client = reqwest::Client::builder()
            .no_proxy()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap();
        let response = client.get(endpoint).send().await.unwrap();
        let trace = std::cell::RefCell::new(Vec::new());
        let error = decode_registration(response, &|entry| trace.borrow_mut().push(entry))
            .await
            .err()
            .unwrap();
        assert!(error.contains("HTTP 400"));
        assert!(error.contains("InvalidToken"));
        assert!(error.contains("expired authorization"));
        assert!(!error.contains("private-account"));
        assert!(!error.contains("private-token"));
        let trace = trace.borrow().join("\n");
        assert!(trace.contains("InvalidToken"));
        assert!(!trace.contains("private-account"));
        assert!(!trace.contains("private-token"));
    });
    server.join().unwrap();
}

#[test]
fn unreadable_or_oversized_rejection_bodies_are_omitted() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    for data in [b"private-html-error".to_vec(), vec![b'x'; 65_537]] {
        let (endpoint, server) = rejection_fixture(400, data);
        runtime.block_on(async {
            let client = reqwest::Client::builder()
                .no_proxy()
                .timeout(Duration::from_secs(5))
                .build()
                .unwrap();
            let error = decode_registration(client.get(endpoint).send().await.unwrap(), &|_| {})
                .await
                .err()
                .unwrap();
            assert!(error.contains("HTTP 400"));
            assert!(!error.contains("private-html-error"));
        });
        server.join().unwrap();
    }
}
