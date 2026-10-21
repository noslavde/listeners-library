//! Fixed-origin device-signed annotation requests. Credentials never enter IPC.
use super::*;
use ring::{
    rand::SystemRandom,
    signature::{RsaKeyPair, RSA_PKCS1_SHA256},
};
use zeroize::{Zeroize, Zeroizing};
const ORIGIN: &str = "https://cde-ta-g7g.amazon.com";
#[derive(Serialize, Deserialize)]
pub struct SigningAuth {
    token: String,
    key: String,
}
impl Drop for SigningAuth {
    fn drop(&mut self) {
        self.token.zeroize();
        self.key.zeroize();
    }
}
fn key_pair(pem: &str) -> Result<RsaKeyPair, String> {
    if pem.len() > 32768 {
        return Err("Invalid annotation signing key".into());
    }
    let encoded = Zeroizing::new(
        pem.lines()
            .filter(|l| !l.starts_with("-----"))
            .collect::<String>()
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect::<String>(),
    );
    let der = Zeroizing::new(
        STANDARD
            .decode(encoded.as_bytes())
            .map_err(|_| "Cannot unlock annotation signing key")?,
    );
    RsaKeyPair::from_pkcs8(&der)
        .or_else(|_| RsaKeyPair::from_der(&der))
        .map_err(|_| "Unsupported annotation signing key".into())
}
pub(super) fn registration(v: &Value) -> Option<SigningAuth> {
    let d = v.pointer("/response/success/tokens/mac_dms")?;
    let token = d["adp_token"].as_str()?.to_owned();
    let key = d["device_private_key"].as_str()?.to_owned();
    let auth = SigningAuth { token, key };
    if auth.token.is_empty() || auth.token.len() > 16384 || key_pair(&auth.key).is_err() {
        return None;
    }
    Some(auth)
}
pub(crate) fn timestamp() -> String {
    let t = unsafe { windows::Win32::System::SystemInformation::GetSystemTime() };
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
        t.wYear, t.wMonth, t.wDay, t.wHour, t.wMinute, t.wSecond, t.wMilliseconds
    )
}
fn signature(
    auth: &SigningAuth,
    method: &str,
    path: &str,
    date: &str,
    body: &str,
) -> Result<String, String> {
    let pair = key_pair(&auth.key)?;
    let message = Zeroizing::new(format!("{method}\n{path}\n{date}\n{body}\n{}", auth.token));
    let mut result = vec![0; pair.public().modulus_len()];
    pair.sign(
        &RSA_PKCS1_SHA256,
        &SystemRandom::new(),
        message.as_bytes(),
        &mut result,
    )
    .map_err(|_| "Cannot sign annotation request")?;
    Ok(format!("{}:{date}", STANDARD.encode(result)))
}
fn sensitive(value: &str) -> Result<reqwest::header::HeaderValue, String> {
    let mut h = reqwest::header::HeaderValue::from_str(value)
        .map_err(|_| "Invalid annotation credential")?;
    h.set_sensitive(true);
    Ok(h)
}
pub(crate) async fn available(app: &tauri::AppHandle, generation: u64) -> Result<bool, String> {
    with_account_generation(app, generation, || Ok(()))?;
    let state = app.state::<MetadataState>();
    let auth = state.auth.lock().await;
    with_account_generation(app, generation, || {
        Ok(auth
            .as_ref()
            .is_some_and(|a| a.annotation_signing.is_some()))
    })
}
pub(crate) async fn request(
    app: &tauri::AppHandle,
    generation: u64,
    asin: &str,
    body: Option<&str>,
) -> Result<Option<Value>, String> {
    if !crate::valid_asin(asin) {
        return Err("Invalid annotation book".into());
    }
    let state = app.state::<MetadataState>();
    let auth = state.auth.lock().await;
    with_account_generation(app, generation, || Ok(()))?;
    let signing=auth.as_ref().and_then(|a|a.annotation_signing.as_ref()).ok_or("Annotation sync needs a new sign-in with Enable Audible annotation sync selected. Your existing local items are kept.")?;
    let path = if body.is_some() {
        "/FionaCDEServiceEngine/sidecar?type=AUDI".to_owned()
    } else {
        format!("/FionaCDEServiceEngine/sidecar?type=AUDI&key={asin}")
    };
    let method = if body.is_some() { "POST" } else { "GET" };
    let date = timestamp();
    let sig = signature(signing, method, &path, &date, body.unwrap_or(""))?;
    let client = http()?;
    let mut builder = client
        .request(
            if body.is_some() {
                reqwest::Method::POST
            } else {
                reqwest::Method::GET
            },
            format!("{ORIGIN}{path}"),
        )
        .header("x-adp-token", sensitive(&signing.token)?)
        .header("x-adp-alg", "SHA256withRSA:1.0")
        .header("x-adp-signature", sensitive(&sig)?);
    if let Some(body) = body {
        if body.len() > 65536 {
            return Err("Annotation request too large".into());
        }
        builder = builder
            .header("content-type", "application/xml; charset=utf-8")
            .body(body.to_owned());
    }
    with_account_generation(app, generation, || Ok(()))?;
    let mut response = builder.send().await.map_err(|_| {
        if body.is_some() {
            "Annotation write outcome unknown. Refresh from Audible; do not send again yet."
        } else {
            "Annotation read failed. Local items were kept."
        }
    })?;
    let status = response.status().as_u16();
    if status == 404 && body.is_none() {
        return Ok(None);
    }
    if !(200..300).contains(&status) {
        return Err(format!("Audible annotation {} returned HTTP {status}. Local data was kept; no response content was logged.",if body.is_some(){"write"}else{"read"}));
    }
    let mut bytes = Zeroizing::new(Vec::new());
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| "Annotation response interrupted; refresh before retrying")?
    {
        if bytes.len() + chunk.len() > 2_000_000 {
            return Err("Annotation response exceeds safe size limit".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    with_account_generation(app, generation, || Ok(()))?;
    if body.is_some() {
        return Ok(Some(Value::Null));
    }
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(|_| "Unexpected annotation response; local data was kept".into())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_bad_keys_without_exposing_them() {
        let v = json!({"response":{"success":{"tokens":{"mac_dms":{"adp_token":"PRIVATE_TOKEN","device_private_key":"PRIVATE_KEY"}}}}});
        assert!(registration(&v).is_none());
        assert!(!key_pair("PRIVATE_KEY")
            .err()
            .unwrap()
            .contains("PRIVATE_KEY"));
    }
    #[test]
    fn utc_timestamp_has_explicit_zone() {
        let t = timestamp();
        assert_eq!(t.len(), 24);
        assert!(t.ends_with('Z'));
        assert_eq!(&t[10..11], "T");
    }
    #[test]
    fn signs_exact_request_with_an_ephemeral_synthetic_key() {
        use std::os::windows::process::CommandExt;
        let mut output=std::process::Command::new("node").creation_flags(0x08000000).args(["-e","const c=require('node:crypto');const k=c.generateKeyPairSync('rsa',{modulusLength:2048,privateKeyEncoding:{type:'pkcs8',format:'pem'},publicKeyEncoding:{type:'pkcs1',format:'der'}});process.stdout.write(JSON.stringify({private:k.privateKey,public:k.publicKey.toString('base64')}));"]).output().expect("Node is required by the documented native test runner");
        assert!(output.status.success());
        #[derive(Deserialize)]
        struct Generated {
            private: String,
            public: String,
        }
        let mut generated: Generated = serde_json::from_slice(&output.stdout).unwrap();
        output.stdout.zeroize();
        let auth = SigningAuth {
            token: "synthetic-token".into(),
            key: std::mem::take(&mut generated.private),
        };
        let date = "2026-10-20T00:00:00.000Z";
        let signed = signature(
            &auth,
            "GET",
            "/FionaCDEServiceEngine/sidecar?type=AUDI&key=B012345678",
            date,
            "",
        )
        .unwrap();
        let (encoded, returned_date) = signed.split_once(':').unwrap();
        assert_eq!(returned_date, date);
        let public = STANDARD.decode(generated.public).unwrap();
        let expected=b"GET\n/FionaCDEServiceEngine/sidecar?type=AUDI&key=B012345678\n2026-10-20T00:00:00.000Z\n\nsynthetic-token";
        ring::signature::UnparsedPublicKey::new(
            &ring::signature::RSA_PKCS1_2048_8192_SHA256,
            public,
        )
        .verify(expected, &STANDARD.decode(encoded).unwrap())
        .unwrap();
        assert!(sensitive("synthetic-token").unwrap().is_sensitive());
    }
}
