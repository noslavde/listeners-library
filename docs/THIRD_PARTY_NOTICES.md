# Third-party protocol reference

The metadata-only provider adapts OAuth device profile parameters, registration shape and metadata request conventions from mkb79/audible-rs, inspected at commit `3188435539fe6924592c0686ae06e0973c08ecc6`. The upstream crate is not linked or shipped. None of its download, license, activation or decryption modules are included.

Source: https://github.com/mkb79/audible-rs

The optional Android API registration profile also uses the documented device/OAuth parameters from `src/auth/device.rs` at that revision. This registers the API client; it does not include upstream CDM provisioning, private device-key handling, or content-key extraction.

The website-session experiment also follows the exchange request parameters and response shape documented by upstream `src/auth/cookies.rs`. The implementation keeps only a bounded, scoped session header in Rust memory; it does not import browser profiles or persist website cookies.

MIT License

Copyright (c) 2026 mkb79

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
