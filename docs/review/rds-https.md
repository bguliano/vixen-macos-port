# RDS HTTP(S) transport — November 11, 2026

RDS HTTPS was a missing upstream capability. It is now implemented; it is no longer an unsupported-platform item.

## Source and reference behavior

`upstream/Vixen/src/Vixen.Modules/Controller/RDSController/Module.cs:48–59` substitutes text/time in the configured URL and passes it directly to `HttpClient.GetAsync`. The optional credentials are a plain `NetworkCredential` on a new `HttpClientHandler`. The original setup labels explicitly say HTTP(S).

A local .NET 10.0.12 reference using that unchanged handler/client sequence records 33 cases in `target/golden/httpclient-reference/results.json`. Its reproducible source is `tools/golden/httpclient` (`bash tools/golden/httpclient/build.sh run`). This checks the framework available on this Mac, not Windows network services. Its request traces establish:

- Initial GET has no Authorization; Basic authentication answers a challenge.
- Every redirect clears credentials, including redirects to the same origin. A destination challenge is not retried with the original plain `NetworkCredential`. A `CredentialCache` positive control behaves differently and is not used by Vixen.
- 300, 301, 302, 303, 307 and 308 are followed; 304, 305 and 306 are returned.
- The default limit is 50 redirects: request 51 returns its final 302 rather than throwing.
- HTTP to HTTPS is followed; HTTPS to HTTP returns the redirect without following it.
- URL userinfo is ignored. Only the configured handler credentials authenticate.

## Implementation

`crates/vixen-output/native/rds_http.c` uses the system libcurl and its TLS implementation. `src/modules/rds/http.rs` owns the Rust/C boundary. No downloaded TLS dependency or command-line subprocess is used by the application. The Swift executable links system `curl`; the Rust build uses the already cached `cc` dependency.

TLS certificate-chain and hostname validation are always enabled. Production has no custom CA option or insecure switch. A private CA path exists only in the Rust unit-test build. URL parsing and relative resolution use libcurl. Redirects are explicit to preserve the credential and downgrade rules above. A per-request, in-memory cookie jar survives redirects and disappears when the radio-text request ends. Basic and Digest are tested; additional mechanisms available through system libcurl have not been independently exercised.

Each transfer uses a synchronous callback into a live Rust cancellation context. The curl multi loop polls at most 100 ms between cancellation checks. The module cancels and joins its workers on Stop and Drop, and restart uses fresh tokens. Handles and callbacks are removed before the Rust context is released. Global libcurl initialization is once per transport and stays alive for process lifetime.

The request deadline is 100 seconds, matching HttpClient's default; connecting is limited to 10 seconds. Response bodies are discarded with a 16 MiB limit; headers are bounded to 64 KiB and Location to 8 KiB. These are explicit resource guards, not a claim of identical arbitrary-response limits. Unsupported URL schemes are rejected. In particular, the .NET reference's surprising FTP-location behavior (sending an HTTP GET to the named port) is deliberately not reproduced. No local-file URL can be fetched.

## Verification

`target/rds-output-tests.log`: **301 passed, 0 failed, 0 ignored**, including the existing encoded HTTP/Basic wire test and serial/controller tests. The two new scenario tests cover:

- Trusted HTTPS, unknown CA rejection, hostname mismatch rejection and final status/reason.
- Basic and Digest challenge/response; no initial credential disclosure; same-origin, cross-port and cross-scheme redirect credential stripping.
- Redirect statuses, relative paths, HTTP to HTTPS, HTTPS downgrade refusal and exact 51-request redirect limit.
- Per-request redirect cookies and isolation between separate radio-text calls.
- URL userinfo, unsupported schemes and embedded-NUL rejection.
- Oversized body/header failures, short deadline, cancellation during a TLS response, Stop joining outstanding HTTP work, no error logged for deliberate cancellation and successful restart.

The fixture `crates/vixen-output/tests/fixtures/rds_http.py` binds only ephemeral loopback ports. It creates and removes a temporary CA/private key without changing system trust. Tests require local Python 3 and OpenSSL. No original show, device or external service is accessed.

Reproduce with:

```sh
source ./env.sh
CARGO_TARGET_DIR=target/vixen-session cargo test -p vixen-output --offline -j2
```

A sandbox must permit local loopback sockets. Real RDS hardware and installed-device certificates remain external verification items. The final integrated application link and resource audit are tracked separately by the integration owner.
