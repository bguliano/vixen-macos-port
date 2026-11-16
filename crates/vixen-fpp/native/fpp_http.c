/* FPP's synchronous HttpClient transport. System libcurl handles TLS/HTTP framing;
 * explicit redirects preserve .NET's method, replay, downgrade and hop rules.
 * All borrowed pointers/fds remain live until this call returns. */
#include <curl/curl.h>
#include <arpa/inet.h>
#include <pthread.h>
#include <errno.h>
#include <limits.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <strings.h>
#include <time.h>
#include <unistd.h>

#define BODY_LIMIT (16u * 1024u * 1024u)
#define HEADER_LIMIT (64u * 1024u)
#define MAX_REDIRECTS 50

typedef int (*cancel_fn)(const void *);
typedef struct {
    int status;
    char error[CURL_ERROR_SIZE];
    unsigned char *body;
    size_t length;
} fpp_result;
typedef struct {
    fpp_result *result;
    char location[8192];
    size_t header_bytes;
    size_t capacity;
    int limit;
    int file_error;
    int fd;
    curl_off_t file_length;
    curl_off_t file_position;
    cancel_fn cancelled;
    const void *cancel_context;
} state;

/* One jar per public FppClient, retained across its API calls. libcurl's lock
 * callbacks serialize cookie access when callers use the client concurrently. */
typedef struct { CURLSH *share; pthread_mutex_t lock; } cookie_jar;
static void cookie_lock(CURL *handle, curl_lock_data data, curl_lock_access access, void *opaque) {
    (void)handle; (void)data; (void)access;
    pthread_mutex_lock(&((cookie_jar *)opaque)->lock);
}
static void cookie_unlock(CURL *handle, curl_lock_data data, void *opaque) {
    (void)handle; (void)data;
    pthread_mutex_unlock(&((cookie_jar *)opaque)->lock);
}
void *vixen_fpp_cookies_new(void) {
    cookie_jar *jar = calloc(1, sizeof(*jar));
    if (!jar) return NULL;
    if (pthread_mutex_init(&jar->lock, NULL)) { free(jar); return NULL; }
    jar->share = curl_share_init();
    if (!jar->share || curl_share_setopt(jar->share, CURLSHOPT_LOCKFUNC, cookie_lock) != CURLSHE_OK
        || curl_share_setopt(jar->share, CURLSHOPT_UNLOCKFUNC, cookie_unlock) != CURLSHE_OK
        || curl_share_setopt(jar->share, CURLSHOPT_USERDATA, jar) != CURLSHE_OK
        || curl_share_setopt(jar->share, CURLSHOPT_SHARE, CURL_LOCK_DATA_COOKIE) != CURLSHE_OK) {
        if (jar->share) curl_share_cleanup(jar->share);
        pthread_mutex_destroy(&jar->lock); free(jar); return NULL;
    }
    return jar;
}
void vixen_fpp_cookies_free(void *opaque) {
    cookie_jar *jar = opaque;
    curl_share_cleanup(jar->share);
    pthread_mutex_destroy(&jar->lock);
    free(jar);
}
static int loopback_host(CURLU *uri) {
    char *host = NULL;
    if (curl_url_get(uri, CURLUPART_HOST, &host, 0) != CURLUE_OK) return 0;
    int allowed = !strcasecmp(host, "localhost");
    struct in_addr v4;
    if (inet_pton(AF_INET, host, &v4) == 1) allowed = (ntohl(v4.s_addr) >> 24) == 127;
    struct in6_addr v6;
    char *address = host;
    size_t length = strlen(host);
    if (length > 2 && host[0] == '[' && host[length-1] == ']') { host[length-1] = 0; address++; }
    if (inet_pton(AF_INET6, address, &v6) == 1) allowed = IN6_IS_ADDR_LOOPBACK(&v6);
    curl_free(host);
    return allowed;
}
static curl_socket_t loopback_socket(void *context, curlsocktype purpose, struct curl_sockaddr *address) {
    (void)context; (void)purpose;
    int allowed = 0;
    if (address->family == AF_INET) allowed = (ntohl(((struct sockaddr_in *)&address->addr)->sin_addr.s_addr) >> 24) == 127;
    else if (address->family == AF_INET6) allowed = IN6_IS_ADDR_LOOPBACK(&((struct sockaddr_in6 *)&address->addr)->sin6_addr);
    return allowed ? socket(address->family, address->socktype, address->protocol) : CURL_SOCKET_BAD;
}

static int64_t milliseconds(void) {
    struct timespec now;
    clock_gettime(CLOCK_MONOTONIC, &now);
    return (int64_t)now.tv_sec * 1000 + now.tv_nsec / 1000000;
}
static size_t body(char *data, size_t size, size_t count, void *opaque) {
    state *s = opaque;
    if (size && count > SIZE_MAX / size) return 0;
    size_t bytes = size * count;
    if (bytes > BODY_LIMIT - s->result->length) { s->limit = 1; return 0; }
    size_t needed = s->result->length + bytes;
    if (needed > s->capacity) {
        size_t capacity = s->capacity ? s->capacity * 2 : 8192;
        if (capacity < needed) capacity = needed;
        if (capacity > BODY_LIMIT) capacity = BODY_LIMIT;
        void *next = realloc(s->result->body, capacity);
        if (!next) { s->limit = 3; return 0; }
        s->result->body = next;
        s->capacity = capacity;
    }
    if (bytes) memcpy(s->result->body + s->result->length, data, bytes);
    s->result->length = needed;
    return bytes;
}
static size_t header(char *data, size_t size, size_t count, void *opaque) {
    state *s = opaque;
    if (size && count > SIZE_MAX / size) return 0;
    size_t bytes = size * count;
    if (bytes > HEADER_LIMIT - s->header_bytes) { s->limit = 2; return 0; }
    s->header_bytes += bytes;
    if (bytes >= 5 && !memcmp(data, "HTTP/", 5)) {
        s->location[0] = 0;
        s->result->length = 0;
    } else if (bytes >= 9 && !strncasecmp(data, "Location:", 9)) {
        const char *value = data + 9;
        size_t n = bytes - 9;
        while (n && (*value == ' ' || *value == '\t')) { value++; n--; }
        while (n && (value[n-1] == '\r' || value[n-1] == '\n' || value[n-1] == ' ' || value[n-1] == '\t')) n--;
        if (n >= sizeof(s->location) || memchr(value, 0, n)) { s->limit = 2; return 0; }
        memcpy(s->location, value, n);
        s->location[n] = 0;
    }
    return bytes;
}
static size_t upload(char *data, size_t size, size_t count, void *opaque) {
    state *s = opaque;
    if (s->cancelled(s->cancel_context) || (size && count > SIZE_MAX / size)) return CURL_READFUNC_ABORT;
    size_t bytes = size * count;
    curl_off_t remaining = s->file_length - s->file_position;
    if ((uint64_t)remaining < bytes) bytes = (size_t)remaining;
    if (!bytes) return 0;
    ssize_t got;
    do { got = pread(s->fd, data, bytes, (off_t)s->file_position); } while (got < 0 && errno == EINTR);
    if (got <= 0) { s->file_error = 1; return CURL_READFUNC_ABORT; }
    s->file_position += got;
    return (size_t)got;
}
static int seek_upload(void *opaque, curl_off_t offset, int origin) {
    state *s = opaque;
    if (origin != SEEK_SET || offset < 0 || offset > s->file_length) return CURL_SEEKFUNC_FAIL;
    s->file_position = offset;
    return CURL_SEEKFUNC_OK;
}
int vixen_fpp_http_init(void) { return (int)curl_global_init(CURL_GLOBAL_DEFAULT); }
void vixen_fpp_http_free(unsigned char *body) { free(body); }

/* Return 0=response, 1=cancelled, 2=timeout, 3=UI test boundary, -1=transport error. ca_file is private
 * test injection: production always passes NULL and uses the system trust store. */
int vixen_fpp_http_send(const char *url, int post, int fd, int64_t file_length,
                        const char *ca_file, long timeout_ms, void *cookies, int loopback_only,
                        cancel_fn cancelled, const void *cancel_context,
                        fpp_result *result) {
    memset(result, 0, sizeof(*result));
    CURLU *uri = curl_url();
    if (!uri) { snprintf(result->error, sizeof(result->error), "Cannot allocate HTTP URL"); return -1; }
    if (curl_url_set(uri, CURLUPART_URL, url, CURLU_DISALLOW_USER) != CURLUE_OK) {
        snprintf(result->error, sizeof(result->error), "Invalid HTTP(S) URL");
        curl_url_cleanup(uri);
        return -1;
    }
    int outcome = -1;
    int64_t deadline = milliseconds() + timeout_ms;
    for (int hop = 0; hop <= MAX_REDIRECTS; hop++) {
        if (cancelled(cancel_context)) { outcome = 1; break; }
        if (loopback_only && !loopback_host(uri)) {
            snprintf(result->error, sizeof(result->error), "External devices are disabled in UI tests.");
            outcome = 3; break;
        }
        char *scheme = NULL;
        curl_url_get(uri, CURLUPART_SCHEME, &scheme, 0);
        int https = scheme && !strcasecmp(scheme, "https");
        int supported = https || (scheme && !strcasecmp(scheme, "http"));
        curl_free(scheme);
        if (!supported) { snprintf(result->error, sizeof(result->error), "Only HTTP and HTTPS URLs are supported"); break; }
        long remaining = (long)(deadline - milliseconds());
        if (remaining <= 0) { outcome = 2; break; }
        CURL *easy = curl_easy_init();
        CURLM *multi = curl_multi_init();
        struct curl_slist *headers = NULL;
        if (!easy || !multi) {
            snprintf(result->error, sizeof(result->error), "Cannot allocate HTTP request");
            if (easy) curl_easy_cleanup(easy);
            if (multi) curl_multi_cleanup(multi);
            break;
        }
        free(result->body);
        result->body = NULL;
        result->length = 0;
        result->error[0] = 0;
        state response = { .result = result, .fd = fd, .file_length = file_length,
                           .cancelled = cancelled, .cancel_context = cancel_context };
        CURLcode setup = CURLE_OK;
#define SET(option, value) do { if (setup == CURLE_OK) setup = curl_easy_setopt(easy, option, value); } while (0)
#define HEADER(value) do { struct curl_slist *next = curl_slist_append(headers, value); if (next) headers = next; else setup = CURLE_OUT_OF_MEMORY; } while (0)
        HEADER("Connection: close");
        HEADER("Expect:");
        if (post) HEADER(fd >= 0 ? "Content-Type: application/octet-stream" : "Content-Type:");
        SET(CURLOPT_HTTPHEADER, headers);
        SET(CURLOPT_CURLU, uri);
        SET(CURLOPT_SHARE, ((cookie_jar *)cookies)->share);
        SET(CURLOPT_COOKIEFILE, "");
        if (loopback_only) {
            SET(CURLOPT_PROXY, "");
            SET(CURLOPT_OPENSOCKETFUNCTION, loopback_socket);
        }
        SET(CURLOPT_PROTOCOLS_STR, "http,https");
        SET(CURLOPT_FOLLOWLOCATION, 0L);
        SET(CURLOPT_NOSIGNAL, 1L);
        SET(CURLOPT_TIMEOUT_MS, remaining);
        SET(CURLOPT_CONNECTTIMEOUT_MS, remaining < 10000 ? remaining : 10000L);
        SET(CURLOPT_SSL_VERIFYPEER, 1L);
        SET(CURLOPT_SSL_VERIFYHOST, 2L);
        SET(CURLOPT_ERRORBUFFER, result->error);
        SET(CURLOPT_WRITEFUNCTION, body);
        SET(CURLOPT_WRITEDATA, &response);
        SET(CURLOPT_HEADERFUNCTION, header);
        SET(CURLOPT_HEADERDATA, &response);
        SET(CURLOPT_SUPPRESS_CONNECT_HEADERS, 1L);
        SET(CURLOPT_HTTP_VERSION, CURL_HTTP_VERSION_1_1);
        if (ca_file) SET(CURLOPT_CAINFO, ca_file);
        if (post) {
            SET(CURLOPT_POST, 1L);
            SET(CURLOPT_POSTFIELDSIZE_LARGE, (curl_off_t)(fd >= 0 ? file_length : 0));
            SET(CURLOPT_READFUNCTION, upload);
            SET(CURLOPT_READDATA, &response);
            SET(CURLOPT_SEEKFUNCTION, seek_upload);
            SET(CURLOPT_SEEKDATA, &response);
        } else SET(CURLOPT_HTTPGET, 1L);
#undef SET
#undef HEADER
        CURLcode transfer = setup;
        CURLMcode mc = CURLM_OK;
        int added = 0, running = 0;
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
        curl_slist_free_all(headers);
        if (outcome == 1) break;
        if (transfer == CURLE_OPERATION_TIMEDOUT) { outcome = 2; break; }
        if (response.limit) {
            snprintf(result->error, sizeof(result->error), "%s", response.limit == 1 ? "The device's answer is too large." : response.limit == 2 ? "The device's headers are too large." : "Cannot allocate HTTP response");
            break;
        }
        if (response.file_error) { snprintf(result->error, sizeof(result->error), "The file ended or could not be read before all of it was sent."); break; }
        if (mc != CURLM_OK) { snprintf(result->error, sizeof(result->error), "%s", curl_multi_strerror(mc)); break; }
        if (transfer != CURLE_OK) {
            if (!result->error[0]) snprintf(result->error, sizeof(result->error), "%s", curl_easy_strerror(transfer));
            break;
        }
        outcome = 0;
        if (!(status == 300 || status == 301 || status == 302 || status == 303 || status == 307 || status == 308) || !response.location[0] || hop == MAX_REDIRECTS) break;
        CURLU *next = curl_url_dup(uri);
        if (!next) { outcome = -1; snprintf(result->error, sizeof(result->error), "Cannot allocate redirect URL"); break; }
        if (curl_url_set(next, CURLUPART_URL, response.location, CURLU_DISALLOW_USER) != CURLUE_OK) { curl_url_cleanup(next); break; }
        scheme = NULL;
        curl_url_get(next, CURLUPART_SCHEME, &scheme, 0);
        int next_https = scheme && !strcasecmp(scheme, "https");
        supported = next_https || (scheme && !strcasecmp(scheme, "http"));
        curl_free(scheme);
        if (!supported || (https && !next_https)) { curl_url_cleanup(next); break; }
        /* HttpClient changes POST to GET on 300/301/302, and non-HEAD to GET on
         * 303. 307/308 preserve the method and replay the seekable file. */
        if (status == 300 || status == 301 || status == 302 || status == 303) post = 0;
        curl_url_cleanup(uri);
        uri = next;
        outcome = -1;
    }
    curl_url_cleanup(uri);
    return outcome;
}
