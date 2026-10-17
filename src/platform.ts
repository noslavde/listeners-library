// Set only by the dedicated macOS build command; ordinary builds stay Windows.
export const macExperimental =
  import.meta.env.VITE_APP_CHANNEL === "macos-experimental";
export const protectionName = macExperimental
  ? "macOS Keychain"
  : "Windows encryption";
