/* RDS HttpClient.GetAsync transport. System libcurl supplies TLS and URL parsing;
 * redirects stay explicit because HttpClient clears NetworkCredential on redirect.
 * All pointers are borrowed for this synchronous call; no callback outlives it. */
#include <curl/curl.h>
#include <limits.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include <strings.h>
#include <time.h>

#define BODY_LIMIT (16u * 1024u * 1024u)
#define HEADER_LIMIT (64u * 1024u)
#define MAX_REDIRECTS 50

typedef int (*cancel_fn)(const void *);
typedef struct {
    int status;
    char reason[1024];
    char error[CURL_ERROR_SIZE];
} rds_result;
typedef struct {
    rds_result *result;
    char location[8192];
    size_t body_bytes;
    size_t header_bytes;
    int limit;
} response_state;

static int64_t milliseconds(void) {
    struct timespec time;
    clock_gettime(CLOCK_MONOTONIC, &time);
    return (int64_t)time.tv_sec * 1000 + time.tv_nsec / 1000000;
}

static size_t body(char *data, size_t size, size_t count, void *opaque) {
    (void)data;
    response_state *s = opaque;
    if (size && count > SIZE_MAX / size) return 0;
    size_t bytes = size * count;
    if (bytes > BODY_LIMIT - s->body_bytes) { s->limit = 1; return 0; }
    s->body_bytes += bytes;
    return bytes;
}

static size_t header(char *data, size_t size, size_t count, void *opaque) {
    response_state *s = opaque;
    if (size && count > SIZE_MAX / size) return 0;
    size_t bytes = size * count;
    if (bytes > HEADER_LIMIT - s->header_bytes) { s->limit = 2; return 0; }
    s->header_bytes += bytes;
    if (bytes >= 5 && !memcmp(data, "HTTP/", 5)) {
        s->location[0] = 0;
        s->result->reason[0] = 0;
        const char *space = memchr(data, ' ', bytes);
        if (space) {
            size_t remaining = bytes - (size_t)(space + 1 - data);
            const char *reason = memchr(space + 1, ' ', remaining);
            if (reason) {
                reason++;
                size_t n = bytes - (size_t)(reason - data);
                while (n && (reason[n - 1] == '\r' || reason[n - 1] == '\n')) n--;
                if (n >= sizeof(s->result->reason)) n = sizeof(s->result->reason) - 1;
                memcpy(s->result->reason, reason, n);
                s->result->reason[n] = 0;
            }
        }
    } else if (bytes >= 9 && !strncasecmp(data, "Location:", 9)) {
        const char *value = data + 9;
        size_t n = bytes - 9;
        while (n && (*value == ' ' || *value == '\t')) { value++; n--; }
        while (n && (value[n - 1] == '\r' || value[n - 1] == '\n' || value[n - 1] == ' ' || value[n - 1] == '\t')) n--;
        if (n >= sizeof(s->location)) { s->limit = 2; return 0; }
        memcpy(s->location, value, n);
        s->location[n] = 0;
    }
    return bytes;
}

int vixen_rds_http_init(void) { return (int)curl_global_init(CURL_GLOBAL_DEFAULT); }

/* ca_file is supplied only by Rust's private test options. Production passes NULL.
 * Return 0 for an HTTP response, 1 for cancellation, -1 for a transport error. */
int vixen_rds_http_get(const char *url, const char *user, const char *password,
                       const char *ca_file, long timeout_ms, long connect_ms,
                       cancel_fn cancelled, const void *cancel_context,
                       rds_result *result) {
    memset(result, 0, sizeof(*result));
    CURLU *uri = curl_url();
    if (!uri) { snprintf(result->error, sizeof(result->error), "Cannot allocate HTTP URL"); return -1; }
    if (curl_url_set(uri, CURLUPART_URL, url, 0) != CURLUE_OK) {
        snprintf(result->error, sizeof(result->error), "Invalid HTTP(S) URL");
        curl_url_cleanup(uri);
        return -1;
    }
    /* HttpClient ignores URL userinfo; only the configured handler credentials
     * participate in authentication. Never carry userinfo through redirects. */
    curl_url_set(uri, CURLUPART_USER, NULL, 0);
    curl_url_set(uri, CURLUPART_PASSWORD, NULL, 0);
    /* HttpClientHandler.UseCookies defaults true. Keep an in-memory jar for
     * this request and its redirects, never across separate radio texts. */
    CURLSH *cookies = curl_share_init();
    if (!cookies || curl_share_setopt(cookies, CURLSHOPT_SHARE, CURL_LOCK_DATA_COOKIE) != CURLSHE_OK) {
        if (cookies) curl_share_cleanup(cookies);
        curl_url_cleanup(uri);
        snprintf(result->error, sizeof(result->error), "Cannot allocate HTTP cookie jar");
        return -1;
    }
    int outcome = -1;
    int64_t deadline = milliseconds() + timeout_ms;
    for (int hop = 0; hop <= MAX_REDIRECTS; hop++) {
        if (cancelled(cancel_context)) { outcome = 1; break; }
        char *scheme = NULL;
        curl_url_get(uri, CURLUPART_SCHEME, &scheme, 0);
        int https = scheme && !strcasecmp(scheme, "https");
        int supported = https || (scheme && !strcasecmp(scheme, "http"));
        curl_free(scheme);
        if (!supported) { snprintf(result->error, sizeof(result->error), "Only HTTP and HTTPS URLs are supported"); break; }
        long remaining = (long)(deadline - milliseconds());
        if (remaining <= 0) { snprintf(result->error, sizeof(result->error), "HTTP request timed out"); break; }
        CURL *easy = curl_easy_init();
        CURLM *multi = curl_multi_init();
        if (!easy || !multi) {
            snprintf(result->error, sizeof(result->error), "Cannot allocate HTTP request");
            if (easy) curl_easy_cleanup(easy);
            if (multi) curl_multi_cleanup(multi);
            break;
        }
        response_state response = { .result = result };
        result->error[0] = 0;
        result->reason[0] = 0;
        CURLcode setup = CURLE_OK;
#define SET(option, value) do { if (setup == CURLE_OK) setup = curl_easy_setopt(easy, option, value); } while (0)
        SET(CURLOPT_CURLU, uri);
        SET(CURLOPT_HTTPGET, 1L);
        SET(CURLOPT_PROTOCOLS_STR, "http,https");
        SET(CURLOPT_FOLLOWLOCATION, 0L);
        SET(CURLOPT_NOSIGNAL, 1L);
        SET(CURLOPT_TIMEOUT_MS, remaining);
        SET(CURLOPT_CONNECTTIMEOUT_MS, connect_ms < remaining ? connect_ms : remaining);
        SET(CURLOPT_SSL_VERIFYPEER, 1L);
        SET(CURLOPT_SSL_VERIFYHOST, 2L);
        SET(CURLOPT_ERRORBUFFER, result->error);
        SET(CURLOPT_WRITEFUNCTION, body);
        SET(CURLOPT_WRITEDATA, &response);
        SET(CURLOPT_HEADERFUNCTION, header);
        SET(CURLOPT_HEADERDATA, &response);
        SET(CURLOPT_SUPPRESS_CONNECT_HEADERS, 1L);
        SET(CURLOPT_SHARE, cookies);
        SET(CURLOPT_COOKIEFILE, "");
        /* HttpClient's default request version is HTTP/1.1. */
        SET(CURLOPT_HTTP_VERSION, CURL_HTTP_VERSION_1_1);
        if (ca_file) SET(CURLOPT_CAINFO, ca_file);
        if (user && hop == 0) {
            SET(CURLOPT_USERNAME, user);
            SET(CURLOPT_PASSWORD, password);
            /* Challenge first, like HttpClientHandler.PreAuthenticate=false. */
            SET(CURLOPT_HTTPAUTH, (long)CURLAUTH_ANY);
        }
#undef SET
        CURLcode transfer = setup;
        CURLMcode mc = CURLM_OK;
        int added = 0;
        int running = 0;
        if (setup == CURLE_OK) {
            mc = curl_multi_add_handle(multi, easy);
            added = mc == CURLM_OK;
            if (added) {
                mc = curl_multi_perform(multi, &running);
                while (mc == CURLM_OK && running && !cancelled(cancel_context)) {
                    mc = curl_multi_poll(multi, NULL, 0, 100, NULL);
                    if (mc == CURLM_OK) mc = curl_multi_perform(multi, &running);
                }
                if (cancelled(cancel_context)) outcome = 1;
                else {
                    transfer = CURLE_RECV_ERROR;
                    int pending = 0;
                    CURLMsg *message;
                    while ((message = curl_multi_info_read(multi, &pending))) {
                        if (message->msg == CURLMSG_DONE) transfer = message->data.result;
                    }
                }
            }
        }
        long status = 0;
        curl_easy_getinfo(easy, CURLINFO_RESPONSE_CODE, &status);
        result->status = (int)status;
        if (added) curl_multi_remove_handle(multi, easy);
        curl_easy_cleanup(easy);
        curl_multi_cleanup(multi);
        if (outcome == 1) break;
        if (response.limit) {
            snprintf(result->error, sizeof(result->error), "HTTP response exceeds %s limit", response.limit == 1 ? "16 MiB body" : "64 KiB header");
            break;
        }
        if (mc != CURLM_OK) { snprintf(result->error, sizeof(result->error), "%s", curl_multi_strerror(mc)); break; }
        if (transfer != CURLE_OK) {
            if (!result->error[0]) snprintf(result->error, sizeof(result->error), "%s", curl_easy_strerror(transfer));
            break;
        }
        outcome = 0;
        if (!(status == 300 || status == 301 || status == 302 || status == 303 || status == 307 || status == 308) || !response.location[0] || hop == MAX_REDIRECTS) break;
        CURLU *next = curl_url_dup(uri);
        if (!next) { outcome = -1; snprintf(result->error, sizeof(result->error), "Cannot allocate redirect URL"); break; }
        if (curl_url_set(next, CURLUPART_URL, response.location, 0) != CURLUE_OK) { curl_url_cleanup(next); break; }
        curl_url_set(next, CURLUPART_USER, NULL, 0);
        curl_url_set(next, CURLUPART_PASSWORD, NULL, 0);
        scheme = NULL;
        curl_url_get(next, CURLUPART_SCHEME, &scheme, 0);
        int next_https = scheme && !strcasecmp(scheme, "https");
        supported = next_https || (scheme && !strcasecmp(scheme, "http"));
        curl_free(scheme);
        /* .NET does not follow HTTPS to HTTP. Reject other protocols explicitly
         * instead of allowing a web-controller URL to read a local file. */
        if (!supported) {
            outcome = -1;
            snprintf(result->error, sizeof(result->error), "Redirect URL must use HTTP or HTTPS");
            curl_url_cleanup(next);
            break;
        }
        if (https && !next_https) { curl_url_cleanup(next); break; }
        curl_url_cleanup(uri);
        uri = next;
        outcome = -1;
    }
    curl_url_cleanup(uri);
    curl_share_cleanup(cookies);
    return outcome;
}
