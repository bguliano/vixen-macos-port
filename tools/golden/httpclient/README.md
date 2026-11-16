# Local HttpClient transport reference

This helper records .NET's request behavior for the original RDS handler setup (`NetworkCredential` on `HttpClientHandler`) and FPP's default `HttpClient`/seekable `StreamContent` requests. It does not implement or substitute the HTTP behavior being checked. Thirty-three cases cover Basic authentication, credential-cache control, same/cross-origin redirects, status-dependent GET/POST conversion/replay, the 50-redirect limit, TLS upgrade/downgrade, URL userinfo and per-client cookie persistence/isolation.

```sh
bash tools/golden/httpclient/build.sh run
```

The build uses only the cached .NET SDK. `run.py` starts ephemeral Python loopback HTTP/TLS servers, generates a temporary test certificate and deletes the certificate/private key on exit. The helper accepts only this test TLS fixture through a callback; it does not change the machine trust store. This is a routing/authentication oracle, not a certificate-verification test. The Rust RDS and FPP transport suites separately assert rejection of an unknown CA and wrong hostname with verification enabled.

The reference checkpoint is **33 cases on .NET 10.0.12, macOS**, with request traces and OS/runtime version in `target/golden/httpclient-reference/results.json`. A Windows runtime has not been exercised. The explicit FTP-location probe remains local: .NET sends HTTP to a loopback port even though the redirect says FTP. Both port transports intentionally restrict requests to HTTP(S); no file or FTP device is contacted. `Program.cs`'s certificate callback is test-only and must never enter production.
