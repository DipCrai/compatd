/**
 * compatd — LD_PRELOAD shim for systemd-compat
 *
 * Intercepts:
 *   sd_notify / sd_notifyf        → no-op (returns 0)
 *   sd_booted                     → 0 (not systemd)
 *   sd_is_socket / sd_is_fifo     → 0 (not from systemd)
 *   sd_listen_fds                 → 0 (no fds from systemd)
 *   sd_watchdog_enabled           → 0 (disabled)
 *   sd_journal_{print,send,...}   → syslog(3)
 *   sd_journal_{open,next,...}    → -ENOENT (empty journal)
 *
 * Compile:
 *   gcc -shared -fPIC -o libcompatd_preload.so sd_notify.c -ldl
 *
 * Use:
 *   LD_PRELOAD=/usr/lib/compatd/libcompatd_preload.so dockerd
 */

#define _GNU_SOURCE
#include <dlfcn.h>
#include <errno.h>
#include <stdarg.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <poll.h>
#include <syslog.h>
#include <sys/socket.h>
#include <sys/stat.h>
#include <unistd.h>

static int compatd_verbose = -1; /* -1 = uninitialized */

static int is_verbose(void) {
    if (compatd_verbose < 0) {
        compatd_verbose = getenv("COMPATD_VERBOSE") ? 1 : 0;
    }
    return compatd_verbose;
}

/* ========== sd_notify ========== */

int sd_notify(int unset_environment, const char *state) {
    if (is_verbose() && state) {
        syslog(LOG_INFO, "[compatd] sd_notify: %s", state);
    }
    (void)unset_environment;
    return 0;
}

int sd_notifyf(int unset_environment, const char *format, ...) {
    if (is_verbose() && format) {
        va_list ap;
        va_start(ap, format);
        vsyslog(LOG_INFO, format, ap);
        va_end(ap);
    }
    (void)unset_environment;
    return 0;
}

int sd_pid_notify(int unset_env, pid_t pid, const char *state) {
    (void)pid;
    return sd_notify(unset_env, state);
}

int sd_pid_notifyf(int unset_env, pid_t pid, const char *fmt, ...) {
    (void)pid;
    if (is_verbose() && fmt) {
        va_list ap;
        va_start(ap, fmt);
        vsyslog(LOG_INFO, fmt, ap);
        va_end(ap);
    }
    (void)unset_env;
    return 0;
}

int sd_notify_barrier(int unset_environment, uint64_t timeout) {
    (void)unset_environment; (void)timeout;
    return 0;
}

int sd_pid_notify_with_fds(int unset_env, pid_t pid, const char *state, const int *fds, unsigned n_fds) {
    (void)pid; (void)fds; (void)n_fds;
    return sd_notify(unset_env, state);
}

int sd_pid_notifyf_with_fds(int unset_env, pid_t pid, const char *fmt, ...) {
    (void)pid; (void)fmt; (void)unset_env;
    return 0;
}

/* ========== sd_booted / sd_is_* / sd_listen_fds / sd_watchdog ========== */

int sd_booted(void) {
    return 0;
}

int sd_is_socket(int fd, int family, int type, int listening) {
    (void)fd; (void)family; (void)type; (void)listening;
    return 0;
}

int sd_is_fifo(int fd, const char *path) {
    (void)fd; (void)path;
    return 0;
}

int sd_is_socket_inet(int fd, int family, int type, int listening, uint16_t port) {
    (void)fd; (void)family; (void)type; (void)listening; (void)port;
    return 0;
}

int sd_is_socket_unix(int fd, int type, int listening, const char *path, size_t length) {
    (void)fd; (void)type; (void)listening; (void)path; (void)length;
    return 0;
}

int sd_is_socket_sockaddr(int fd, int type, const struct sockaddr *addr, unsigned len) {
    (void)fd; (void)type; (void)addr; (void)len;
    return 0;
}

int sd_is_mq(int fd, const char *path) {
    (void)fd; (void)path;
    return 0;
}

int sd_is_special(int fd) {
    (void)fd;
    return 0;
}

int sd_listen_fds(int unset_environment) {
    (void)unset_environment;
    return 0;
}

int sd_listen_fds_with_names(int unset_environment, char ***names) {
    if (names) *names = NULL;
    (void)unset_environment;
    return 0;
}

int sd_watchdog_enabled(int unset_environment, uint64_t *usec) {
    if (usec) *usec = 0;
    (void)unset_environment;
    return 0;
}

/* ========== sd_journal — write → syslog ========== */

/* sd_journal_print(priority, fmt, ...) → syslog */
int sd_journal_print(int priority, const char *format, ...) {
    va_list ap;
    va_start(ap, format);
    vsyslog(priority, format, ap);
    va_end(ap);
    return 0;
}

int sd_journal_printv(int priority, const char *format, va_list ap) {
    vsyslog(priority, format, ap);
    return 0;
}

int sd_journal_send(const char *format, ...) {
    if (!format) return 0;

    va_list ap;
    va_start(ap, format);
    vsyslog(LOG_INFO, format, ap);
    va_end(ap);
    return 0;
}

int sd_journal_sendv(const struct iovec *iov, int n) {
    if (!iov || n <= 0) return 0;

    size_t total = 0;
    for (int i = 0; i < n && i < 64; i++) {
        total += iov[i].iov_len;
    }
    char buf[4096];
    size_t pos = 0;
    for (int i = 0; i < n && pos < sizeof(buf) - 1; i++) {
        size_t copy = iov[i].iov_len;
        if (pos + copy >= sizeof(buf) - 1)
            copy = sizeof(buf) - 1 - pos;
        memcpy(buf + pos, iov[i].iov_base, copy);
        pos += copy;
        if (pos < sizeof(buf) - 1)
            buf[pos++] = ' ';
    }
    buf[pos] = '\0';
    syslog(LOG_INFO, "%s", buf);
    return 0;
}

int sd_journal_perror(const char *message) {
    if (message)
        syslog(LOG_ERR, "%s: %m", message);
    else
        syslog(LOG_ERR, "%m");
    return 0;
}

int sd_journal_stream_fd(const char *identifier, int priority, int level_prefix) {
    (void)identifier; (void)priority; (void)level_prefix;
    errno = ENOENT;
    return -1;
}

int sd_journal_stream_fd_with_namespace(const char *namespace, const char *identifier, int priority, int level_prefix) {
    (void)namespace;
    return sd_journal_stream_fd(identifier, priority, level_prefix);
}

/* ========== sd_journal — read ========== */
/* All read functions return -ENOENT — journal is empty */

typedef struct {} sd_journal;

int sd_journal_open(sd_journal **ret, int flags) {
    (void)flags;
    if (ret) *ret = NULL;
    return -ENOENT;
}

int sd_journal_open_directory(sd_journal **ret, const char *path, int flags) {
    (void)path; (void)flags;
    if (ret) *ret = NULL;
    return -ENOENT;
}

int sd_journal_open_files(sd_journal **ret, const char **paths, int flags) {
    (void)paths; (void)flags;
    if (ret) *ret = NULL;
    return -ENOENT;
}

int sd_journal_open_container(sd_journal **ret, const char *machine, int flags) {
    (void)machine; (void)flags;
    if (ret) *ret = NULL;
    return -ENOENT;
}

int sd_journal_open_namespace(sd_journal **ret, const char *namespace, int flags) {
    (void)namespace; (void)flags;
    if (ret) *ret = NULL;
    return -ENOENT;
}

int sd_journal_open_directory_fd(sd_journal **ret, int fd, int flags) {
    (void)fd; (void)flags;
    if (ret) *ret = NULL;
    return -ENOENT;
}

int sd_journal_open_files_fd(sd_journal **ret, int fd, int flags) {
    (void)fd; (void)flags;
    if (ret) *ret = NULL;
    return -ENOENT;
}

void sd_journal_close(sd_journal *j) {
    (void)j;
}

int sd_journal_next(sd_journal *j) {
    (void)j;
    return 0;
}

int sd_journal_previous(sd_journal *j) {
    (void)j;
    return 0;
}

int sd_journal_next_skip(sd_journal *j, uint64_t skip) {
    (void)j; (void)skip;
    return 0;
}

int sd_journal_previous_skip(sd_journal *j, uint64_t skip) {
    (void)j; (void)skip;
    return 0;
}

int sd_journal_get_data(sd_journal *j, const char *field, const void **data, size_t *length) {
    (void)j; (void)field; (void)data; (void)length;
    return -ENOENT;
}

int sd_journal_enumerate_data(sd_journal *j, const void **data, size_t *length) {
    (void)j; (void)data; (void)length;
    return 0;
}

int sd_journal_restart_data(sd_journal *j) {
    (void)j;
    return 0;
}

int sd_journal_get_cursor(sd_journal *j, char **cursor) {
    (void)j;
    if (cursor) *cursor = NULL;
    return -ENOENT;
}

int sd_journal_test_cursor(sd_journal *j, const char *cursor) {
    (void)j; (void)cursor;
    return 0;
}

int sd_journal_get_realtime_usec(sd_journal *j, uint64_t *usec) {
    (void)j;
    if (usec) *usec = 0;
    return -ENOENT;
}

int sd_journal_get_monotonic_usec(sd_journal *j, uint64_t *usec) {
    (void)j;
    if (usec) *usec = 0;
    return -ENOENT;
}

int sd_journal_seek_head(sd_journal *j) {
    (void)j;
    return 0;
}

int sd_journal_seek_tail(sd_journal *j) {
    (void)j;
    return 0;
}

int sd_journal_seek_monotonic_usec(sd_journal *j, uint64_t usec) {
    (void)j; (void)usec;
    return 0;
}

int sd_journal_seek_realtime_usec(sd_journal *j, uint64_t usec) {
    (void)j; (void)usec;
    return 0;
}

int sd_journal_seek_cursor(sd_journal *j, const char *cursor) {
    (void)j; (void)cursor;
    return 0;
}

int sd_journal_get_usage(sd_journal *j, uint64_t *bytes) {
    (void)j;
    if (bytes) *bytes = 0;
    return 0;
}

int sd_journal_get_cutoff_realtime_usec(sd_journal *j, uint64_t *from, uint64_t *to) {
    (void)j;
    if (from) *from = 0;
    if (to) *to = 0;
    return -ENOENT;
}

int sd_journal_get_cutoff_monotonic_usec(sd_journal *j, uint64_t *from, uint64_t *to) {
    (void)j;
    if (from) *from = 0;
    if (to) *to = 0;
    return -ENOENT;
}

int sd_journal_wait(sd_journal *j, uint64_t timeout_usec) {
    (void)j; (void)timeout_usec;
    return 0;
}

int sd_journal_get_events(sd_journal *j) {
    (void)j;
    return POLLIN;
}

int sd_journal_get_fd(sd_journal *j) {
    (void)j;
    errno = ENOENT;
    return -1;
}

int sd_journal_reliable_fd(sd_journal *j) {
    (void)j;
    return 0;
}

int sd_journal_process(sd_journal *j) {
    (void)j;
    return 0;
}

int sd_journal_get_timeout(sd_journal *j, uint64_t *timeout_usec) {
    (void)j;
    if (timeout_usec) *timeout_usec = (uint64_t)-1;
    return 0;
}

int sd_journal_add_match(sd_journal *j, const void *data, size_t size) {
    (void)j; (void)data; (void)size;
    return 0;
}

int sd_journal_flush_matches(sd_journal *j) {
    (void)j;
    return 0;
}

int sd_journal_add_disjunction(sd_journal *j) {
    (void)j;
    return 0;
}

int sd_journal_add_conjunction(sd_journal *j) {
    (void)j;
    return 0;
}

int sd_journal_query_unique(sd_journal *j, const char *field) {
    (void)j; (void)field;
    return 0;
}

int sd_journal_enumerate_unique(sd_journal *j, const void **data, size_t *length) {
    (void)j; (void)data; (void)length;
    return 0;
}

int sd_journal_restart_unique(sd_journal *j) {
    (void)j;
    return 0;
}

int sd_journal_enumerate_fields(sd_journal *j, const char **field) {
    (void)j;
    if (field) *field = NULL;
    return 0;
}

int sd_journal_restart_fields(sd_journal *j) {
    (void)j;
    return 0;
}

int sd_journal_has_runtime_files(sd_journal *j) {
    (void)j;
    return 0;
}

int sd_journal_has_persistent_files(sd_journal *j) {
    (void)j;
    return 0;
}

int sd_journal_get_data_threshold(sd_journal *j, size_t *sz) {
    (void)j;
    if (sz) *sz = 0;
    return 0;
}

int sd_journal_set_data_threshold(sd_journal *j, size_t sz) {
    (void)j; (void)sz;
    return 0;
}

int sd_journal_get_catalog(sd_journal *j, char **catalog) {
    (void)j;
    if (catalog) *catalog = NULL;
    return -ENOENT;
}

int sd_journal_get_catalog_for_message_id(const void *id, char **catalog) {
    (void)id;
    if (catalog) *catalog = NULL;
    return -ENOENT;
}

int sd_journal_step_one(sd_journal *j, int advanced) {
    (void)j; (void)advanced;
    return 0;
}

int sd_journal_enumerate_available_data(sd_journal *j, const void **data, size_t *length) {
    (void)j; (void)data; (void)length;
    return 0;
}

int sd_journal_enumerate_available_unique(sd_journal *j, const void **data, size_t *length) {
    (void)j; (void)data; (void)length;
    return 0;
}

int sd_journal_get_seqnum(sd_journal *j, uint64_t *seqnum) {
    (void)j;
    if (seqnum) *seqnum = 0;
    return -ENOENT;
}

/* ========== sd_id128 ========== */

int sd_id128_get_machine(const void *ret) {
    FILE *f = fopen("/etc/machine-id", "r");
    if (!f) {
        memset((void*)ret, 0, 16);
        return 0;
    }
    char buf[33] = {0};
    if (fread(buf, 1, 32, f) < 32) {
        memset((void*)ret, 0, 16);
        fclose(f);
        return 0;
    }
    fclose(f);

    unsigned char *r = (unsigned char*)ret;
    for (int i = 0; i < 16; i++) {
        unsigned int byte;
        sscanf(buf + 2*i, "%2x", &byte);
        r[i] = (unsigned char)byte;
    }
    return 0;
}
