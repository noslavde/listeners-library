use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

// OAuth registration profiles from the upstream protocol reference. These are
// API client registrations, not imported CDM/device keys or security levels.
#[derive(Clone, Copy, Default, Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Profile {
    #[default]
    Ios,
    Android,
}
impl Profile {
    pub fn device_type(self) -> &'static str {
        match self {
            Self::Ios => "A2CZJZGLK2JJVM",
            Self::Android => "A10KISP2GWF0E4",
        }
    }
    pub fn assoc(self) -> &'static str {
        match self {
            Self::Ios => "amzn_audible_ios_us",
            Self::Android => "amzn_audible_android_experiment_us",
        }
    }
    pub fn page(self) -> &'static str {
        match self {
            Self::Ios => "amzn_audible_ios_v2_light_us",
            Self::Android => "amzn_audible_android_aui_v2_dark_us",
        }
    }
    pub fn agent(self) -> &'static str {
        match self {
            Self::Ios => "AmazonWebView/Audible/4.60/iOS/26.2/iPhone",
            Self::Android => "Dalvik/2.1.0 (Linux; U; Android 30; OnePlus8 Build/RP1A.201005.001)",
        }
    }
    pub fn refresh_app(self) -> (&'static str, &'static str) {
        match self {
            Self::Ios => ("Audible", "3.56.2"),
            Self::Android => ("com.audible.application", "160008"),
        }
    }
    pub fn registration(self, serial: &str) -> Value {
        let (domain, version, os, model, name, software) = match self {
            Self::Ios => ("Device", "4.60", "26.2", "iPhone", "Audible", "46000856"),
            Self::Android => (
                "DeviceLegacy",
                "160008",
                "30",
                "OnePlus8",
                "com.audible.application",
                "130050002",
            ),
        };
        // Each sign-in already has a fresh random serial. Derive a stable name
        // for that attempt rather than reusing one name for every registration.
        // Do not include the user's account, computer name or other personal data.
        let suffix: String = Sha256::digest(serial.as_bytes())[..16]
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        let device_name = format!("Listeners Library {suffix}");
        json!({"domain":domain,"app_version":version,"device_type":self.device_type(),"device_name":device_name,"os_version":os,"device_serial":serial,"device_model":model,"app_name":name,"software_version":software})
    }
}
